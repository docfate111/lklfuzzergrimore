//! A libfuzzer-like fuzzer with llmp-multithreading support and restarts
//! The example harness is built for libpng.
//! This will fuzz javascript.

use clap::Parser;
use core::time::Duration;
use std::{env, fs, io::Read, net::SocketAddr, path::PathBuf};

use libafl::{
    bolts::{
        core_affinity::Cores,
        current_nanos,
        launcher::Launcher,
        rands::StdRand,
        shmem::{ShMemProvider, StdShMemProvider},
        tuples::tuple_list,
        AsMutSlice,
    },
    corpus::{CachedOnDiskCorpus, Corpus, OnDiskCorpus},
    events::EventConfig,
    executors::{
	forkserver::{ForkserverExecutor, TimeoutForkserverExecutor},
        HasObservers,
    },
    feedback_or, feedback_or_fast,
    feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback, TimeoutFeedback},
    fuzzer::{Fuzzer, StdFuzzer},
    inputs::BytesInput,
    monitors::MultiMonitor,
    mutators::{
        GrimoireExtensionMutator, GrimoireRandomDeleteMutator, GrimoireRecursiveReplacementMutator,
        GrimoireStringReplacementMutator, StdScheduledMutator,
    },
    prelude::{MatchName, ShMem},
    observers::{HitcountsMapObserver, MapObserver, StdMapObserver, TimeObserver},
    schedulers::{IndexesLenTimeMinimizerScheduler, QueueScheduler},
    stages::mutational::StdMutationalStage,
    state::{HasCorpus, StdState},
    Error, Evaluator,
};

use nix::sys::signal::Signal;
/// Parses a millseconds int into a [`Duration`], used for commandline arg parsing
fn timeout_from_millis_str(time: &str) -> Result<Duration, Error> {
    Ok(Duration::from_millis(time.parse()?))
}

#[derive(Debug, Parser)]
#[command(
    name = "libafl_lkl",
    about = "Fuzz lkl with libafl",
    author = "th3lsh3ll + Andrea Fioraldi <andreafioraldi@gmail.com>"
)]
struct Opt {
    #[arg(
        short,
        long,
        value_parser = Cores::from_cmdline,
        help = "Spawn a client in each of the provided cores. Broker runs in the 0th core. 'all' to select all available cores. 'none' to run a client without binding to any core. eg: '1,2-4,6' selects the cores 1,2,3,4,6.",
        name = "CORES"
    )]
    cores: Cores,
    #[arg(short = 'a', long, help = "Specify a remote broker", name = "REMOTE")]
    remote_broker_addr: Option<SocketAddr>,

    #[arg(
        short,
        long,
        help = "Set the output directory, default is ./out",
        name = "OUTPUT",
        default_value = "./out"
    )]
    output: PathBuf,

    #[arg(
        short,
        long,
        help = "Convert a stored testcase to JavaScript text",
        name = "REPRO"
    )]
    repro: Option<PathBuf>,

    #[arg(
        value_parser = timeout_from_millis_str,
        short,
        long,
        help = "Set the execution timeout in milliseconds, default is 1000",
        name = "TIMEOUT",
        default_value = "1000"
    )]
    timeout: Duration,
     #[arg(
        help = "Signal used to stop child",
        short = 's',
        long = "signal",
        value_parser = str::parse::<Signal>,
        default_value = "SIGKILL"
    )]
    signal: Signal,
}

//const NUM_GENERATED: usize = 28;
const CORPUS_CACHE: usize = 4096;

/// The main fn, `no_mangle` as it is a C symbol
//#[no_mangle]
//#[allow(clippy::too_many_lines)]
pub fn main() {
    const MAP_SIZE: usize = 65536;
    // Registry the metadata types used in this fuzzer
    // Needed only on no_std
    //RegistryBuilder::register::<Tokens>();
    let opt = Opt::parse();

    let mut initial_inputs = vec![];
    for entry in fs::read_dir("./corpus").unwrap() {
        let path = entry.unwrap().path();
        let attr = fs::metadata(&path);
        if attr.is_err() {
            continue;
        }
        let attr = attr.unwrap();

        if attr.is_file() && attr.len() > 0 {
            println!("Loading file {:?} ...", &path);
            let mut file = fs::File::open(path).expect("no file found");
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).expect("buffer overflow");
            let input = BytesInput::new(buffer);
            initial_inputs.push(input);
        }
    }

    println!(
        "Workdir: {:?}",
        env::current_dir().unwrap().to_string_lossy().to_string()
    );

    let mut shmem_provider = StdShMemProvider::new().expect("Failed to init shared memory");
    // The coverage map shared between observer and executor
    let mut shmem = shmem_provider.new_shmem(MAP_SIZE).unwrap();
    // let the forkserver know the shmid
    shmem.write_to_env("__AFL_SHM_ID").unwrap();
    let shmem_buf = shmem.as_mut_slice();
    let stats = MultiMonitor::new(|s| println!("{s}"));

    let mut run_client = |state: Option<StdState<_, _, _, _>>, mut restarting_mgr, _core_id| {
        let mut objective_dir = opt.output.clone();
        objective_dir.push("crashes");
        let mut corpus_dir = opt.output.clone();
        corpus_dir.push("corpus");

        let edges_observer =
		unsafe { HitcountsMapObserver::new(StdMapObserver::new("shared_mem", shmem_buf)) };
        // Create an observation channel to keep track of the execution time
        let time_observer = TimeObserver::new("time");

        // Feedback to rate the interestingness of an input
        // This one is composed by two Feedbacks in OR
        let mut feedback = feedback_or!(
            // New maximization map feedback linked to the edges observer and the feedback state
            MaxMapFeedback::new_tracking(&edges_observer, true, false),
            // Time feedback, this one does not need a feedback state
            TimeFeedback::new_with_observer(&time_observer)
        );

        // A feedback to choose if an input is a solution or not
        let mut objective = feedback_or_fast!(CrashFeedback::new(), TimeoutFeedback::new());

        // If not restarting, create a State from scratch
        let mut state = state.unwrap_or_else(|| {
            StdState::new(
                // RNG
                StdRand::with_seed(current_nanos()),
                // Corpus that will be evolved, we keep it in memory for performance
                CachedOnDiskCorpus::new(corpus_dir, CORPUS_CACHE).unwrap(),
                // Corpus in which we store solutions (crashes in this example),
                // on disk so the user can get them after stopping the fuzzer
                OnDiskCorpus::new(objective_dir).unwrap(),
                &mut feedback,
                &mut objective,
            )
            .unwrap()
        });

        // A minimization+queue policy to get testcasess from the corpus
        let scheduler = IndexesLenTimeMinimizerScheduler::new(QueueScheduler::new());

        // A fuzzer with feedbacks and a corpus scheduler
        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);
	// let mut tokens = Tokens::new();
        let mut forkserver = ForkserverExecutor::builder()
        .program("./main")
        .debug_child(true)
        //.shmem_provider(&mut shmem_provider)
        //.autotokens(&mut tokens)
        .parse_afl_cmdline(vec!["@@".to_string(), "ext4-00.img".to_string(), "ext4".to_string()])
        .coverage_map_size(MAP_SIZE)
        .build(tuple_list!(time_observer, edges_observer))
        .unwrap();

        if let Some(dynamic_map_size) = forkserver.coverage_map_size() {
             forkserver
            .observers_mut()
            .match_name_mut::<HitcountsMapObserver<StdMapObserver<'_, u8, false>>>("shared_mem")
            .unwrap()
            .downsize_map(dynamic_map_size);
         }
	let mut executor = TimeoutForkserverExecutor::with_signal(
        forkserver,
        opt.timeout,
        opt.signal,
           )
          .expect("Failed to create the executor.");

        // In case the corpus is empty (on first run), reset
        if state.corpus().count() < 1 {
            for input in &initial_inputs {
                fuzzer
                    .add_input(
                        &mut state,
                        &mut executor,
                        &mut restarting_mgr,
                        input.clone().into(),
                    )
                    .unwrap();
            }
        }

        // Setup a basic mutator with a mutational stage
        let mutator = StdScheduledMutator::with_max_stack_pow(
            tuple_list!(
                GrimoireExtensionMutator::new(),
                GrimoireRecursiveReplacementMutator::new(),
                GrimoireStringReplacementMutator::new(),
                // give more probability to avoid
                // large inputs
                GrimoireRandomDeleteMutator::new(),
                GrimoireRandomDeleteMutator::new(),
            ),
            3,
        );
        let mut stages = tuple_list!(StdMutationalStage::new(mutator));

        fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut restarting_mgr)?;
        Ok(())
    };

    match Launcher::builder()
        .shmem_provider(shmem_provider)
        .configuration(EventConfig::from_build_id())
        .monitor(stats)
        .run_client(&mut run_client)
        .cores(&opt.cores)
        .broker_port(1337)
        .remote_broker_addr(opt.remote_broker_addr)
        .stdout_file(Some("fuzzer_stdout"))
        .build()
        .launch()
    {
        Ok(()) => (),
        Err(Error::ShuttingDown) => println!("Fuzzing stopped by user. Good bye."),
        Err(err) => panic!("Failed to run launcher: {err:?}"),
    }
}
