// # Network Hardening Module
//
// Military-grade network security hardening with modular components.

pub mod firewall_rules;
pub mod intrusion_detection;
pub mod manager;
pub mod rate_limiting;

// Re-export main types
pub use firewall_rules::{
    ConnectionGuard, ConnectionLimits, ConnectionStats, IPReputation, IPReputationChecker,
    ReputationConfig, ReputationStats, ViolationType,
};

pub use rate_limiting::{
    AdaptiveRateLimiter, DDoSAnalysisResult, DDoSAttack, DDoSAttackType, DDoSMitigator, DDoSStats,
    DDoSThresholds, MitigationAction, RateLimitConfig, RateLimitStats, SlidingWindow, TokenBucket,
};

pub use intrusion_detection::{
    Anomaly, AnomalyStats, AnomalyType, NetworkAnomalyDetector, ProtocolValidator, TLSConfig,
    TLSEnforcer, TLSStats, TLSVersion, ValidationRules, ValidationStats,
};

pub use manager::{NetworkHardeningManager, NetworkHardeningStats};
