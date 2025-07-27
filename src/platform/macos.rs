use crate::{
    device::{NetworkReader, NetworkStats},
    error::{NetwatchError, Result},
};
use std::ffi::CStr;
use std::ptr;
use std::time::SystemTime;

pub struct MacOSReader;

impl Default for MacOSReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOSReader {
    pub fn new() -> Self {
        Self
    }
}

impl NetworkReader for MacOSReader {
    fn list_devices(&self) -> Result<Vec<String>> {
        // Use getifaddrs to list network interfaces
        unsafe {
            let mut ifap: *mut libc::ifaddrs = ptr::null_mut();
            if libc::getifaddrs(&mut ifap) != 0 {
                return Err(NetwatchError::Platform(
                    "Failed to get interface list".to_string(),
                ));
            }

            let mut devices = Vec::new();
            let mut current = ifap;

            while !current.is_null() {
                let ifa = &*current;

                if !ifa.ifa_name.is_null() {
                    let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().to_string();

                    // Only include unique interfaces and filter out virtual/loopback interfaces
                    if !devices.contains(&name) && !name.starts_with("lo") {
                        devices.push(name);
                    }
                }

                current = ifa.ifa_next;
            }

            libc::freeifaddrs(ifap);
            Ok(devices)
        }
    }

    fn read_stats(&self, device: &str) -> Result<NetworkStats> {
        // Use a more robust approach - simply parse /proc/net/dev equivalent on macOS
        // On macOS, we'll use netstat command as a fallback until we get proper sysctl working
        use std::process::Command;

        let output = Command::new("netstat").args(["-I", device, "-b"]).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let lines: Vec<&str> = stdout.lines().collect();

                    // Find the line with our interface data
                    for line in lines {
                        if let Some(stats_line) = line.strip_prefix(&format!("{device:<10}")) {
                            let parts: Vec<&str> = stats_line.split_whitespace().collect();
                            if parts.len() >= 10 {
                                // Parse netstat output: [mtu] [network] [address] [ipkts] [ierrs] [ibytes] [opkts] [oerrs] [obytes] [coll]
                                if let (
                                    Ok(packets_in),
                                    Ok(errors_in),
                                    Ok(bytes_in),
                                    Ok(packets_out),
                                    Ok(errors_out),
                                    Ok(bytes_out),
                                ) = (
                                    parts[3].parse::<u64>(), // ipkts
                                    parts[4].parse::<u64>(), // ierrs
                                    parts[5].parse::<u64>(), // ibytes
                                    parts[6].parse::<u64>(), // opkts
                                    parts[7].parse::<u64>(), // oerrs
                                    parts[8].parse::<u64>(), // obytes
                                ) {
                                    return Ok(NetworkStats {
                                        timestamp: SystemTime::now(),
                                        bytes_in,
                                        bytes_out,
                                        packets_in,
                                        packets_out,
                                        errors_in,
                                        errors_out,
                                        drops_in: 0, // netstat doesn't provide drop info in this format
                                        drops_out: 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback to zero stats if netstat fails
                return Ok(NetworkStats {
                    timestamp: SystemTime::now(),
                    bytes_in: 0,
                    bytes_out: 0,
                    packets_in: 0,
                    packets_out: 0,
                    errors_in: 0,
                    errors_out: 0,
                    drops_in: 0,
                    drops_out: 0,
                });
            }
        }

        Err(NetwatchError::DeviceNotFound(device.to_string()))
    }

    fn is_available(&self) -> bool {
        // Always available on macOS
        true
    }
}
