use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpInfo {
    pub country: String,
    pub country_code: String,
    pub city: String,
    pub region: String,
    pub is_internal: bool,
    pub is_suspicious: bool,
    pub threat_level: ThreatLevel,
    pub organization: String,
    pub asn: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatLevel {
    Clean,
    Suspicious,
    Malicious,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ConnectionIntelligence {
    pub remote_ip: IpAddr,
    pub local_port: u16,
    pub remote_port: u16,
    pub protocol: String,
    pub service_name: String,
    pub geo_info: Option<GeoIpInfo>,
    pub connection_duration: Duration,
    pub bytes_transferred: u64,
    pub packet_count: u64,
    pub first_seen: SystemTime,
    pub last_activity: SystemTime,
    pub is_outbound: bool,
    pub threat_indicators: Vec<ThreatIndicator>,
}

#[derive(Debug, Clone)]
pub enum ThreatIndicator {
    PortScanAttempt {
        ports_scanned: u16,
        time_window: Duration,
    },
    UnusualTrafficVolume {
        bytes_per_second: u64,
        baseline: u64,
    },
    SuspiciousPort {
        port: u16,
        reason: String,
    },
    GeoAnomalyConnection {
        country: String,
        reason: String,
    },
    RapidConnections {
        count: u32,
        time_window: Duration,
    },
    LongLivedConnection {
        duration: Duration,
    },
    HighBandwidthUsage {
        bandwidth: u64,
        threshold: u64,
    },
}

#[derive(Debug, Clone)]
pub struct PortScanDetection {
    pub scanner_ip: IpAddr,
    pub ports_scanned: HashSet<u16>,
    pub scan_start_time: SystemTime,
    pub scan_duration: Duration,
    pub scan_rate: f64,  // ports per second
    pub confidence: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone)]
pub struct NetworkAnomaly {
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub description: String,
    pub affected_ip: Option<IpAddr>,
    pub affected_port: Option<u16>,
    pub detected_at: SystemTime,
    pub confidence: f64,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub enum AnomalyType {
    PortScan,
    TrafficSpike,
    UnusualGeoLocation,
    SuspiciousProtocol,
    BandwidthAnomaly,
    ConnectionFlood,
    DnsAnomaly,
    TunnelDetection,
}

#[derive(Debug, Clone)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

pub struct NetworkIntelligenceEngine {
    connection_history: VecDeque<ConnectionIntelligence>,
    geo_cache: HashMap<IpAddr, GeoIpInfo>,
    port_scan_detectors: HashMap<IpAddr, PortScanDetection>,
    anomalies: VecDeque<NetworkAnomaly>,
    #[allow(dead_code)]
    traffic_baselines: HashMap<String, TrafficBaseline>,
    known_services: HashMap<u16, String>,
    suspicious_ips: HashSet<IpAddr>,
    internal_networks: Vec<(IpAddr, u8)>, // CIDR notation
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TrafficBaseline {
    average_bps: f64,
    std_deviation: f64,
    samples: VecDeque<(SystemTime, u64)>,
    last_updated: SystemTime,
}

impl NetworkIntelligenceEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            connection_history: VecDeque::with_capacity(10000),
            geo_cache: HashMap::new(),
            port_scan_detectors: HashMap::new(),
            anomalies: VecDeque::with_capacity(1000),
            traffic_baselines: HashMap::new(),
            known_services: Self::initialize_known_services(),
            suspicious_ips: HashSet::new(),
            internal_networks: Self::initialize_internal_networks(),
        };

        // Pre-populate with some threat intelligence
        engine.load_threat_intelligence();

        engine
    }

    fn initialize_known_services() -> HashMap<u16, String> {
        let mut services = HashMap::new();

        // Well-known ports
        services.insert(22, "SSH".to_string());
        services.insert(23, "Telnet".to_string());
        services.insert(25, "SMTP".to_string());
        services.insert(53, "DNS".to_string());
        services.insert(80, "HTTP".to_string());
        services.insert(110, "POP3".to_string());
        services.insert(143, "IMAP".to_string());
        services.insert(443, "HTTPS".to_string());
        services.insert(993, "IMAPS".to_string());
        services.insert(995, "POP3S".to_string());
        services.insert(3389, "RDP".to_string());
        services.insert(5432, "PostgreSQL".to_string());
        services.insert(3306, "MySQL".to_string());
        services.insert(27017, "MongoDB".to_string());
        services.insert(6379, "Redis".to_string());
        services.insert(9200, "Elasticsearch".to_string());
        services.insert(8080, "HTTP-Alt".to_string());
        services.insert(8443, "HTTPS-Alt".to_string());

        // Common malicious ports
        services.insert(1337, "Elite/Leet (Suspicious)".to_string());
        services.insert(31337, "Back Orifice (Malware)".to_string());
        services.insert(12345, "NetBus (Malware)".to_string());
        services.insert(54321, "Back Orifice 2000 (Malware)".to_string());

        services
    }

    fn initialize_internal_networks() -> Vec<(IpAddr, u8)> {
        vec![
            (IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8), // 10.0.0.0/8
            (IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0)), 12), // 172.16.0.0/12
            (IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0)), 16), // 192.168.0.0/16
            (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0)), 8), // 127.0.0.0/8 (loopback)
            (IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0)), 7), // fc00::/7 (ULA)
        ]
    }

    fn load_threat_intelligence(&mut self) {
        // No pre-populated threat intelligence - load from external sources only
        // In production, this would load from real threat feeds:
        // - Abuse.ch feeds
        // - SANS ISC feeds
        // - Custom threat intelligence feeds
        // For now, keep empty - no fake data
    }

    pub fn analyze_connection(
        &mut self,
        connection: &crate::connections::NetworkConnection,
    ) -> ConnectionIntelligence {
        let remote_ip = connection.remote_addr.ip();
        let local_port = connection.local_addr.port();
        let remote_port = connection.remote_addr.port();

        // Determine if connection is outbound
        let is_outbound = self.is_internal_ip(&connection.local_addr.ip());

        // Get or create GeoIP info
        let geo_info = self.get_geo_info(&remote_ip);

        // Determine protocol and service
        let (protocol, service_name) = self.identify_service(
            local_port,
            remote_port,
            &format!("{:?}", connection.protocol),
        );

        // Calculate metrics (use duration from socket_info if available)
        let connection_duration = connection
            .socket_info
            .duration
            .as_ref()
            .and_then(|d| parse_duration(d))
            .unwrap_or_else(|| Duration::from_secs(0));

        // Detect threat indicators
        let mut threat_indicators = Vec::new();

        // Check for port scanning
        if let Some(scan_detection) = self.detect_port_scan(&remote_ip, remote_port) {
            threat_indicators.push(ThreatIndicator::PortScanAttempt {
                ports_scanned: scan_detection.ports_scanned.len() as u16,
                time_window: scan_detection.scan_duration,
            });
        }

        // Check for suspicious ports
        if self.is_suspicious_port(remote_port) {
            threat_indicators.push(ThreatIndicator::SuspiciousPort {
                port: remote_port,
                reason: "Known malicious or uncommon port".to_string(),
            });
        }

        // Check bandwidth usage
        let bytes_per_second = if connection_duration.as_secs() > 0 {
            connection.bytes_sent + connection.bytes_received / connection_duration.as_secs()
        } else {
            0
        };

        if bytes_per_second > 10_000_000 {
            // >10MB/s threshold
            threat_indicators.push(ThreatIndicator::HighBandwidthUsage {
                bandwidth: bytes_per_second,
                threshold: 10_000_000,
            });
        }

        // Check for geo anomalies
        if let Some(ref geo) = geo_info {
            if geo.is_suspicious || geo.threat_level != ThreatLevel::Clean {
                threat_indicators.push(ThreatIndicator::GeoAnomalyConnection {
                    country: geo.country.clone(),
                    reason: "Connection from suspicious geographic location".to_string(),
                });
            }
        }

        ConnectionIntelligence {
            remote_ip,
            local_port,
            remote_port,
            protocol: protocol.clone(),
            service_name,
            geo_info,
            connection_duration,
            bytes_transferred: connection.bytes_sent + connection.bytes_received,
            packet_count: 0, // Not available in current connection struct
            first_seen: SystemTime::now() - connection_duration, // Estimate based on duration
            last_activity: SystemTime::now(),
            is_outbound,
            threat_indicators,
        }
    }

    fn get_geo_info(&mut self, ip: &IpAddr) -> Option<GeoIpInfo> {
        // Check cache first
        if let Some(cached) = self.geo_cache.get(ip) {
            return Some(cached.clone());
        }

        // Skip internal IPs
        if self.is_internal_ip(ip) {
            let internal_info = GeoIpInfo {
                country: "Internal".to_string(),
                country_code: "INT".to_string(),
                city: "Local Network".to_string(),
                region: "Private".to_string(),
                is_internal: true,
                is_suspicious: false,
                threat_level: ThreatLevel::Clean,
                organization: "Internal Network".to_string(),
                asn: 0,
            };
            self.geo_cache.insert(*ip, internal_info.clone());
            return Some(internal_info);
        }

        // Simplified GeoIP lookup (in real implementation, use MaxMind GeoIP2 or similar)
        let geo_info = self.mock_geo_lookup(ip);
        self.geo_cache.insert(*ip, geo_info.clone());
        Some(geo_info)
    }

    fn mock_geo_lookup(&self, ip: &IpAddr) -> GeoIpInfo {
        // No fake geo data - return unknown for all IPs
        // In production, integrate with real GeoIP service like MaxMind
        let is_suspicious = self.suspicious_ips.contains(ip);

        let threat_level = if is_suspicious {
            ThreatLevel::Malicious
        } else {
            ThreatLevel::Clean
        };

        GeoIpInfo {
            country: "Unknown".to_string(),
            country_code: "UN".to_string(),
            city: "Unknown".to_string(),
            region: "Unknown".to_string(),
            is_internal: false,
            is_suspicious,
            threat_level,
            organization: "Unknown".to_string(),
            asn: 0, // Unknown ASN
        }
    }

    fn is_internal_ip(&self, ip: &IpAddr) -> bool {
        for (network, prefix_len) in &self.internal_networks {
            if self.ip_in_cidr(ip, network, *prefix_len) {
                return true;
            }
        }
        false
    }

    fn ip_in_cidr(&self, ip: &IpAddr, network: &IpAddr, prefix_len: u8) -> bool {
        match (ip, network) {
            (IpAddr::V4(ip), IpAddr::V4(net)) => {
                let ip_u32 = u32::from(*ip);
                let net_u32 = u32::from(*net);
                let mask = if prefix_len == 0 {
                    0
                } else {
                    !((1u32 << (32 - prefix_len)) - 1)
                };
                (ip_u32 & mask) == (net_u32 & mask)
            }
            (IpAddr::V6(ip), IpAddr::V6(net)) => {
                let ip_bytes = ip.octets();
                let net_bytes = net.octets();
                let full_bytes = prefix_len / 8;
                let remaining_bits = prefix_len % 8;

                // Check full bytes
                for i in 0..full_bytes as usize {
                    if ip_bytes[i] != net_bytes[i] {
                        return false;
                    }
                }

                // Check remaining bits
                if remaining_bits > 0 && full_bytes < 16 {
                    let mask = 0xFF << (8 - remaining_bits);
                    if (ip_bytes[full_bytes as usize] & mask)
                        != (net_bytes[full_bytes as usize] & mask)
                    {
                        return false;
                    }
                }

                true
            }
            _ => false,
        }
    }

    fn identify_service(
        &self,
        local_port: u16,
        remote_port: u16,
        protocol: &str,
    ) -> (String, String) {
        let port_to_check = if local_port < 1024 {
            local_port
        } else {
            remote_port
        };

        let service_name = self
            .known_services
            .get(&port_to_check)
            .cloned()
            .unwrap_or_else(|| {
                if port_to_check < 1024 {
                    "System Service".to_string()
                } else if port_to_check < 49152 {
                    "Registered Service".to_string()
                } else {
                    "Dynamic/Ephemeral".to_string()
                }
            });

        (protocol.to_uppercase(), service_name)
    }

    fn detect_port_scan(&mut self, ip: &IpAddr, port: u16) -> Option<PortScanDetection> {
        let now = SystemTime::now();

        // Check if we already have a detector for this IP
        let mut updated_detector = if let Some(existing) = self.port_scan_detectors.get(ip) {
            let mut detector = existing.clone();
            detector.ports_scanned.insert(port);
            detector.scan_duration = now
                .duration_since(detector.scan_start_time)
                .unwrap_or_default();

            // Calculate scan rate
            if detector.scan_duration.as_secs() > 0 {
                detector.scan_rate =
                    detector.ports_scanned.len() as f64 / detector.scan_duration.as_secs_f64();
            }

            detector
        } else {
            // Create new detector
            let mut detector = PortScanDetection {
                scanner_ip: *ip,
                ports_scanned: HashSet::new(),
                scan_start_time: now,
                scan_duration: Duration::from_secs(0),
                scan_rate: 0.0,
                confidence: 0.0,
            };
            detector.ports_scanned.insert(port);
            detector
        };

        // Calculate confidence
        updated_detector.confidence = self.calculate_port_scan_confidence(&updated_detector);

        // Update the detector in the map
        self.port_scan_detectors
            .insert(*ip, updated_detector.clone());

        // Clean up old detectors (older than 5 minutes)
        let cutoff = now - Duration::from_secs(300);
        self.port_scan_detectors
            .retain(|_, detector| detector.scan_start_time > cutoff);

        // Return detector if confidence is high enough
        if updated_detector.confidence > 0.7 {
            Some(updated_detector)
        } else {
            None
        }
    }

    fn calculate_port_scan_confidence(&self, detector: &PortScanDetection) -> f64 {
        let mut confidence = 0.0;

        // Number of ports scanned
        let port_count = detector.ports_scanned.len() as f64;
        confidence += (port_count / 20.0).min(0.4); // Max 0.4 for port count

        // Scan rate (ports per second)
        if detector.scan_rate > 10.0 {
            confidence += 0.3; // Very fast scanning
        } else if detector.scan_rate > 1.0 {
            confidence += 0.2; // Moderate scanning
        }

        // Sequential port scanning pattern
        let mut ports: Vec<u16> = detector.ports_scanned.iter().cloned().collect();
        ports.sort();
        let sequential_count = self.count_sequential_ports(&ports);
        if sequential_count > 5 {
            confidence += 0.2;
        }

        // Common port scan targets
        let common_scan_ports: HashSet<u16> =
            [22, 23, 80, 443, 21, 25, 53, 110, 143, 993, 995, 3389]
                .iter()
                .cloned()
                .collect();
        let scan_common_count = detector
            .ports_scanned
            .intersection(&common_scan_ports)
            .count();
        if scan_common_count > 3 {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }

    fn count_sequential_ports(&self, ports: &[u16]) -> usize {
        if ports.len() < 2 {
            return 0;
        }

        let mut max_sequential = 0;
        let mut current_sequential = 1;

        for i in 1..ports.len() {
            if ports[i] == ports[i - 1] + 1 {
                current_sequential += 1;
            } else {
                max_sequential = max_sequential.max(current_sequential);
                current_sequential = 1;
            }
        }

        max_sequential.max(current_sequential)
    }

    fn is_suspicious_port(&self, port: u16) -> bool {
        // Known malicious ports
        matches!(port, 1337 | 31337 | 12345 | 54321 | 6667 | 6668 | 6669)
    }

    pub fn get_recent_anomalies(&self, limit: usize) -> Vec<&NetworkAnomaly> {
        self.anomalies.iter().rev().take(limit).collect()
    }

    pub fn get_port_scan_alerts(&self) -> Vec<PortScanDetection> {
        self.port_scan_detectors
            .values()
            .filter(|detector| detector.confidence > 0.7)
            .cloned()
            .collect()
    }

    pub fn get_connection_stats(&self) -> ConnectionStats {
        let total_connections = self.connection_history.len();
        let external_connections = self
            .connection_history
            .iter()
            .filter(|conn| !conn.geo_info.as_ref().map_or(true, |geo| geo.is_internal))
            .count();

        let suspicious_connections = self
            .connection_history
            .iter()
            .filter(|conn| !conn.threat_indicators.is_empty())
            .count();

        let countries: HashSet<String> = self
            .connection_history
            .iter()
            .filter_map(|conn| conn.geo_info.as_ref())
            .filter(|geo| !geo.is_internal)
            .map(|geo| geo.country.clone())
            .collect();

        ConnectionStats {
            total_connections,
            external_connections,
            suspicious_connections,
            unique_countries: countries.len(),
            active_port_scans: self.port_scan_detectors.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub external_connections: usize,
    pub suspicious_connections: usize,
    pub unique_countries: usize,
    pub active_port_scans: usize,
}

impl Default for NetworkIntelligenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to parse duration strings from ss command output
fn parse_duration(duration_str: &str) -> Option<Duration> {
    // Parse duration strings like "1h30m", "45m", "30s", etc.
    let mut total_secs = 0u64;
    let mut current_num = String::new();

    for ch in duration_str.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            if let Ok(num) = current_num.parse::<u64>() {
                match ch {
                    'h' => total_secs += num * 3600,
                    'm' => total_secs += num * 60,
                    's' => total_secs += num,
                    _ => {}
                }
            }
            current_num.clear();
        }
    }

    if total_secs > 0 {
        Some(Duration::from_secs(total_secs))
    } else {
        None
    }
}
