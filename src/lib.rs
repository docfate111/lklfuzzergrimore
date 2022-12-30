#[cfg(linux)]
use std::ptr::write_volatile;
use std::{fs, io::Read, path::PathBuf};

use libafl::{
    bolts::{current_nanos, rands::StdRand, tuples::tuple_list, AsSlice},
    corpus::{InMemoryCorpus, OnDiskCorpus},
    events::SimpleEventManager,
    executors::{inprocess::InProcessExecutor, ExitKind},
    feedback_or,
    feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback},
    fuzzer::{Evaluator, Fuzzer, StdFuzzer},
    inputs::{GeneralizedInput, HasTargetBytes},
    monitors::SimpleMonitor,
    mutators::{
        //havoc_mutations, 
        scheduled::StdScheduledMutator, GrimoireExtensionMutator,
        GrimoireRandomDeleteMutator, GrimoireRecursiveReplacementMutator,
        GrimoireStringReplacementMutator, Tokens,
    },
    observers::{HitcountsMapObserver, StdMapObserver, TimeObserver},
    schedulers::QueueScheduler,
    stages::{mutational::StdMutationalStage, GeneralizationStage},
    state::{HasMetadata, StdState},
};


use libafl_targets::{libfuzzer_initialize, libfuzzer_test_one_input, EDGES_MAP, MAX_EDGES_NUM};

//use hdrepresentation::Program;
//use hdexecutor::exec;


#[allow(clippy::similar_names)]
pub fn libafl_main() {
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

    // The closure that we want to fuzz
    let mut harness = |input: &GeneralizedInput| {
        let target_bytes = input.target_bytes();
        let bytes = target_bytes.as_slice();

            if input.grimoire_mutated {
                // println!(">>> {:?}", input.generalized());
                let p = unsafe { 
                    Program::from_str(std::str::from_utf8_unchecked(bytes).to_string())
                };
                if p.is_err() {
                    // fuzzer should ignore paths that break the parser
                    // hopefully returning Ok avoids this path being explored
                    return ExitKind::Ok;
                }
                let prog = p.unwrap();
                // this can cause a segmentation fault in the C library
                return match exec(&prog, "btrfs.img".to_string(), "btrfs".to_string()) {
                    Err(_) => { ExitKind::Ok },
                    Ok(_) => { ExitKind::Ok },
                };
            }
        ExitKind::Ok
    };

    // Create an observation channel using the coverage map
    let edges = unsafe { &mut EDGES_MAP[0..MAX_EDGES_NUM] };
    let edges_observer = HitcountsMapObserver::new(StdMapObserver::new("edges",
                     edges));
   let time_observer = TimeObserver::new("time"); 
    // Create an observation channel to keep track of the 
    // Feedback to rate the interestingness of an input
    let mut feedback = feedback_or!(
        MaxMapFeedback::new_tracking(&edges_observer, true, false),
        TimeFeedback::new_with_observer(&time_observer)
    );

    // A feedback to choose if an input is a solution or not
    let mut objective = CrashFeedback::new();

    // create a State from scratch
    let mut state = StdState::new(
        // RNG
        StdRand::with_seed(current_nanos()),
        // Corpus that will be evolved, we keep it in memory for performance
        InMemoryCorpus::new(),
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

    if state.metadata().get::<Tokens>().is_none() {
        state.add_metadata(Tokens::from([b"FOO".to_vec(), b"BAR".to_vec()]));
    }

    // The Monitor trait define how the fuzzer stats are reported to the user
    let monitor = SimpleMonitor::new(|s| println!("{}", s));

    // The event manager handle the various events generated during the fuzzing loop
    // such as the notification of the addition of a new item to the corpus
    let mut mgr = SimpleEventManager::new(monitor);

    // A queue policy to get testcasess from the corpus
    let scheduler = QueueScheduler::new();

    // A fuzzer with feedbacks and a corpus scheduler
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    let generalization = GeneralizationStage::new(&edges_observer);

    // Create the executor for an in-process function with just one observer
    let mut executor = InProcessExecutor::new(
        &mut harness,
        tuple_list!(edges_observer, time_observer),
        &mut fuzzer,
        &mut state,
        &mut mgr,
    )
    .expect("Failed to create the Executor");

    // Setup a mutational stage with a basic bytes mutator
    //let mutator = StdScheduledMutator::with_max_stack_pow(havoc_mutations(), 2);
    let grimoire_mutator = StdScheduledMutator::with_max_stack_pow(
        tuple_list!(
            GrimoireExtensionMutator::new(),
            GrimoireRecursiveReplacementMutator::new(),
            GrimoireStringReplacementMutator::new(),
            // give more probability to avoid large inputs
            GrimoireRandomDeleteMutator::new(),
            GrimoireRandomDeleteMutator::new(),
        ),
        3,
    );
    let mut stages = tuple_list!(
        generalization,
        //StdMutationalStage::new(mutator),
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
