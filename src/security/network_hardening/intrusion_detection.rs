// # Intrusion Detection
//
// Protocol validation, TLS enforcement, and network anomaly detection.

use std::time::{SystemTime, Instant};
use std::collections::{HashSet, HashMap, VecDeque};
use crate::{Result, DbError};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

// Constants
const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024;
const MAX_HEADER_COUNT: usize = 100;
const MAX_HEADER_SIZE: usize = 8192;
const ANOMALY_WINDOW_SIZE: usize = 1000;
const ANOMALY_Z_SCORE_THRESHOLD: f64 = 3.0;

// ============================================================================
// Protocol Validator
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    pub max_request_size: usize,
    pub max_header_count: usize,
    pub max_header_size: usize,
    pub max_uri_length: usize,
    pub allowed_methods: HashSet<String>,
    pub allowed_content_types: HashSet<String>,
    pub require_host_header: bool,
    pub require_user_agent: bool,
}

impl Default for ValidationRules {
    fn default() -> Self {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert("GET".to_string());
        allowed_methods.insert("POST".to_string());
        allowed_methods.insert("PUT".to_string());
        allowed_methods.insert("DELETE".to_string());
        allowed_methods.insert("PATCH".to_string());
        allowed_methods.insert("HEAD".to_string());
        allowed_methods.insert("OPTIONS".to_string());

        let mut allowed_content_types = HashSet::new();
        allowed_content_types.insert("application/json".to_string());
        allowed_content_types.insert("application/x-www-form-urlencoded".to_string());
        allowed_content_types.insert("multipart/form-data".to_string());
        allowed_content_types.insert("text/plain".to_string());

        Self {
            max_request_size: MAX_REQUEST_SIZE,
            max_header_count: MAX_HEADER_COUNT,
            max_header_size: MAX_HEADER_SIZE,
            max_uri_length: 2048,
            allowed_methods,
            allowed_content_types,
            require_host_header: true,
            require_user_agent: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationStats {
    pub total_validated: u64,
    pub validation_passed: u64,
    pub validation_failed: u64,
    pub method_violations: u64,
    pub size_violations: u64,
    pub header_violations: u64,
    pub protocol_violations: u64,
}

pub struct ProtocolValidator {
    rules: Arc<RwLock<ValidationRules>>,
    stats: Arc<RwLock<ValidationStats>>,
}

impl ProtocolValidator {
    pub fn new(rules: ValidationRules) -> Self {
        Self {
            rules: Arc::new(RwLock::new(rules)),
            stats: Arc::new(RwLock::new(ValidationStats::default())),
        }
    }

    pub fn validate_request(
        &self,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        bodysize: usize,
    ) -> Result<()> {
        let mut stats = self.stats.write();
        stats.total_validated += 1;

        let rules = self.rules.read();

        if !rules.allowed_methods.contains(method) {
            stats.validation_failed += 1;
            stats.method_violations += 1;
            return Err(DbError::InvalidOperation(format!("Invalid method: {}", method)));
        }

        if uri.len() > rules.max_uri_length {
            stats.validation_failed += 1;
            stats.size_violations += 1;
            return Err(DbError::InvalidOperation("URI too long".to_string()));
        }

        if headers.len() > rules.max_header_count {
            stats.validation_failed += 1;
            stats.header_violations += 1;
            return Err(DbError::InvalidOperation("Too many headers".to_string()));
        }

        for (key, value) in headers {
            if key.len() + value.len() > rules.max_header_size {
                stats.validation_failed += 1;
                stats.header_violations += 1;
                return Err(DbError::InvalidOperation("Header too large".to_string()));
            }
        }

        if rules.require_host_header && !headers.contains_key("Host") {
            stats.validation_failed += 1;
            stats.header_violations += 1;
            return Err(DbError::InvalidOperation("Missing Host header".to_string()));
        }

        if bodysize > rules.max_request_size {
            stats.validation_failed += 1;
            stats.size_violations += 1;
            return Err(DbError::InvalidOperation("Request body too large".to_string()));
        }

        if let Some(content_type) = headers.get("Content-Type") {
            let ct = content_type.split(';').next().unwrap_or("").trim();
            if !rules.allowed_content_types.contains(ct) {
                stats.validation_failed += 1;
                stats.protocol_violations += 1;
                return Err(DbError::InvalidOperation(format!("Invalid content type: {}", ct)));
            }
        }

        stats.validation_passed += 1;
        Ok(())
    }

    pub fn get_stats(&self) -> ValidationStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// TLS Enforcer
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TLSVersion {
    TLS12,
    TLS13,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TLSConfig {
    pub min_version: TLSVersion,
    pub require_pfs: bool,
    pub allowed_ciphers: Vec<String>,
    pub require_client_cert: bool,
    pub enable_cert_pinning: bool,
    pub enable_ocsp_stapling: bool,
}

impl Default for TLSConfig {
    fn default() -> Self {
        Self {
            min_version: TLSVersion::TLS13,
            require_pfs: true,
            allowed_ciphers: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            require_client_cert: false,
            enable_cert_pinning: true,
            enable_ocsp_stapling: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TLSStats {
    pub connections_established: u64,
    pub handshake_failures: u64,
    pub cert_validation_failures: u64,
    pub pinning_violations: u64,
    pub protocol_downgrades_blocked: u64,
    pub weak_cipher_rejected: u64,
}

pub struct TLSEnforcer {
    config: Arc<RwLock<TLSConfig>>,
    pinned_certs: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    stats: Arc<RwLock<TLSStats>>,
}

impl TLSEnforcer {
    pub fn new(config: TLSConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            pinned_certs: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(TLSStats::default())),
        }
    }

    pub fn pin_certificate(&self, host: String, cert_der: Vec<u8>) {
        let fingerprint = Self::calculate_fingerprint(&cert_der);
        self.pinned_certs.write().insert(host, fingerprint);
    }

    pub fn validate_connection(
        &self,
        version: TLSVersion,
        cipher: &str,
        cert_der: Option<&[u8]>,
        host: Option<&str>,
    ) -> Result<()> {
        let mut stats = self.stats.write();
        let config = self.config.read();

        if version < config.min_version {
            stats.protocol_downgrades_blocked += 1;
            return Err(DbError::InvalidOperation(format!(
                "TLS version {:?} below minimum {:?}",
                version, config.min_version
            )));
        }

        if !config.allowed_ciphers.iter().any(|c| c == cipher) {
            stats.weak_cipher_rejected += 1;
            return Err(DbError::InvalidOperation(format!("Cipher not allowed: {}", cipher)));
        }

        if config.require_pfs && !Self::cipher_supports_pfs(cipher) {
            stats.weak_cipher_rejected += 1;
            return Err(DbError::InvalidOperation("Cipher does not support PFS".to_string()));
        }

        if config.enable_cert_pinning {
            if let (Some(cert), Some(host)) = (cert_der, host) {
                let fingerprint = Self::calculate_fingerprint(cert);

                if let Some(pinned_fp) = self.pinned_certs.read().get(host) {
                    if &fingerprint != pinned_fp {
                        stats.pinning_violations += 1;
                        return Err(DbError::InvalidOperation("Certificate pinning violation".to_string()));
                    }
                }
            }
        }

        if config.require_client_cert && cert_der.is_none() {
            stats.cert_validation_failures += 1;
            return Err(DbError::InvalidOperation("Client certificate required".to_string()));
        }

        stats.connections_established += 1;
        Ok(())
    }

    fn calculate_fingerprint(cert_der: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().to_vec()
    }

    fn cipher_supports_pfs(cipher: &str) -> bool {
        cipher.contains("ECDHE") || cipher.contains("DHE")
    }

    pub fn get_stats(&self) -> TLSStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// Network Anomaly Detector
// ============================================================================

#[derive(Debug, Clone)]
struct MetricsTimeSeries {
    request_rates: VecDeque<(Instant, f64)>,
    response_times: VecDeque<(Instant, f64)>,
    error_rates: VecDeque<(Instant, f64)>,
    payload_sizes: VecDeque<(Instant, usize)>,
}

impl MetricsTimeSeries {
    fn new() -> Self {
        Self {
            request_rates: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
            response_times: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
            error_rates: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
            payload_sizes: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
        }
    }

    fn add_sample(&mut self, request_rate: f64, response_time: f64, error_rate: f64, payloadsize: usize) {
        let now = Instant::now();

        self.request_rates.push_back((now, request_rate));
        self.response_times.push_back((now, response_time));
        self.error_rates.push_back((now, error_rate));
        self.payload_sizes.push_back((now, payloadsize));

        if self.request_rates.len() > ANOMALY_WINDOW_SIZE {
            self.request_rates.pop_front();
            self.response_times.pop_front();
            self.error_rates.pop_front();
            self.payload_sizes.pop_front();
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct BaselineStats {
    mean_request_rate: f64,
    stddev_request_rate: f64,
    mean_response_time: f64,
    stddev_response_time: f64,
    mean_error_rate: f64,
    stddev_error_rate: f64,
    mean_payload_size: f64,
    stddev_payload_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub timestamp: Instant,
    pub anomaly_type: AnomalyType,
    pub severity: f64,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    RequestRateSpike,
    ResponseTimeSpike,
    ErrorRateSpike,
    PayloadSizeAnomaly,
    MultiMetricAnomaly,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnomalyStats {
    pub total_samples: u64,
    pub anomalies_detected: u64,
    pub request_rate_anomalies: u64,
    pub response_time_anomalies: u64,
    pub error_rate_anomalies: u64,
    pub payload_anomalies: u64,
}

pub struct NetworkAnomalyDetector {
    metrics: Arc<RwLock<MetricsTimeSeries>>,
    baseline: Arc<RwLock<BaselineStats>>,
    anomalies: Arc<RwLock<VecDeque<Anomaly>>>,
    stats: Arc<RwLock<AnomalyStats>>,
}

impl NetworkAnomalyDetector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MetricsTimeSeries::new())),
            baseline: Arc::new(RwLock::new(BaselineStats::default())),
            anomalies: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            stats: Arc::new(RwLock::new(AnomalyStats::default())),
        }
    }

    pub fn add_sample(&self, request_rate: f64, response_time: f64, error_rate: f64, payload_size: usize) {
        let mut metrics = self.metrics.write();
        metrics.add_sample(request_rate, response_time, error_rate, payload_size);

        self.stats.write().total_samples += 1;

        if metrics.request_rates.len() >= ANOMALY_WINDOW_SIZE / 2 {
            drop(metrics);
            self.update_baseline();
        }
    }

    pub fn detect_anomalies(&self) -> Vec<Anomaly> {
        let metrics = self.metrics.read();
        let baseline = self.baseline.read();
        let mut detected = Vec::new();

        if metrics.request_rates.is_empty() {
            return detected;
        }

        let current_rr = metrics.request_rates.back().map(|(_, v)| *v).unwrap_or(0.0);
        let current_rt = metrics.response_times.back().map(|(_, v)| *v).unwrap_or(0.0);
        let current_er = metrics.error_rates.back().map(|(_, v)| *v).unwrap_or(0.0);

        let rr_zscore = if baseline.stddev_request_rate > 0.0 {
            (current_rr - baseline.mean_request_rate) / baseline.stddev_request_rate
        } else {
            0.0
        };

        let rt_zscore = if baseline.stddev_response_time > 0.0 {
            (current_rt - baseline.mean_response_time) / baseline.stddev_response_time
        } else {
            0.0
        };

        let er_zscore = if baseline.stddev_error_rate > 0.0 {
            (current_er - baseline.mean_error_rate) / baseline.stddev_error_rate
        } else {
            0.0
        };

        let mut stats = self.stats.write();

        if rr_zscore.abs() > ANOMALY_Z_SCORE_THRESHOLD {
            detected.push(Anomaly {
                timestamp: Instant::now(),
                anomaly_type: AnomalyType::RequestRateSpike,
                severity: rr_zscore.abs(),
                description: format!("Request rate anomaly: {:.2} req/s (z-score: {:.2})", current_rr, rr_zscore),
            });
            stats.anomalies_detected += 1;
            stats.request_rate_anomalies += 1;
        }

        if rt_zscore.abs() > ANOMALY_Z_SCORE_THRESHOLD {
            detected.push(Anomaly {
                timestamp: Instant::now(),
                anomaly_type: AnomalyType::ResponseTimeSpike,
                severity: rt_zscore.abs(),
                description: format!("Response time anomaly: {:.2} ms (z-score: {:.2})", current_rt, rt_zscore),
            });
            stats.anomalies_detected += 1;
            stats.response_time_anomalies += 1;
        }

        if er_zscore.abs() > ANOMALY_Z_SCORE_THRESHOLD {
            detected.push(Anomaly {
                timestamp: Instant::now(),
                anomaly_type: AnomalyType::ErrorRateSpike,
                severity: er_zscore.abs(),
                description: format!("Error rate anomaly: {:.2}% (z-score: {:.2})", current_er * 100.0, er_zscore),
            });
            stats.anomalies_detected += 1;
            stats.error_rate_anomalies += 1;
        }

        let mut anomalies = self.anomalies.write();
        for anomaly in &detected {
            anomalies.push_back(anomaly.clone());
            if anomalies.len() > 1000 {
                anomalies.pop_front();
            }
        }

        detected
    }

    fn update_baseline(&self) {
        let metrics = self.metrics.read();
        let mut baseline = self.baseline.write();

        if !metrics.request_rates.is_empty() {
            let values: Vec<f64> = metrics.request_rates.iter().map(|(_, v)| *v).collect();
            baseline.mean_request_rate = Self::mean(&values);
            baseline.stddev_request_rate = Self::stddev(&values, baseline.mean_request_rate);
        }

        if !metrics.response_times.is_empty() {
            let values: Vec<f64> = metrics.response_times.iter().map(|(_, v)| *v).collect();
            baseline.mean_response_time = Self::mean(&values);
            baseline.stddev_response_time = Self::stddev(&values, baseline.mean_response_time);
        }

        if !metrics.error_rates.is_empty() {
            let values: Vec<f64> = metrics.error_rates.iter().map(|(_, v)| *v).collect();
            baseline.mean_error_rate = Self::mean(&values);
            baseline.stddev_error_rate = Self::stddev(&values, baseline.mean_error_rate);
        }
    }

    fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<f64>() / values.len() as f64
    }

    fn stddev(values: &[f64], mean: f64) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / (values.len() - 1) as f64;
        variance.sqrt()
    }

    pub fn get_recent_anomalies(&self, count: usize) -> Vec<Anomaly> {
        let anomalies = self.anomalies.read();
        anomalies.iter().rev().take(count).cloned().collect()
    }

    pub fn get_stats(&self) -> AnomalyStats {
        self.stats.read().clone()
    }
}
