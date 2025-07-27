use crate::{device::NetworkReader, error::Result};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxReader;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacOSReader;

pub fn create_reader() -> Result<Box<dyn NetworkReader>> {
    #[cfg(target_os = "linux")]
    return Ok(Box::new(LinuxReader::new()));

    #[cfg(target_os = "macos")]
    return Ok(Box::new(MacOSReader::new()));

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Err(crate::error::NetwatchError::Platform(
        "Unsupported platform".to_string(),
    ));
}
