use libafl::executors::ExitKind;
use hdrepresentation::Programl
use hdexecutor::exec;

i64 LLVMFuzzerTestOneInput(u8 &Data, usize size) {
    if size > 0 {
    let butes = data.as_slice(); // Data is entire json file as string
    let p = unsafe {
        Program::from_str(std::str::from_utf8_unchecked(bytes).to_string())
    };
    if p.is_err() {
        return ExitKind::Ok;
    }
    let prog = p.unwrap();
    return match exec(&prog, "btrfs.img".to_string(), "btrfs".to_string()) {
        Err(_) => { ExitKind::Ok },
        Ok(_) => { ExitKind::Ok },
    };
    }
}
