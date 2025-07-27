use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct NetworkConnection {
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub state: ConnectionState,
    pub protocol: Protocol,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    // Enhanced ss command data
    pub socket_info: SocketInfo,
}

#[derive(Debug, Clone, Default)]
pub struct SocketInfo {
    pub rtt: Option<f64>,          // Round trip time in ms
    pub rttvar: Option<f64>,       // RTT variation in ms
    pub cwnd: Option<u32>,         // Congestion window size
    pub ssthresh: Option<u32>,     // Slow start threshold
    pub send_queue: u32,           // Send queue size
    pub recv_queue: u32,           // Receive queue size
    pub bandwidth: Option<u64>,    // Estimated bandwidth
    pub pacing_rate: Option<u64>,  // Pacing rate
    pub retrans: u32,              // Retransmission count
    pub lost: u32,                 // Lost packet count
    pub duration: Option<String>,  // Connection duration
    pub interface: Option<String>, // Network interface
    pub tcp_info: Option<TcpInfo>, // Extended TCP information
}

#[derive(Debug, Clone)]
pub struct TcpInfo {
    pub mss: u32,                   // Maximum segment size
    pub pmtu: u32,                  // Path MTU
    pub rcv_mss: u32,               // Receive MSS
    pub advmss: u32,                // Advertised MSS
    pub cwnd_clamp: u32,            // Congestion window clamp
    pub delivery_rate: Option<u64>, // Delivery rate
    pub app_limited: bool,          // Application limited
    pub reordering: u32,            // Packet reordering metric
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Established,
    Listen,
    SynSent,
    SynReceived,
    FinWait1,
    FinWait2,
    TimeWait,
    Close,
    CloseWait,
    LastAck,
    Closing,
    Unknown,
}

impl ConnectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionState::Established => "ESTABLISHED",
            ConnectionState::Listen => "LISTEN",
            ConnectionState::SynSent => "SYN_SENT",
            ConnectionState::SynReceived => "SYN_RECV",
            ConnectionState::FinWait1 => "FIN_WAIT1",
            ConnectionState::FinWait2 => "FIN_WAIT2",
            ConnectionState::TimeWait => "TIME_WAIT",
            ConnectionState::Close => "CLOSE",
            ConnectionState::CloseWait => "CLOSE_WAIT",
            ConnectionState::LastAck => "LAST_ACK",
            ConnectionState::Closing => "CLOSING",
            ConnectionState::Unknown => "UNKNOWN",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            ConnectionState::Established => Color::Green,
            ConnectionState::Listen => Color::Blue,
            ConnectionState::SynSent | ConnectionState::SynReceived => Color::Yellow,
            ConnectionState::FinWait1
            | ConnectionState::FinWait2
            | ConnectionState::TimeWait
            | ConnectionState::CloseWait
            | ConnectionState::LastAck
            | ConnectionState::Closing => Color::Red,
            ConnectionState::Close => Color::Gray,
            ConnectionState::Unknown => Color::Magenta,
        }
    }
}

impl FromStr for ConnectionState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "01" => Ok(ConnectionState::Established),
            "02" => Ok(ConnectionState::SynSent),
            "03" => Ok(ConnectionState::SynReceived),
            "04" => Ok(ConnectionState::FinWait1),
            "05" => Ok(ConnectionState::FinWait2),
            "06" => Ok(ConnectionState::TimeWait),
            "07" => Ok(ConnectionState::Close),
            "08" => Ok(ConnectionState::CloseWait),
            "09" => Ok(ConnectionState::LastAck),
            "0A" | "10" => Ok(ConnectionState::Listen),
            "0B" | "11" => Ok(ConnectionState::Closing),
            _ => Ok(ConnectionState::Unknown),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Tcp,
    Udp,
    Tcp6,
    Udp6,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Tcp => "TCP",
            Protocol::Udp => "UDP",
            Protocol::Tcp6 => "TCP6",
            Protocol::Udp6 => "UDP6",
        }
    }
}

pub struct ConnectionMonitor {
    connections: Vec<NetworkConnection>,
    process_cache: HashMap<u32, String>,
}

impl ConnectionMonitor {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            process_cache: HashMap::new(),
        }
    }

    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Clear existing connections to get fresh data
        self.connections.clear();

        // On macOS, skip ss command entirely and go straight to netstat/lsof
        #[cfg(target_os = "macos")]
        {
            self.read_tcp_connections()?;
            self.read_udp_connections()?;
            // Skip process info update as it may fail on macOS in some environments
            let _ = self.update_process_info();
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Try using ss command for rich socket information (Linux/modern systems)
            if self.read_ss_connections().is_ok() {
                // ss command succeeded, we have rich data
            } else {
                // Fallback to /proc parsing or demo data
                self.read_tcp_connections()?;
                self.read_udp_connections()?;

                // Update process information
                self.update_process_info()?;
            }
        }

        // Sort by connection quality (RTT first, then bytes transferred)
        self.connections.sort_by(|a, b| {
            // First sort by connection health (lower RTT = better)
            match (a.socket_info.rtt, b.socket_info.rtt) {
                (Some(rtt_a), Some(rtt_b)) => rtt_a
                    .partial_cmp(&rtt_b)
                    .unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => {
                    // Fallback to bytes transferred
                    (b.bytes_sent + b.bytes_received).cmp(&(a.bytes_sent + a.bytes_received))
                }
            }
        });

        Ok(())
    }

    #[allow(dead_code)]
    fn read_ss_connections(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;

        // Execute ss command with comprehensive options for rich socket data
        let output = Command::new("ss")
            .args(["-tupln", "-i", "-e", "-p"]) // TCP/UDP, processes, listening, numeric, internal, extended
            .output()?;

        if !output.status.success() {
            return Err("ss command failed".into());
        }

        let content = String::from_utf8_lossy(&output.stdout);
        self.parse_ss_output(&content)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn parse_ss_output(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip header line
            if line.starts_with("Netid") || line.is_empty() {
                i += 1;
                continue;
            }

            // Parse main connection line
            if let Some(connection) = self.parse_ss_connection_line(line)? {
                // Look for additional lines with socket details
                let mut socket_info = SocketInfo::default();

                // Check next lines for extended information
                i += 1;
                while i < lines.len() {
                    let next_line = lines[i].trim();

                    // If next line starts with socket details, parse it
                    if next_line.starts_with("cubic")
                        || next_line.starts_with("rto:")
                        || next_line.contains("rtt:")
                    {
                        self.parse_socket_details(next_line, &mut socket_info)?;
                        i += 1;
                    } else {
                        // This line doesn't belong to current connection
                        break;
                    }
                }

                let mut conn = connection;
                conn.socket_info = socket_info;
                self.connections.push(conn);
            } else {
                i += 1;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn parse_ss_connection_line(
        &self,
        line: &str,
    ) -> Result<Option<NetworkConnection>, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Ok(None);
        }

        // Parse protocol
        let protocol = match parts[0] {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            "tcp6" => Protocol::Tcp6,
            "udp6" => Protocol::Udp6,
            _ => return Ok(None),
        };

        // Parse state
        let state = match parts[1] {
            "ESTAB" => ConnectionState::Established,
            "LISTEN" => ConnectionState::Listen,
            "SYN-SENT" => ConnectionState::SynSent,
            "SYN-RECV" => ConnectionState::SynReceived,
            "FIN-WAIT-1" => ConnectionState::FinWait1,
            "FIN-WAIT-2" => ConnectionState::FinWait2,
            "TIME-WAIT" => ConnectionState::TimeWait,
            "CLOSE" => ConnectionState::Close,
            "CLOSE-WAIT" => ConnectionState::CloseWait,
            "LAST-ACK" => ConnectionState::LastAck,
            "CLOSING" => ConnectionState::Closing,
            _ => ConnectionState::Unknown,
        };

        // Parse queue sizes (recv-q send-q)
        let recv_queue = parts[2].parse().unwrap_or(0);
        let send_queue = parts[3].parse().unwrap_or(0);

        // Parse local address
        let local_addr = self.parse_address(parts[4])?;

        // Parse remote address
        let remote_addr = if parts.len() > 5 && parts[5] != "*:*" {
            self.parse_address(parts[5])?
        } else {
            SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)), 0)
        };

        // Extract process information if available
        let (pid, process_name) =
            if let Some(process_part) = parts.iter().find(|p| p.starts_with("users:")) {
                self.parse_process_info(process_part)?
            } else {
                (None, None)
            };

        let socket_info = SocketInfo {
            recv_queue,
            send_queue,
            ..Default::default()
        };

        Ok(Some(NetworkConnection {
            local_addr,
            remote_addr,
            state,
            protocol,
            pid,
            process_name,
            bytes_sent: 0, // Will be populated from extended info if available
            bytes_received: 0,
            socket_info,
        }))
    }

    fn parse_address_string(
        &self,
        addr_str: &str,
    ) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        // Parse macOS netstat format: 192.168.86.21.58412 or [fe80::1]:22
        if addr_str.contains("*:*") || addr_str == "*" {
            return Ok(SocketAddr::new(
                IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                0,
            ));
        }

        if addr_str.starts_with('[') {
            // IPv6 format: [::1]:22 or [fe80::1]:22
            let end_bracket = addr_str.find(']').ok_or("Invalid IPv6 format")?;
            let ip_str = &addr_str[1..end_bracket];
            let port_str = &addr_str[end_bracket + 2..]; // Skip ']:'
            let ip = ip_str.parse()?;
            let port = port_str.parse()?;
            Ok(SocketAddr::new(ip, port))
        } else {
            // IPv4 format: 192.168.86.21.58412 (note: port is after last dot)
            let last_dot = addr_str.rfind('.').ok_or("Invalid address format")?;
            let ip_str = &addr_str[..last_dot];
            let port_str = &addr_str[last_dot + 1..];
            let ip = ip_str.parse()?;
            let port = port_str.parse()?;
            Ok(SocketAddr::new(ip, port))
        }
    }

    #[allow(dead_code)]
    fn parse_address(&self, addr_str: &str) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        // Handle IPv4 and IPv6 addresses from ss output
        if addr_str.starts_with('[') {
            // IPv6 format: [::1]:22
            let end_bracket = addr_str.find(']').ok_or("Invalid IPv6 format")?;
            let ip_str = &addr_str[1..end_bracket];
            let port_str = &addr_str[end_bracket + 2..]; // Skip ']:'
            let ip = ip_str.parse()?;
            let port = port_str.parse()?;
            Ok(SocketAddr::new(ip, port))
        } else {
            // IPv4 format: 192.168.1.1:80
            let parts: Vec<&str> = addr_str.rsplitn(2, ':').collect();
            if parts.len() != 2 {
                return Err("Invalid address format".into());
            }
            let port = parts[0].parse()?;
            let ip = parts[1].parse()?;
            Ok(SocketAddr::new(ip, port))
        }
    }

    #[allow(dead_code)]
    fn parse_process_info(
        &self,
        process_part: &str,
    ) -> Result<(Option<u32>, Option<String>), Box<dyn std::error::Error>> {
        // Parse format like: users:(("sshd",pid=1234,fd=3))
        if let Some(start) = process_part.find("pid=") {
            let pid_part = &process_part[start + 4..];
            if let Some(end) = pid_part.find(',') {
                let pid_str = &pid_part[..end];
                if let Ok(pid) = pid_str.parse::<u32>() {
                    // Extract process name
                    if let Some(name_start) = process_part.find("\"") {
                        if let Some(name_end) = process_part[name_start + 1..].find("\"") {
                            let name = &process_part[name_start + 1..name_start + 1 + name_end];
                            return Ok((Some(pid), Some(name.to_string())));
                        }
                    }
                    return Ok((Some(pid), None));
                }
            }
        }
        Ok((None, None))
    }

    #[allow(dead_code)]
    fn parse_socket_details(
        &self,
        line: &str,
        socket_info: &mut SocketInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse detailed socket information from ss output
        for part in line.split_whitespace() {
            if let Some(rtt_part) = part.strip_prefix("rtt:") {
                // Parse RTT: rtt:12.5/24.0ms
                if let Some(slash_pos) = rtt_part.find('/') {
                    let rtt_str = &rtt_part[..slash_pos];
                    socket_info.rtt = rtt_str.parse().ok();

                    let rttvar_part = &rtt_part[slash_pos + 1..];
                    if let Some(ms_pos) = rttvar_part.find("ms") {
                        let rttvar_str = &rttvar_part[..ms_pos];
                        socket_info.rttvar = rttvar_str.parse().ok();
                    }
                }
            } else if let Some(cwnd_part) = part.strip_prefix("cwnd:") {
                socket_info.cwnd = cwnd_part.parse().ok();
            } else if let Some(ssthresh_part) = part.strip_prefix("ssthresh:") {
                socket_info.ssthresh = ssthresh_part.parse().ok();
            } else if part.starts_with("pacing_rate") {
                // Parse pacing_rate 1.2Mbps
                if let Some(rate_str) = part.split(':').nth(1) {
                    socket_info.pacing_rate = self.parse_bandwidth(rate_str);
                }
            } else if let Some(retrans_part) = part.strip_prefix("retrans:") {
                // Parse retrans:0/10
                if let Some(slash_pos) = retrans_part.find('/') {
                    socket_info.retrans = retrans_part[..slash_pos].parse().unwrap_or(0);
                    socket_info.lost = retrans_part[slash_pos + 1..].parse().unwrap_or(0);
                }
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn parse_bandwidth(&self, bw_str: &str) -> Option<u64> {
        let bw_str = bw_str.trim();
        if let Some(kbps_part) = bw_str.strip_suffix("Kbps") {
            kbps_part.parse::<f64>().ok().map(|n| (n * 1000.0) as u64)
        } else if let Some(mbps_part) = bw_str.strip_suffix("Mbps") {
            mbps_part
                .parse::<f64>()
                .ok()
                .map(|n| (n * 1_000_000.0) as u64)
        } else if let Some(gbps_part) = bw_str.strip_suffix("Gbps") {
            gbps_part
                .parse::<f64>()
                .ok()
                .map(|n| (n * 1_000_000_000.0) as u64)
        } else {
            bw_str.parse().ok()
        }
    }

    fn read_tcp_connections(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Try Linux /proc filesystem first
        if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
            self.parse_connections(&content, Protocol::Tcp)?;
        } else {
            // macOS - get real connection data from system commands
            self.create_real_connections_from_system(Protocol::Tcp);
        }

        if let Ok(content) = fs::read_to_string("/proc/net/tcp6") {
            self.parse_connections(&content, Protocol::Tcp6)?;
        } else {
            // macOS fallback
            self.create_real_connections_from_system(Protocol::Tcp6);
        }

        Ok(())
    }

    fn read_udp_connections(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Read IPv4 UDP connections
        if let Ok(content) = fs::read_to_string("/proc/net/udp") {
            self.parse_connections(&content, Protocol::Udp)?;
        } else {
            self.create_real_connections_from_system(Protocol::Udp);
        }

        // Read IPv6 UDP connections
        if let Ok(content) = fs::read_to_string("/proc/net/udp6") {
            self.parse_connections(&content, Protocol::Udp6)?;
        } else {
            self.create_real_connections_from_system(Protocol::Udp6);
        }

        Ok(())
    }

    fn parse_connections(
        &mut self,
        content: &str,
        protocol: Protocol,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for line in content.lines().skip(1) {
            // Skip header
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 10 {
                continue;
            }

            // Parse local and remote addresses
            let local_addr = self.parse_socket_addr(fields[1])?;
            let remote_addr = self.parse_socket_addr(fields[2])?;

            // Parse connection state
            let state = ConnectionState::from_str(fields[3]).unwrap_or(ConnectionState::Unknown);

            // Parse PID (if available in field 7)
            let pid = if fields.len() > 7 {
                fields[7].parse().ok()
            } else {
                None
            };

            // Create connection
            let connection = NetworkConnection {
                local_addr,
                remote_addr,
                state,
                protocol: protocol.clone(),
                pid,
                process_name: None, // Will be filled later
                bytes_sent: 0,      // Would need additional parsing from /proc/net/netstat
                bytes_received: 0,
                socket_info: SocketInfo::default(),
            };

            self.connections.push(connection);
        }

        Ok(())
    }

    fn parse_socket_addr(&self, addr_str: &str) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = addr_str.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid socket address format".into());
        }

        // Parse IP address (hex format)
        let ip_hex = parts[0];
        let port_hex = parts[1];

        let port = u16::from_str_radix(port_hex, 16)?;

        // Parse IP address based on length
        let ip = if ip_hex.len() == 8 {
            // IPv4 address in hex (little-endian)
            let ip_num = u32::from_str_radix(ip_hex, 16)?;
            let ip_bytes = [
                (ip_num & 0xFF) as u8,
                ((ip_num >> 8) & 0xFF) as u8,
                ((ip_num >> 16) & 0xFF) as u8,
                ((ip_num >> 24) & 0xFF) as u8,
            ];
            IpAddr::V4(ip_bytes.into())
        } else if ip_hex.len() == 32 {
            // IPv6 address in hex
            let mut ip_bytes = [0u8; 16];
            for i in 0..16 {
                ip_bytes[i] = u8::from_str_radix(&ip_hex[i * 2..i * 2 + 2], 16)?;
            }
            IpAddr::V6(ip_bytes.into())
        } else {
            return Err("Invalid IP address length".into());
        };

        Ok(SocketAddr::new(ip, port))
    }

    fn update_process_info(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Build PID to process name mapping
        let mut pid_to_name = HashMap::new();

        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if let Ok(pid) = file_name.parse::<u32>() {
                        let comm_path = format!("/proc/{pid}/comm");
                        if let Ok(process_name) = fs::read_to_string(comm_path) {
                            pid_to_name.insert(pid, process_name.trim().to_string());
                        }
                    }
                }
            }
        }

        // Update connection process names
        for connection in &mut self.connections {
            if let Some(pid) = connection.pid {
                connection.process_name = pid_to_name.get(&pid).cloned();
            }
        }

        self.process_cache = pid_to_name;
        Ok(())
    }

    pub fn get_connections(&self) -> &[NetworkConnection] {
        &self.connections
    }

    pub fn get_connection_stats(&self) -> ConnectionStats {
        let mut stats = ConnectionStats::default();

        for conn in &self.connections {
            match conn.state {
                ConnectionState::Established => stats.established += 1,
                ConnectionState::Listen => stats.listening += 1,
                ConnectionState::TimeWait => stats.time_wait += 1,
                _ => stats.other += 1,
            }

            match conn.protocol {
                Protocol::Tcp | Protocol::Tcp6 => stats.tcp += 1,
                Protocol::Udp | Protocol::Udp6 => stats.udp += 1,
            }

            stats.total += 1;
        }

        stats
    }

    pub fn get_top_processes(&self) -> Vec<(String, u32)> {
        let mut process_counts: HashMap<String, u32> = HashMap::new();

        for conn in &self.connections {
            if let Some(process_name) = &conn.process_name {
                *process_counts.entry(process_name.clone()).or_insert(0) += 1;
            }
        }

        let mut sorted_processes: Vec<(String, u32)> = process_counts.into_iter().collect();
        sorted_processes.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_processes.truncate(10); // Top 10

        sorted_processes
    }

    pub fn get_remote_hosts(&self) -> Vec<(IpAddr, u32)> {
        let mut host_counts: HashMap<IpAddr, u32> = HashMap::new();

        for conn in &self.connections {
            if conn.state == ConnectionState::Established {
                *host_counts.entry(conn.remote_addr.ip()).or_insert(0) += 1;
            }
        }

        let mut sorted_hosts: Vec<(IpAddr, u32)> = host_counts.into_iter().collect();
        sorted_hosts.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_hosts.truncate(10); // Top 10

        sorted_hosts
    }
}

#[derive(Default)]
pub struct ConnectionStats {
    pub total: u32,
    pub established: u32,
    pub listening: u32,
    pub time_wait: u32,
    pub other: u32,
    pub tcp: u32,
    pub udp: u32,
}

impl ConnectionMonitor {
    fn create_real_connections_from_system(&mut self, protocol: Protocol) {
        // Use system commands to get real connection data instead of fake demo data
        self.get_connections_from_netstat(protocol);
    }

    fn get_connections_from_netstat(&mut self, protocol: Protocol) {
        use std::process::Command;

        let protocol_flag = match protocol {
            Protocol::Tcp => "tcp",
            Protocol::Tcp6 => "tcp6",
            Protocol::Udp => "udp",
            Protocol::Udp6 => "udp6",
        };

        // Use netstat to get real connection data
        let output = Command::new("netstat")
            .args(["-n", "-p", protocol_flag])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_netstat_output(&stdout, protocol);
            }
            Err(_e) => {
                // If netstat fails, try lsof as fallback
                self.get_connections_from_lsof(protocol);
            }
        }
    }

    fn get_connections_from_lsof(&mut self, protocol: Protocol) {
        use std::process::Command;

        let protocol_flag = match protocol {
            Protocol::Tcp | Protocol::Tcp6 => "TCP",
            Protocol::Udp | Protocol::Udp6 => "UDP",
        };

        let output = Command::new("lsof")
            .args(["-i", protocol_flag, "-n"])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.parse_lsof_output(&stdout, protocol);
        }
        // If both netstat and lsof fail, just leave connections empty instead of fake data
    }

    fn parse_netstat_output(&mut self, output: &str, protocol: Protocol) {
        // Parse real netstat output to create NetworkConnection objects
        for line in output.lines().skip(2) {
            // Skip headers
            if let Some(connection) = self.parse_netstat_line(line, &protocol) {
                self.connections.push(connection);
            }
        }
    }

    fn parse_lsof_output(&mut self, output: &str, protocol: Protocol) {
        // Parse real lsof output to create NetworkConnection objects
        for line in output.lines().skip(1) {
            // Skip header
            if let Some(connection) = self.parse_lsof_line(line, &protocol) {
                self.connections.push(connection);
            }
        }
    }

    fn parse_netstat_line(&self, line: &str, protocol: &Protocol) -> Option<NetworkConnection> {
        // Parse macOS netstat format: tcp4  0  0  192.168.86.21.58412  34.36.57.103.443  ESTABLISHED
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            return None;
        }

        // Skip header lines
        if parts[0] == "Proto" || parts[0] == "Active" {
            return None;
        }

        // Parse local and remote addresses (parts[3] and parts[4])
        let local_addr = self.parse_address_string(parts[3]).ok()?;
        let remote_addr = self.parse_address_string(parts[4]).unwrap_or_else(|_| {
            std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)), 0)
        });

        // Parse state (parts[5])
        let state = match parts[5] {
            "ESTABLISHED" => ConnectionState::Established,
            "LISTEN" => ConnectionState::Listen,
            "TIME_WAIT" => ConnectionState::TimeWait,
            "CLOSE_WAIT" => ConnectionState::CloseWait,
            "FIN_WAIT_1" => ConnectionState::FinWait1,
            "FIN_WAIT_2" => ConnectionState::FinWait2,
            "SYN_SENT" => ConnectionState::SynSent,
            "SYN_RECV" => ConnectionState::SynReceived,
            "CLOSING" => ConnectionState::Closing,
            "LAST_ACK" => ConnectionState::LastAck,
            _ => ConnectionState::Unknown,
        };

        Some(NetworkConnection {
            local_addr,
            remote_addr,
            state,
            protocol: protocol.clone(),
            pid: None,
            process_name: None,
            bytes_sent: 0,
            bytes_received: 0,
            socket_info: SocketInfo::default(),
        })
    }

    fn parse_lsof_line(&self, line: &str, protocol: &Protocol) -> Option<NetworkConnection> {
        // Parse macOS lsof format: command pid user fd type device size/off node name
        // Example: rapportd 699 kevin 8u IPv4 0x666a2de494291f52 0t0 TCP *:64566 (LISTEN)
        // Example: identitys 721 kevin 36u IPv6 0xe327a3d736b97a9a 0t0 TCP [fe80:18::3af0:34ed:86c8:f8bc]:1024->[fe80:18::190d:c0da:7b3c:37fa]:1024 (ESTABLISHED)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            return None;
        }

        // Skip header line
        if parts[0] == "COMMAND" {
            return None;
        }

        let process_name = Some(parts[0].to_string());
        let pid = parts[1].parse::<u32>().ok();

        // Find the TCP/UDP part and connection info (usually last few parts)
        let network_part = parts
            .iter()
            .find(|&&part| part.contains("->") || (part.contains(":") && !part.contains("0x")))?;

        // Parse connection state from the last part in parentheses
        let state = if let Some(state_part) = parts.last() {
            match state_part.trim_matches(|c| c == '(' || c == ')') {
                "ESTABLISHED" => ConnectionState::Established,
                "LISTEN" => ConnectionState::Listen,
                "TIME_WAIT" => ConnectionState::TimeWait,
                "CLOSE_WAIT" => ConnectionState::CloseWait,
                "SYN_SENT" => ConnectionState::SynSent,
                "SYN_RECV" => ConnectionState::SynReceived,
                "FIN_WAIT1" => ConnectionState::FinWait1,
                "FIN_WAIT2" => ConnectionState::FinWait2,
                "CLOSING" => ConnectionState::Closing,
                "LAST_ACK" => ConnectionState::LastAck,
                _ => ConnectionState::Unknown,
            }
        } else {
            ConnectionState::Unknown
        };

        // Parse addresses based on format
        if let Some((local_str, remote_str)) = network_part.split_once("->") {
            // Established connection with local->remote
            let local_addr = self.parse_lsof_address(local_str).ok()?;
            let remote_addr = self.parse_lsof_address(remote_str).ok()?;

            return Some(NetworkConnection {
                local_addr,
                remote_addr,
                state,
                protocol: protocol.clone(),
                pid,
                process_name,
                bytes_sent: 0,
                bytes_received: 0,
                socket_info: SocketInfo::default(),
            });
        } else if network_part.contains(":") {
            // Listening socket (format: *:port or ip:port)
            let local_addr = self.parse_lsof_address(network_part).ok()?;
            let remote_addr = std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                0,
            );

            return Some(NetworkConnection {
                local_addr,
                remote_addr,
                state,
                protocol: protocol.clone(),
                pid,
                process_name,
                bytes_sent: 0,
                bytes_received: 0,
                socket_info: SocketInfo::default(),
            });
        }

        None
    }

    fn parse_lsof_address(&self, addr_str: &str) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        // Parse lsof address formats:
        // *:64566 (listening on all interfaces)
        // [fe80:18::3af0:34ed:86c8:f8bc]:1024 (IPv6)
        // 192.168.1.1:80 (IPv4)

        if let Some(port_str) = addr_str.strip_prefix("*:") {
            // Wildcard listening address
            let port = port_str.parse()?;
            return Ok(SocketAddr::new(
                IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                port,
            ));
        }

        if addr_str.starts_with('[') {
            // IPv6 format: [fe80::1]:22
            let end_bracket = addr_str.find(']').ok_or("Invalid IPv6 format")?;
            let ip_str = &addr_str[1..end_bracket];
            let port_str = &addr_str[end_bracket + 2..]; // Skip ']:'
            let ip = ip_str.parse()?;
            let port = port_str.parse()?;
            Ok(SocketAddr::new(ip, port))
        } else {
            // IPv4 format: 192.168.1.1:80
            let parts: Vec<&str> = addr_str.rsplitn(2, ':').collect();
            if parts.len() != 2 {
                return Err("Invalid address format".into());
            }
            let port = parts[0].parse()?;
            let ip = parts[1].parse()?;
            Ok(SocketAddr::new(ip, port))
        }
    }
}

impl Default for ConnectionMonitor {
    fn default() -> Self {
        Self::new()
    }
}
