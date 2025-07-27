//! Security monitoring and intrusion detection for netwatch
//!
//! This module provides security monitoring capabilities to detect
//! potential attacks and suspicious behavior during operation.

use crate::error::{NetwatchError, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Security event types that can be monitored
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityEvent {
    /// Invalid input attempt (injection, traversal, etc.)
    InvalidInput {
        input_type: String,
        attempted_value: String,
        source: String,
    },
    /// Suspicious file access pattern
    SuspiciousFileAccess { path: String, access_type: String },
    /// Rate limiting triggered
    RateLimitExceeded { source: String, attempt_count: u32 },
    /// Configuration tampering detected
    ConfigTampering {
        config_field: String,
        old_value: String,
        new_value: String,
    },
    /// Resource exhaustion attempt
    ResourceExhaustion {
        resource_type: String,
        usage_amount: u64,
        limit: u64,
    },
}

/// Security monitor that tracks and analyzes security events
pub struct SecurityMonitor {
    events: Vec<(Instant, SecurityEvent)>,
    event_counts: HashMap<String, u32>,
    rate_limits: HashMap<String, (Instant, u32)>,
    max_events: usize,
}

impl SecurityMonitor {
    /// Create a new security monitor
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            event_counts: HashMap::new(),
            rate_limits: HashMap::new(),
            max_events: 1000, // Keep last 1000 events
        }
    }

    /// Record a security event
    pub fn record_event(&mut self, event: SecurityEvent) {
        let now = Instant::now();

        // Add to event log
        self.events.push((now, event.clone()));

        // Maintain size limit
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }

        // Update event counts
        let event_key = self.event_key(&event);
        *self.event_counts.entry(event_key).or_insert(0) += 1;

        // Log critical events
        if self.is_critical_event(&event) {
            eprintln!("SECURITY ALERT: {event:?}");
        }
    }

    /// Check if rate limiting should be applied
    pub fn check_rate_limit(&mut self, source: &str, max_per_minute: u32) -> Result<()> {
        let now = Instant::now();
        let key = format!("rate_limit_{source}");

        match self.rate_limits.get_mut(&key) {
            Some((last_reset, count)) => {
                if now.duration_since(*last_reset) > Duration::from_secs(60) {
                    // Reset counter after 1 minute
                    *last_reset = now;
                    *count = 1;
                } else {
                    *count += 1;
                    if *count > max_per_minute {
                        return Err(NetwatchError::Security(format!(
                            "Rate limit exceeded for source: {source}"
                        )));
                    }
                }
            }
            None => {
                self.rate_limits.insert(key, (now, 1));
            }
        }

        Ok(())
    }

    /// Get security event statistics
    pub fn get_statistics(&self) -> SecurityStatistics {
        let now = Instant::now();
        let last_hour = now - Duration::from_secs(3600);
        let last_day = now - Duration::from_secs(86400);

        let events_last_hour = self
            .events
            .iter()
            .filter(|(time, _)| *time > last_hour)
            .count();

        let events_last_day = self
            .events
            .iter()
            .filter(|(time, _)| *time > last_day)
            .count();

        let critical_events = self
            .events
            .iter()
            .filter(|(_, event)| self.is_critical_event(event))
            .count();

        SecurityStatistics {
            total_events: self.events.len(),
            events_last_hour,
            events_last_day,
            critical_events,
            event_types: self.event_counts.clone(),
        }
    }

    /// Check for security anomalies
    pub fn check_anomalies(&self) -> Vec<SecurityAnomaly> {
        let mut anomalies = Vec::new();
        let now = Instant::now();
        let last_minute = now - Duration::from_secs(60);

        // Check for burst of events
        let recent_events = self
            .events
            .iter()
            .filter(|(time, _)| *time > last_minute)
            .count();

        if recent_events > 10 {
            anomalies.push(SecurityAnomaly::EventBurst {
                event_count: recent_events,
                time_window: Duration::from_secs(60),
            });
        }

        // Check for repeated invalid inputs
        let mut invalid_input_sources = HashMap::new();
        for (time, event) in &self.events {
            if *time > last_minute {
                if let SecurityEvent::InvalidInput { source, .. } = event {
                    *invalid_input_sources.entry(source.clone()).or_insert(0) += 1;
                }
            }
        }

        for (source, count) in invalid_input_sources {
            if count > 3 {
                anomalies.push(SecurityAnomaly::RepeatedInvalidInput {
                    source,
                    attempt_count: count,
                });
            }
        }

        anomalies
    }

    fn event_key(&self, event: &SecurityEvent) -> String {
        match event {
            SecurityEvent::InvalidInput { input_type, .. } => format!("invalid_input_{input_type}"),
            SecurityEvent::SuspiciousFileAccess { .. } => "suspicious_file_access".to_string(),
            SecurityEvent::RateLimitExceeded { .. } => "rate_limit_exceeded".to_string(),
            SecurityEvent::ConfigTampering { .. } => "config_tampering".to_string(),
            SecurityEvent::ResourceExhaustion { .. } => "resource_exhaustion".to_string(),
        }
    }

    fn is_critical_event(&self, event: &SecurityEvent) -> bool {
        matches!(
            event,
            SecurityEvent::ConfigTampering { .. }
                | SecurityEvent::SuspiciousFileAccess { .. }
                | SecurityEvent::ResourceExhaustion { .. }
        )
    }
}

impl Default for SecurityMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Security statistics for monitoring
#[derive(Debug, Clone)]
pub struct SecurityStatistics {
    pub total_events: usize,
    pub events_last_hour: usize,
    pub events_last_day: usize,
    pub critical_events: usize,
    pub event_types: HashMap<String, u32>,
}

/// Security anomalies detected by the monitor
#[derive(Debug, Clone)]
pub enum SecurityAnomaly {
    /// Burst of security events in short time
    EventBurst {
        event_count: usize,
        time_window: Duration,
    },
    /// Repeated invalid input from same source
    RepeatedInvalidInput { source: String, attempt_count: u32 },
    /// Unusual access pattern detected
    UnusualAccessPattern {
        pattern_description: String,
        confidence: f32,
    },
}

/// Global security monitor instance
static mut SECURITY_MONITOR: Option<SecurityMonitor> = None;
static mut MONITOR_INITIALIZED: bool = false;

/// Initialize the global security monitor
pub fn init_security_monitor() {
    unsafe {
        if !MONITOR_INITIALIZED {
            SECURITY_MONITOR = Some(SecurityMonitor::new());
            MONITOR_INITIALIZED = true;
        }
    }
}

/// Record a security event to the global monitor
pub fn record_security_event(event: SecurityEvent) {
    unsafe {
        if let Some(ref mut monitor) = SECURITY_MONITOR {
            monitor.record_event(event);
        }
    }
}

/// Check rate limit using the global monitor
pub fn check_security_rate_limit(source: &str, max_per_minute: u32) -> Result<()> {
    unsafe {
        if let Some(ref mut monitor) = SECURITY_MONITOR {
            monitor.check_rate_limit(source, max_per_minute)
        } else {
            Ok(())
        }
    }
}

/// Get security statistics from the global monitor
#[allow(static_mut_refs)]
pub fn get_security_statistics() -> Option<SecurityStatistics> {
    unsafe {
        SECURITY_MONITOR
            .as_ref()
            .map(|monitor| monitor.get_statistics())
    }
}

/// Check for security anomalies
#[allow(static_mut_refs)]
pub fn check_security_anomalies() -> Vec<SecurityAnomaly> {
    unsafe {
        SECURITY_MONITOR
            .as_ref()
            .map(|monitor| monitor.check_anomalies())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_event_recording() {
        let mut monitor = SecurityMonitor::new();

        let event = SecurityEvent::InvalidInput {
            input_type: "interface_name".to_string(),
            attempted_value: "../etc/passwd".to_string(),
            source: "cli".to_string(),
        };

        monitor.record_event(event);

        let stats = monitor.get_statistics();
        assert_eq!(stats.total_events, 1);
        assert_eq!(
            stats.event_types.get("invalid_input_interface_name"),
            Some(&1)
        );
    }

    #[test]
    fn test_rate_limiting() {
        let mut monitor = SecurityMonitor::new();

        // First few attempts should succeed
        for _i in 0..3 {
            assert!(monitor.check_rate_limit("test_source", 5).is_ok());
        }

        // Should still be under limit
        assert!(monitor.check_rate_limit("test_source", 5).is_ok());
        assert!(monitor.check_rate_limit("test_source", 5).is_ok());

        // This should exceed the limit
        assert!(monitor.check_rate_limit("test_source", 5).is_err());
    }

    #[test]
    fn test_anomaly_detection() {
        let mut monitor = SecurityMonitor::new();

        // Generate burst of events
        for i in 0..15 {
            let event = SecurityEvent::InvalidInput {
                input_type: "test".to_string(),
                attempted_value: format!("attack_{i}"),
                source: "attacker".to_string(),
            };
            monitor.record_event(event);
        }

        let anomalies = monitor.check_anomalies();
        assert!(!anomalies.is_empty());

        // Should detect event burst
        assert!(anomalies
            .iter()
            .any(|a| matches!(a, SecurityAnomaly::EventBurst { .. })));

        // Should detect repeated invalid input
        assert!(anomalies
            .iter()
            .any(|a| matches!(a, SecurityAnomaly::RepeatedInvalidInput { .. })));
    }

    #[test]
    fn test_critical_event_detection() {
        let mut monitor = SecurityMonitor::new();

        let critical_event = SecurityEvent::ConfigTampering {
            config_field: "log_file".to_string(),
            old_value: "/tmp/netwatch.log".to_string(),
            new_value: "/etc/passwd".to_string(),
        };

        monitor.record_event(critical_event);

        let stats = monitor.get_statistics();
        assert_eq!(stats.critical_events, 1);
    }
}
