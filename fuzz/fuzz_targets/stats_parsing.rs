#![no_main]
use libfuzzer_sys::fuzz_target;
use netwatch::platform::LinuxReader;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let reader = LinuxReader::new();
        // This should never panic, only return errors for malformed data
        let _ = reader.parse_proc_net_dev(input, "eth0");
    }
});