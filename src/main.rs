#[cfg(windows)]
use std::ptr::write_volatile;
use std::{fs, io::Read, path::PathBuf};

use libafl::{
    bolts::{shmem::StdShmemProvider, shmem::UnixShMemProvider, current_nanos, rands::StdRand, tuples::tuple_list},
    corpus::{OnDiskCorpus},
    events::SimpleEventManager,
    executors::{forkserver::ForkserverExecutor},
    feedbacks::{CrashFeedback, MaxMapFeedback},
    fuzzer::{Evaluator, Fuzzer, StdFuzzer},
    inputs::GeneralizedInput,
    monitors::MultiMonitor,
    mutators::{
        scheduled::StdScheduledMutator, GrimoireExtensionMutator,
        GrimoireRandomDeleteMutator,
        GrimoireStringReplacementMutator,
    },
    observers::{HitcountsMapObserver, StdMapObserver, TimeObserver},
    schedulers::QueueScheduler,
    stages::mutational::StdMutationalStage,
    state::StdState,
};

/// Coverage map with explicit assignments due to the lack of instrumentation
static mut SIGNALS: [u8; 16] = [0; 16];

/// Assign a signal to the signals map
fn signals_set(idx: usize) {
    unsafe { SIGNALS[idx] = 1 };
}

/*fn is_sub<T: PartialEq>(mut haystack: &[T], needle: &[T]) -> bool {
    if needle.is_empty() {
        return true;
    }
    while !haystack.is_empty() {
        if haystack.starts_with(needle) {
            return true;
        }
        haystack = &haystack[1..];
    }
    false
}*/

#[allow(clippy::similar_names)]
pub fn main() {
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
            let input = GeneralizedInput::new(buffer);
            initial_inputs.push(input);
        }
    }


     /*  // function to fuzz
    let mut harness = |input: &GeneralizedInput| {
        let target_bytes = input.target_bytes();
        let bytes = target_bytes.as_slice();

        if is_sub(bytes, "fn".as_bytes()) {
            signals_set(2);
        }
        ExitKind::Ok
    };*/

    // Create an observation channel using the signals map
    let observer = StdMapObserver::new("signals", unsafe { &mut SIGNALS });
    let time_observer = TimeObserver::new("time");

    // Feedback to rate the interestingness of an input
    let mut feedback = MaxMapFeedback::new_tracking(&observer, false, true);

    const MAP_SIZE: usize = 65536;
    let mut shmem_provider = UnixShMemProvider::new().unwrap();
    // The coverage map shared between observer and executor
    let mut shmem = shmem_provider.new_shmem(MAP_SIZE).unwrap();
    //let the forkserver know the shmid
    shmem.write_to_env("__AFL_SHM_ID").unwrap();
    let shmem_buf = shmem.as_mut_slice();
    // write shared memory id to environment so Executor knows about it
            // Create an observation channel using the signals map
    let edges_observer = HitcountsMapObserver::new(StdMapObserver::new("shared_mem",
                 shmem_buf));
    let mut feedback = MaxMapFeedback::new_tracking(&edges_observer, true, true);

    // A feedback to choose if an input is a solution or not
    let mut objective = CrashFeedback::new();

    // create a State from scratch
    let mut state = StdState::new(
        // RNG
        StdRand::with_seed(current_nanos()),
        // Corpus that will be evolved, we keep it in memory for performance
        OnDiskCorpus::new(PathBuf::from("./corpus")).unwrap(),
        // Corpus in which we store solutions (crashes in this example),
        // on disk so the user can get them after stopping the fuzzer
        OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(),
        // States of the feedbacks.
        // The feedbacks can report the data that should persist in the State.
        &mut feedback,
        // Same for objective feedbacks
        &mut objective,
    )
    .unwrap();

    /*if state.metadata().get::<Tokens>().is_none() {
        state.add_metadata(Tokens::from([b"FOO".to_vec(), b"BAR".to_vec()]));
    }*/

    // The Monitor trait define how the fuzzer stats are reported to the user
    let monitor = MultiMonitor::new(|s| println!("{}", s));
    // The event manager handle the various events generated during the fuzzing loop
    // such as the notification of the addition of a new item to the corpus
    let mut mgr = SimpleEventManager::new(monitor);
    //let mut mgr = SimpleEventManager::new(stats);
    // A queue policy to get testcasess from the corpus
    let scheduler = QueueScheduler::new();

    // A fuzzer with feedbacks and a corpus scheduler
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    //let generalization = GeneralizationStage::new(&observer);

    // Create the executor for an in-process function with just one observer
    /*let mut executor = InProcessExecutor::new(
        // not in scope anymore 
        &mut harness,
        tuple_list!(observer),
        &mut fuzzer,
        &mut state,
        &mut mgr,
    )
    .expect("Failed to create the Executor");
    */
    let program = 
    let mut executor = ForkserverExecutor::builder()
                    .is_persistent(true)
                    .build_dynamic_map(edges_observer, tuple_list!(time_observer))
                    
        .program("LD_LIBRARY_PATH=/home/t/Fuzzing/HDexecutor ./home/t/Fuzzing/HDexecutor/target/release/hdexecutor".to_string())
                    .args(
    &[String::from("@@"), "/home/t/Fuzzing/HDexecutor/btrfs.img".to_string(), "btrfs".to_string()])
                    .shmem_provider(&mut shmem_provider)
                    .coverage_map_size(MAP_SIZE).unwrap();

    let grimoire_mutator = StdScheduledMutator::with_max_stack_pow(
        tuple_list!(
            GrimoireExtensionMutator::new(),
            GrimoireStringReplacementMutator::new(),
            GrimoireRandomDeleteMutator::new(),
        ),
        3,
    );
    
    let mut stages = tuple_list!(
        StdMutationalStage::new(grimoire_mutator)
    );


    for input in initial_inputs {
        fuzzer
            .evaluate_input(&mut state, &mut executor, &mut mgr, input)
            .unwrap();
    }

    fuzzer
        .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
        .expect("Error in the fuzzing loop");
}
