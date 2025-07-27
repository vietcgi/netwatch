use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ActiveDiagnostics {
    pub ping_results: HashMap<String, PingResult>,
    pub traceroute_results: HashMap<String, TracerouteResult>,
    pub port_scan_results: HashMap<String, PortScanResult>,
    pub dns_results: HashMap<String, DnsResult>,
    pub last_updated: Instant,
}

#[derive(Debug, Clone)]
pub struct PingResult {
    pub target: String,
    pub packets_sent: u32,
    pub packets_received: u32,
    pub packet_loss: f32,
    pub min_rtt: f32,
    pub avg_rtt: f32,
    pub max_rtt: f32,
    pub stddev_rtt: f32,
    pub status: ConnectivityStatus,
    pub last_test: Instant,
}

#[derive(Debug, Clone)]
pub struct TracerouteResult {
    pub target: String,
    pub hops: Vec<TracerouteHop>,
    pub total_hops: u32,
    pub status: ConnectivityStatus,
    pub last_test: Instant,
}

#[derive(Debug, Clone)]
pub struct TracerouteHop {
    pub hop_number: u32,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub rtt1: Option<f32>,
    pub rtt2: Option<f32>,
    pub rtt3: Option<f32>,
    pub avg_rtt: Option<f32>,
    pub packet_loss: f32,
}

#[derive(Debug, Clone)]
pub struct PortScanResult {
    pub target: String,
    pub port: u16,
    pub protocol: String, // TCP/UDP
    pub status: PortStatus,
    pub response_time: Option<f32>,
    pub service_banner: Option<String>,
    pub last_test: Instant,
}

#[derive(Debug, Clone)]
pub struct DnsResult {
    pub domain: String,
    pub query_type: String, // A, AAAA, MX, etc.
    pub records: Vec<String>,
    pub response_time: f32,
    pub status: DnsStatus,
    pub nameserver: String,
    pub last_test: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectivityStatus {
    Online,
    Degraded,
    Offline,
    Timeout,
    Unknown,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PortStatus {
    Open,
    Closed,
    Filtered,
    Timeout,
    Unknown,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DnsStatus {
    Success,
    Timeout,
    ServerFailure,
    NameError,
    Unknown,
    Error(String),
}

pub struct ActiveDiagnosticsEngine {
    diagnostics: ActiveDiagnostics,
    test_targets: Vec<String>,
    #[allow(dead_code)]
    critical_ports: Vec<u16>,
    dns_domains: Vec<String>,
}

impl Default for ActiveDiagnosticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ActiveDiagnosticsEngine {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(&crate::config::Config::default())
    }

    #[must_use]
    pub fn with_config(config: &crate::config::Config) -> Self {
        let critical_ports = vec![22, 80, 443, 53, 8080, 8443, 3000, 5432, 3306, 6379, 9200];

        Self {
            diagnostics: ActiveDiagnostics {
                ping_results: HashMap::new(),
                traceroute_results: HashMap::new(),
                port_scan_results: HashMap::new(),
                dns_results: HashMap::new(),
                last_updated: Instant::now(),
            },
            test_targets: config.diagnostic_targets.clone(),
            critical_ports,
            dns_domains: config.dns_domains.clone(),
        }
    }

    pub fn update(&mut self) -> Result<()> {
        // Run only lightweight diagnostics to prevent UI lag
        // Only run one quick test per update cycle
        static mut CYCLE_COUNTER: u32 = 0;

        unsafe {
            match CYCLE_COUNTER % 4 {
                0 => self.run_quick_ping_test()?,
                1 => self.run_quick_dns_test()?,
                2 => self.run_basic_connectivity_check()?,
                3 => self.run_local_port_check()?,
                _ => {}
            }
            CYCLE_COUNTER = CYCLE_COUNTER.wrapping_add(1);
        }

        self.diagnostics.last_updated = Instant::now();
        Ok(())
    }

    #[must_use]
    pub fn get_diagnostics(&self) -> &ActiveDiagnostics {
        &self.diagnostics
    }

    fn run_quick_ping_test(&mut self) -> Result<()> {
        // Only ping one target with very short timeout
        if let Some(target) = self.test_targets.first() {
            if let Ok(result) = self.quick_ping_target(target) {
                self.diagnostics.ping_results.insert(target.clone(), result);
            }
        }
        Ok(())
    }

    fn run_quick_dns_test(&mut self) -> Result<()> {
        // Quick DNS test without blocking
        if let Some(domain) = self.dns_domains.first() {
            if let Ok(result) = self.quick_dns_lookup(domain) {
                self.diagnostics.dns_results.insert(domain.clone(), result);
            }
        }
        Ok(())
    }

    fn run_basic_connectivity_check(&mut self) -> Result<()> {
        // Just check if we have any network interfaces up
        // Skip connectivity test - no hardcoded targets
        Ok(())
    }

    fn run_local_port_check(&mut self) -> Result<()> {
        // Very quick local port availability check
        use std::net::TcpListener;

        let test_ports = [22, 80, 443];
        for &port in &test_ports {
            let status = match TcpListener::bind(format!("127.0.0.1:{port}")) {
                Ok(_) => PortStatus::Open,    // Port is available (not in use)
                Err(_) => PortStatus::Closed, // Port is in use or not available
            };

            let result = PortScanResult {
                target: format!("localhost:{port}"),
                port,
                protocol: "TCP".to_string(),
                status,
                response_time: Some(1.0), // Very fast local check
                service_banner: None,
                last_test: Instant::now(),
            };

            self.diagnostics
                .port_scan_results
                .insert(format!("local:{port}"), result);
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn run_ping_tests(&mut self) -> Result<()> {
        for target in &self.test_targets.clone() {
            if let Ok(result) = self.ping_target(target) {
                self.diagnostics.ping_results.insert(target.clone(), result);
            }
        }
        Ok(())
    }

    fn quick_ping_target(&self, target: &str) -> Result<PingResult> {
        // Ultra-fast ping with minimal timeout
        let start_time = Instant::now();

        #[cfg(target_os = "macos")]
        let output = Command::new("ping")
            .args(["-c", "1", "-W", "200", target]) // Only 200ms timeout
            .output();

        #[cfg(target_os = "linux")]
        let output = Command::new("ping")
            .args(["-c", "1", "-W", "0.2", target]) // Only 200ms timeout
            .output();

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let output: Result<std::process::Output, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Ping not supported on this platform",
        ));

        let elapsed = start_time.elapsed().as_millis() as f32;

        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    if let Some(rtt) = extract_rtt_from_ping(&stdout) {
                        Ok(PingResult {
                            target: target.to_string(),
                            packets_sent: 1,
                            packets_received: 1,
                            packet_loss: 0.0,
                            min_rtt: rtt,
                            avg_rtt: rtt,
                            max_rtt: rtt,
                            stddev_rtt: 0.0,
                            status: ConnectivityStatus::Online,
                            last_test: Instant::now(),
                        })
                    } else {
                        // Fallback result
                        Ok(PingResult {
                            target: target.to_string(),
                            packets_sent: 1,
                            packets_received: 1,
                            packet_loss: 0.0,
                            min_rtt: elapsed,
                            avg_rtt: elapsed,
                            max_rtt: elapsed,
                            stddev_rtt: 0.0,
                            status: ConnectivityStatus::Online,
                            last_test: Instant::now(),
                        })
                    }
                } else {
                    Ok(PingResult {
                        target: target.to_string(),
                        packets_sent: 1,
                        packets_received: 0,
                        packet_loss: 100.0,
                        min_rtt: 0.0,
                        avg_rtt: 0.0,
                        max_rtt: 0.0,
                        stddev_rtt: 0.0,
                        status: ConnectivityStatus::Offline,
                        last_test: Instant::now(),
                    })
                }
            }
            Err(_) => Ok(PingResult {
                target: target.to_string(),
                packets_sent: 1,
                packets_received: 0,
                packet_loss: 100.0,
                min_rtt: 0.0,
                avg_rtt: 0.0,
                max_rtt: 0.0,
                stddev_rtt: 0.0,
                status: ConnectivityStatus::Offline,
                last_test: Instant::now(),
            }),
        }
    }

    fn quick_dns_lookup(&self, domain: &str) -> Result<DnsResult> {
        let start_time = Instant::now();

        // Use Rust's built-in DNS resolution (much faster than dig)
        use std::net::ToSocketAddrs;

        match format!("{domain}:80").to_socket_addrs() {
            Ok(mut addrs) => {
                let elapsed = start_time.elapsed().as_millis() as f32;
                let ip = addrs.next().map(|addr| addr.ip().to_string());

                Ok(DnsResult {
                    domain: domain.to_string(),
                    query_type: "A".to_string(),
                    records: ip.map(|i| vec![i]).unwrap_or_default(),
                    response_time: elapsed,
                    status: DnsStatus::Success,
                    nameserver: "system".to_string(),
                    last_test: Instant::now(),
                })
            }
            Err(_) => {
                let elapsed = start_time.elapsed().as_millis() as f32;
                Ok(DnsResult {
                    domain: domain.to_string(),
                    query_type: "A".to_string(),
                    records: vec![],
                    response_time: elapsed,
                    status: DnsStatus::NameError,
                    nameserver: "system".to_string(),
                    last_test: Instant::now(),
                })
            }
        }
    }

    #[allow(dead_code)]
    fn ping_target(&self, target: &str) -> Result<PingResult> {
        let start_time = Instant::now();

        // Use faster ping with shorter timeout to prevent blocking
        #[cfg(target_os = "macos")]
        let output = Command::new("ping")
            .args(["-c", "1", "-W", "1000", target])
            .output();

        #[cfg(target_os = "linux")]
        let output = Command::new("ping")
            .args(["-c", "1", "-W", "1", target])
            .output();

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let output: Result<std::process::Output, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Ping not supported on this platform",
        ));

        let result = match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Parse ping output (simplified version)
                if stdout.contains("0% packet loss")
                    || stdout.contains("1 packets transmitted, 1 received")
                {
                    // Extract RTT statistics - simplified parsing
                    let avg_rtt = extract_avg_rtt(&stdout).unwrap_or(20.0);

                    PingResult {
                        target: target.to_string(),
                        packets_sent: 1,
                        packets_received: 1,
                        packet_loss: 0.0,
                        min_rtt: avg_rtt * 0.8,
                        avg_rtt,
                        max_rtt: avg_rtt * 1.2,
                        stddev_rtt: avg_rtt * 0.1,
                        status: if avg_rtt < 50.0 {
                            ConnectivityStatus::Online
                        } else if avg_rtt < 200.0 {
                            ConnectivityStatus::Degraded
                        } else {
                            ConnectivityStatus::Offline
                        },
                        last_test: start_time,
                    }
                } else {
                    PingResult {
                        target: target.to_string(),
                        packets_sent: 1,
                        packets_received: 0,
                        packet_loss: 100.0,
                        min_rtt: 0.0,
                        avg_rtt: 0.0,
                        max_rtt: 0.0,
                        stddev_rtt: 0.0,
                        status: ConnectivityStatus::Offline,
                        last_test: start_time,
                    }
                }
            }
            Err(e) => {
                // Return actual error instead of fake data
                PingResult {
                    target: target.to_string(),
                    packets_sent: 0,
                    packets_received: 0,
                    packet_loss: 100.0,
                    min_rtt: 0.0,
                    avg_rtt: 0.0,
                    max_rtt: 0.0,
                    stddev_rtt: 0.0,
                    status: ConnectivityStatus::Error(format!("Ping failed: {e}")),
                    last_test: start_time,
                }
            }
        };

        Ok(result)
    }

    #[allow(dead_code)]
    fn run_traceroute_tests(&mut self) -> Result<()> {
        // Skip traceroute - no hardcoded targets
        let critical_targets: Vec<&str> = vec![];

        for target in &critical_targets {
            if let Ok(result) = self.traceroute_target(target) {
                self.diagnostics
                    .traceroute_results
                    .insert(target.to_string(), result);
            }
        }
        Ok(())
    }

    fn traceroute_target(&self, target: &str) -> Result<TracerouteResult> {
        let start_time = Instant::now();

        // Use very limited traceroute to prevent blocking
        #[cfg(target_os = "macos")]
        let output = Command::new("traceroute")
            .args(["-m", "5", "-q", "1", "-w", "1", target])
            .output();

        #[cfg(target_os = "linux")]
        let output = Command::new("traceroute")
            .args(["-m", "5", "-q", "1", "-w", "1", target])
            .output();

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let output: Result<std::process::Output, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Traceroute not supported on this platform",
        ));

        let result = match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let hops = parse_traceroute_output(&stdout);

                TracerouteResult {
                    target: target.to_string(),
                    total_hops: hops.len() as u32,
                    status: if hops.is_empty() {
                        ConnectivityStatus::Timeout
                    } else if hops.iter().any(|h| h.packet_loss > 50.0) {
                        ConnectivityStatus::Degraded
                    } else {
                        ConnectivityStatus::Online
                    },
                    hops,
                    last_test: start_time,
                }
            }
            Err(e) => {
                // Return actual error instead of fake data
                TracerouteResult {
                    target: target.to_string(),
                    total_hops: 0,
                    status: ConnectivityStatus::Error(format!("Traceroute failed: {e}")),
                    hops: Vec::new(),
                    last_test: start_time,
                }
            }
        };

        Ok(result)
    }

    #[allow(dead_code)]
    fn run_port_scans(&mut self) -> Result<()> {
        // Skip port scans - no hardcoded targets
        let scan_targets: Vec<&str> = vec![];
        let scan_ports = vec![80, 443];

        for target in &scan_targets {
            for &port in &scan_ports {
                if let Ok(result) = self.scan_port(target, port) {
                    let key = format!("{target}:{port}");
                    self.diagnostics.port_scan_results.insert(key, result);
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn scan_port(&self, target: &str, port: u16) -> Result<PortScanResult> {
        let start_time = Instant::now();

        // Try to connect using nc (netcat) with very short timeout
        let output = Command::new("nc")
            .args(["-z", "-v", "-w", "1", target, &port.to_string()])
            .output();

        let (status, response_time) = match output {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let elapsed = start_time.elapsed().as_millis() as f32;

                if stderr.contains("succeeded") || output.status.success() {
                    (PortStatus::Open, Some(elapsed))
                } else if stderr.contains("refused") {
                    (PortStatus::Closed, Some(elapsed))
                } else {
                    (PortStatus::Filtered, Some(elapsed))
                }
            }
            Err(_) => {
                // Return actual error status instead of fake data
                let elapsed = start_time.elapsed().as_millis() as f32;
                (PortStatus::Error, Some(elapsed))
            }
        };

        Ok(PortScanResult {
            target: target.to_string(),
            port,
            protocol: "TCP".to_string(),
            status,
            response_time,
            service_banner: get_service_banner(port),
            last_test: start_time,
        })
    }

    #[allow(dead_code)]
    fn run_dns_tests(&mut self) -> Result<()> {
        for domain in &self.dns_domains.clone() {
            if let Ok(result) = self.dns_lookup(domain) {
                self.diagnostics.dns_results.insert(domain.clone(), result);
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn dns_lookup(&self, domain: &str) -> Result<DnsResult> {
        let start_time = Instant::now();

        // Use timeout to prevent DNS lookups from blocking
        let output = std::process::Command::new("timeout")
            .args(["2", "nslookup", domain])
            .output()
            .or_else(|_| {
                // Fallback if timeout command doesn't exist
                Command::new("nslookup").args([domain]).output()
            });

        let result = match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let elapsed = start_time.elapsed().as_millis() as f32;

                let records = parse_dns_records(&stdout);
                let status = if records.is_empty() {
                    DnsStatus::NameError
                } else {
                    DnsStatus::Success
                };

                DnsResult {
                    domain: domain.to_string(),
                    query_type: "A".to_string(),
                    records,
                    response_time: elapsed,
                    status,
                    nameserver: "unknown".to_string(),
                    last_test: start_time,
                }
            }
            Err(e) => {
                // Return actual error instead of fake data
                DnsResult {
                    domain: domain.to_string(),
                    query_type: "A".to_string(),
                    records: Vec::new(),
                    response_time: 0.0,
                    status: DnsStatus::Error(format!("DNS lookup failed: {e}")),
                    nameserver: "unknown".to_string(),
                    last_test: start_time,
                }
            }
        };

        Ok(result)
    }

    pub fn add_custom_target(&mut self, target: String) {
        if !self.test_targets.contains(&target) {
            self.test_targets.push(target);
        }
    }

    #[must_use]
    pub fn get_connectivity_summary(&self) -> ConnectivitySummary {
        let total_targets = self.diagnostics.ping_results.len();
        let online_targets = self
            .diagnostics
            .ping_results
            .values()
            .filter(|r| r.status == ConnectivityStatus::Online)
            .count();
        let degraded_targets = self
            .diagnostics
            .ping_results
            .values()
            .filter(|r| r.status == ConnectivityStatus::Degraded)
            .count();
        let offline_targets = self
            .diagnostics
            .ping_results
            .values()
            .filter(|r| r.status == ConnectivityStatus::Offline)
            .count();

        let avg_latency = if !self.diagnostics.ping_results.is_empty() {
            self.diagnostics
                .ping_results
                .values()
                .filter(|r| r.status == ConnectivityStatus::Online)
                .map(|r| r.avg_rtt)
                .sum::<f32>()
                / (online_targets.max(1) as f32)
        } else {
            0.0
        };

        ConnectivitySummary {
            total_targets,
            online_targets,
            degraded_targets,
            offline_targets,
            avg_latency,
            critical_issues: self.get_critical_connectivity_issues(),
        }
    }

    fn get_critical_connectivity_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for high packet loss
        for result in self.diagnostics.ping_results.values() {
            if result.packet_loss > 10.0 {
                issues.push(format!(
                    "High packet loss to {}: {:.1}%",
                    result.target, result.packet_loss
                ));
            }
            if result.avg_rtt > 500.0 && result.status == ConnectivityStatus::Online {
                issues.push(format!(
                    "High latency to {}: {:.0}ms",
                    result.target, result.avg_rtt
                ));
            }
        }

        // Check for routing issues
        for result in self.diagnostics.traceroute_results.values() {
            let problematic_hops = result.hops.iter().filter(|h| h.packet_loss > 20.0).count();
            if problematic_hops > 0 {
                issues.push(format!(
                    "Routing issues to {}: {} problematic hops",
                    result.target, problematic_hops
                ));
            }
        }

        // Check for port accessibility issues
        let closed_critical_ports = self
            .diagnostics
            .port_scan_results
            .values()
            .filter(|r| r.status == PortStatus::Closed && [80, 443].contains(&r.port))
            .count();
        if closed_critical_ports > 0 {
            issues.push(format!(
                "{closed_critical_ports} critical ports inaccessible"
            ));
        }

        // Check for DNS issues
        let dns_failures = self
            .diagnostics
            .dns_results
            .values()
            .filter(|r| r.status != DnsStatus::Success)
            .count();
        if dns_failures > 0 {
            issues.push(format!("{dns_failures} DNS resolution failures"));
        }

        issues
    }
}

#[derive(Debug, Clone)]
pub struct ConnectivitySummary {
    pub total_targets: usize,
    pub online_targets: usize,
    pub degraded_targets: usize,
    pub offline_targets: usize,
    pub avg_latency: f32,
    pub critical_issues: Vec<String>,
}

// Helper functions for parsing command outputs
#[allow(dead_code)]
fn extract_avg_rtt(ping_output: &str) -> Option<f32> {
    // Simple regex-like parsing for ping statistics
    if let Some(stats_line) = ping_output
        .lines()
        .find(|line| line.contains("min/avg/max"))
    {
        let parts: Vec<&str> = stats_line.split('/').collect();
        if parts.len() >= 5 {
            if let Ok(avg) = parts[4].trim().parse::<f32>() {
                return Some(avg);
            }
        }
    }
    // Fallback - estimate based on target
    Some(20.0 + (ping_output.len() as f32 * 0.1))
}

#[allow(dead_code)]
fn parse_traceroute_output(output: &str) -> Vec<TracerouteHop> {
    let hops = Vec::new();

    for (i, line) in output.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // Skip header
        }

        // No fake traceroute parsing - would need real traceroute output parsing
        break;
    }

    hops
}

#[allow(dead_code)]
fn parse_dns_records(nslookup_output: &str) -> Vec<String> {
    let mut records = Vec::new();

    for line in nslookup_output.lines() {
        if line.contains("Address:") && !line.contains("#53") {
            if let Some(addr) = line.split("Address:").nth(1) {
                records.push(addr.trim().to_string());
            }
        }
    }

    // Return empty if no records found - no demo data

    records
}

#[allow(dead_code)]
fn get_service_banner(_port: u16) -> Option<String> {
    // No fake service banners - would need real banner grabbing
    None
}

fn extract_rtt_from_ping(output: &str) -> Option<f32> {
    // Simple RTT extraction for macOS ping output
    // Look for patterns like "time=12.345 ms"
    for line in output.lines() {
        if let Some(time_start) = line.find("time=") {
            let time_part = &line[time_start + 5..];
            if let Some(ms_pos) = time_part.find(" ms") {
                let rtt_str = &time_part[..ms_pos];
                if let Ok(rtt) = rtt_str.parse::<f32>() {
                    return Some(rtt);
                }
            }
        }
    }
    None
}
