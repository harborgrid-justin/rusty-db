// # Network Hardening Manager
//
// Integrated network hardening system orchestrator.

use std::net::IpAddr;
use std::sync::Arc;
use std::collections::HashMap;
use crate::Result;
use serde::{Serialize, Deserialize};

use super::firewall_rules::*;
use super::rate_limiting::*;
use super::intrusion_detection::*;

// ============================================================================
// Integrated Network Hardening Manager
// ============================================================================

pub struct NetworkHardeningManager {
    pub rate_limiter: Arc<AdaptiveRateLimiter>,
    pub connection_guard: Arc<ConnectionGuard>,
    pub ddos_mitigator: Arc<DDoSMitigator>,
    pub protocol_validator: Arc<ProtocolValidator>,
    pub tls_enforcer: Arc<TLSEnforcer>,
    pub anomaly_detector: Arc<NetworkAnomalyDetector>,
    pub ip_reputation: Arc<IPReputationChecker>,
}

impl NetworkHardeningManager {
    pub fn new() -> Self {
        let ip_reputation = Arc::new(IPReputationChecker::new(ReputationConfig::default()));

        Self {
            rate_limiter: Arc::new(AdaptiveRateLimiter::new(
                RateLimitConfig::default(),
                ip_reputation.clone(),
            )),
            connection_guard: Arc::new(ConnectionGuard::new(ConnectionLimits::default())),
            ddos_mitigator: Arc::new(DDoSMitigator::new(DDoSThresholds::default())),
            protocol_validator: Arc::new(ProtocolValidator::new(ValidationRules::default())),
            tls_enforcer: Arc::new(TLSEnforcer::new(TLSConfig::default())),
            anomaly_detector: Arc::new(NetworkAnomalyDetector::new()),
            ip_reputation,
        }
    }

    pub fn check_request(
        &self,
        ip: IpAddr,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        body_size: usize,
    ) -> Result<bool> {
        // 1. IP reputation check
        if self.ip_reputation.is_blacklisted(&ip) {
            return Ok(false);
        }

        // 2. Connection guard
        if !self.connection_guard.check_connection(ip)? {
            self.ip_reputation.record_violation(ip, ViolationType::RateLimitExceeded);
            return Ok(false);
        }

        // 3. Rate limiting
        let rate_key = format!("ip:{}", ip);
        if !self.rate_limiter.check_rate_limit(&rate_key, ip)? {
            self.ip_reputation.record_violation(ip, ViolationType::RateLimitExceeded);
            return Ok(false);
        }

        // 4. Protocol validation
        if let Err(_) = self.protocol_validator.validate_request(method, uri, headers, body_size) {
            self.ip_reputation.record_violation(ip, ViolationType::ProtocolViolation);
            return Ok(false);
        }

        // 5. DDoS detection
        let user_agent = headers.get("User-Agent").cloned();
        let analysis = self.ddos_mitigator.analyze_request(
            ip,
            body_size,
            uri.to_string(),
            user_agent,
        )?;

        match analysis {
            DDoSAnalysisResult::Blocked(_) => {
                self.ip_reputation.record_violation(ip, ViolationType::DDoSAttempt);
                return Ok(false);
            }
            DDoSAnalysisResult::Suspicious => {
                self.ip_reputation.record_violation(ip, ViolationType::SuspiciousPattern);
            }
            DDoSAnalysisResult::Clean => {}
        }

        // Record successful check
        self.ip_reputation.record_request(ip, true);

        Ok(true)
    }

    pub fn get_all_stats(&self) -> NetworkHardeningStats {
        NetworkHardeningStats {
            rate_limit: self.rate_limiter.get_stats(),
            connection: self.connection_guard.get_stats(),
            ddos: self.ddos_mitigator.get_stats(),
            validation: self.protocol_validator.get_stats(),
            tls: self.tls_enforcer.get_stats(),
            anomaly: self.anomaly_detector.get_stats(),
            reputation: self.ip_reputation.get_stats(),
        }
    }
}

impl Default for NetworkHardeningManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHardeningStats {
    pub rate_limit: RateLimitStats,
    pub connection: ConnectionStats,
    pub ddos: DDoSStats,
    pub validation: ValidationStats,
    pub tls: TLSStats,
    pub anomaly: AnomalyStats,
    pub reputation: ReputationStats,
}
