// # Network Hardening Module
//
// Military-grade network security hardening with modular components.

pub mod firewall_rules;
pub mod rate_limiting;
pub mod intrusion_detection;
pub mod manager;

// Re-export main types
pub use firewall_rules::{
    IPReputationChecker, IPReputation, ViolationType,
    ReputationConfig, ReputationStats,
    ConnectionGuard, ConnectionLimits, ConnectionStats,
};

pub use rate_limiting::{
    AdaptiveRateLimiter, RateLimitConfig, TokenBucket, SlidingWindow, RateLimitStats,
    DDoSMitigator, DDoSThresholds, DDoSAttack, DDoSAttackType,
    MitigationAction, DDoSStats, DDoSAnalysisResult,
};

pub use intrusion_detection::{
    ProtocolValidator, ValidationRules, ValidationStats,
    TLSEnforcer, TLSConfig, TLSVersion, TLSStats,
    NetworkAnomalyDetector, Anomaly, AnomalyType, AnomalyStats,
};

pub use manager::{
    NetworkHardeningManager, NetworkHardeningStats,
};
