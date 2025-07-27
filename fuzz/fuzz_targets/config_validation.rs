#![no_main]
use libfuzzer_sys::fuzz_target;
use netwatch::validation::{validate_file_path, validate_config_string, sanitize_user_input};

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        // Test file path validation
        let _ = validate_file_path(input, None);
        let _ = validate_file_path(input, Some("log"));
        
        // Test config string validation
        let _ = validate_config_string(input, "test_field");
        
        // Test input sanitization
        let _ = sanitize_user_input(input, 1024);
    }
});