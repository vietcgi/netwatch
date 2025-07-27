#![no_main]
use libfuzzer_sys::fuzz_target;
use netwatch::validation::validate_interface_name;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        // Should handle any string input without panicking
        let _ = validate_interface_name(input);
    }
});