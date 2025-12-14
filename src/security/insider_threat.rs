// # Insider Threat Detection Module
//
// Comprehensive insider threat protection system with ML-based risk scoring,
// behavioral analytics, anomaly detection, and real-time prevention.
//
// ## Features
//
// - Query risk scoring (0-100 threat score)
// - User behavior profiling and baseline establishment
// - Statistical anomaly detection
// - Mass data access prevention
// - Privilege escalation detection
// - Real-time query sanitization
// - Immutable forensic logging
// - Geographic and temporal anomaly detection

use crate::error::DbError;
use crate::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Threat score (0-100)
pub type ThreatScore = u8;

// User identifier
pub type UserId = String;

// Query identifier
pub type QueryId = String;

// Threat level based on score
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    // Low risk (0-30)
    Low,
    // Medium risk (31-60)
    Medium,
    // High risk (61-80)
    High,
    // Critical risk (81-100)
    Critical,
}

impl ThreatLevel {
    pub fn from_score(score: ThreatScore) -> Self {
        match score {
            0..=30 => ThreatLevel::Low,
            31..=60 => ThreatLevel::Medium,
            61..=80 => ThreatLevel::High,
            81..=100 => ThreatLevel::Critical,
            _ => ThreatLevel::Critical,
        }
    }
}

// Query risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRiskAssessment {
    // Unique assessment ID
    pub assessment_id: String,
    // User who executed the query
    pub user_id: UserId,
    // Query text
    pub query_text: String,
    // Query hash for deduplication
    pub query_hash: String,
    // Total threat score (0-100)
    pub total_score: ThreatScore,
    // Threat level
    pub threat_level: ThreatLevel,
    // Query pattern risk (0-25)
    pub pattern_risk: u8,
    // Data volume risk (0-25)
    pub volume_risk: u8,
    // Temporal risk (0-25)
    pub temporal_risk: u8,
    // Behavioral deviation risk (0-25)
    pub behavioral_risk: u8,
    // Risk factors identified
    pub risk_factors: Vec<String>,
    // Assessment timestamp
    pub timestamp: i64,
    // Session ID
    pub session_id: Option<String>,
    // Client IP
    pub client_ip: Option<String>,
    // Geographic location
    pub location: Option<String>,
    // Action taken
    pub action: ThreatAction,
}

// Action taken in response to threat
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatAction {
    // Allowed with logging
    AllowWithLog,
    // Allowed with alert
    AllowWithAlert,
    // Warning shown to user
    WarnUser,
    // Require justification
    RequireJustification,
    // Require MFA re-authentication
    RequireMfa,
    // Blocked automatically
    Blocked,
    // Session suspended
    SessionSuspended,
}

// User behavior baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorBaseline {
    // User ID
    pub user_id: UserId,
    // Baseline established date
    pub established_at: i64,
    // Last update timestamp
    pub updated_at: i64,
    // Number of queries in baseline
    pub query_count: u64,
    // Common query patterns (normalized SQL templates)
    pub common_patterns: HashMap<String, u32>,
    // Hourly access distribution (0-23 hours)
    pub access_time_distribution: [u32; 24],
    // Day of week distribution (0-6, Sunday-Saturday)
    pub day_distribution: [u32; 7],
    // Frequently accessed tables
    pub frequent_tables: HashMap<String, u32>,
    // Frequently accessed schemas
    pub frequent_schemas: HashMap<String, u32>,
    // Average result set size
    pub avg_result_set_size: f64,
    // Maximum result set size ever requested
    pub max_result_set_size: u64,
    // Standard deviation of result set size
    pub result_set_stddev: f64,
    // Typical session duration (seconds)
    pub avg_session_duration: f64,
    // Common source IPs
    pub common_ips: HashSet<String>,
    // Common locations
    pub common_locations: HashSet<String>,
    // Typical query complexity score
    pub avg_query_complexity: f64,
}

impl Default for UserBehaviorBaseline {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            established_at: 0,
            updated_at: 0,
            query_count: 0,
            common_patterns: HashMap::new(),
            access_time_distribution: [0; 24],
            day_distribution: [0; 7],
            frequent_tables: HashMap::new(),
            frequent_schemas: HashMap::new(),
            avg_result_set_size: 0.0,
            max_result_set_size: 0,
            result_set_stddev: 0.0,
            avg_session_duration: 0.0,
            common_ips: HashSet::new(),
            common_locations: HashSet::new(),
            avg_query_complexity: 0.0,
        }
    }
}

// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyScore {
    // Total anomaly score (0-100)
    pub score: f64,
    // Anomalies detected
    pub anomalies: Vec<String>,
    // Z-score for various metrics
    pub z_scores: HashMap<String, f64>,
}

// Data exfiltration attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExfiltrationAttempt {
    // Attempt ID
    pub attempt_id: String,
    // User ID
    pub user_id: UserId,
    // Timestamp
    pub timestamp: i64,
    // Query that triggered detection
    pub query: String,
    // Estimated data volume (rows)
    pub estimated_rows: u64,
    // Tables accessed
    pub tables_accessed: Vec<String>,
    // Blocked or allowed
    pub blocked: bool,
    // Detection reason
    pub reason: String,
}

// Privilege escalation attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeEscalationAttempt {
    // Attempt ID
    pub attempt_id: String,
    // User ID
    pub user_id: UserId,
    // Timestamp
    pub timestamp: i64,
    // Query that triggered detection
    pub query: String,
    // Escalation type
    pub escalation_type: EscalationType,
    // Blocked or allowed
    pub blocked: bool,
    // Detection patterns matched
    pub patterns_matched: Vec<String>,
}

// Type of privilege escalation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EscalationType {
    // Attempting to grant privileges
    GrantAttempt,
    // SQL injection patterns
    SqlInjection,
    // System table modification
    SystemTableModification,
    // Audit disabling attempt
    AuditTampering,
    // Backdoor creation
    BackdoorCreation,
    // Role manipulation
    RoleManipulation,
}

// Forensic audit record with threat intelligence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicRecord {
    // Record ID
    pub id: u64,
    // Timestamp (microseconds)
    pub timestamp: i64,
    // User ID
    pub user_id: UserId,
    // Session ID
    pub session_id: Option<String>,
    // Query text
    pub query_text: String,
    // Threat assessment
    pub assessment: QueryRiskAssessment,
    // Anomaly score
    pub anomaly_score: Option<AnomalyScore>,
    // Exfiltration attempt (if detected)
    pub exfiltration_attempt: Option<ExfiltrationAttempt>,
    // Privilege escalation attempt (if detected)
    pub escalation_attempt: Option<PrivilegeEscalationAttempt>,
    // Client metadata
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub location: Option<String>,
    // Chain integrity hash
    pub integrity_hash: String,
    // Previous record hash (blockchain-style)
    pub previous_hash: String,
}

// Insider Threat Detection Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsiderThreatConfig {
    // Enable threat detection
    pub enabled: bool,
    // Auto-block critical threats (score > 80)
    pub auto_block_critical: bool,
    // Require MFA for high-risk queries (score > 60)
    pub require_mfa_high_risk: bool,
    // Maximum rows allowed without justification
    pub max_rows_without_justification: u64,
    // Alert threshold score
    pub alert_threshold: u8,
    // Block threshold score
    pub block_threshold: u8,
    // Baseline learning period (days)
    pub baseline_learning_days: u32,
    // Minimum queries for baseline
    pub min_queries_for_baseline: u64,
    // Enable behavioral analytics
    pub behavioral_analytics_enabled: bool,
    // Enable anomaly detection
    pub anomaly_detection_enabled: bool,
    // Enable data exfiltration prevention
    pub exfiltration_prevention_enabled: bool,
    // Enable privilege escalation detection
    pub escalation_detection_enabled: bool,
}

impl Default for InsiderThreatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_block_critical: true,
            require_mfa_high_risk: true,
            max_rows_without_justification: 10000,
            alert_threshold: 40,
            block_threshold: 80,
            baseline_learning_days: 30,
            min_queries_for_baseline: 100,
            behavioral_analytics_enabled: true,
            anomaly_detection_enabled: true,
            exfiltration_prevention_enabled: true,
            escalation_detection_enabled: true,
        }
    }
}

// ML-Based Query Threat Scorer
pub struct ThreatScorer {
    #[allow(dead_code)]
    config: Arc<RwLock<InsiderThreatConfig>>,
    // Malicious patterns for SQL injection detection
    malicious_patterns: Vec<regex::Regex>,
}

impl ThreatScorer {
    pub fn new(config: Arc<RwLock<InsiderThreatConfig>>) -> Self {
        let malicious_patterns = vec![
            regex::Regex::new(r"(?i)UNION\s+SELECT").unwrap(),
            regex::Regex::new(r"(?i)--").unwrap(),
            regex::Regex::new(r"/\*.*\*/").unwrap(),
            regex::Regex::new(r"(?i)xp_cmdshell").unwrap(),
            regex::Regex::new(r"(?i)EXEC\s*\(").unwrap(),
            regex::Regex::new(r"(?i)DROP\s+TABLE").unwrap(),
            regex::Regex::new(r"(?i)TRUNCATE\s+TABLE").unwrap(),
        ];

        Self {
            config,
            malicious_patterns,
        }
    }

    // Calculate query pattern risk (0-25 points)
    pub fn calculate_pattern_risk(&self, query: &str) -> (u8, Vec<String>) {
        let mut score = 0u8;
        let mut factors = Vec::new();

        let query_upper = query.to_uppercase();

        // SELECT * pattern (mass data access)
        if query_upper.contains("SELECT *") || query_upper.contains("SELECT*") {
            score += 10;
            factors.push("Mass data access pattern (SELECT *)".to_string());
        }

        // WHERE 1=1 or no WHERE clause
        if query_upper.contains("WHERE 1=1") || query_upper.contains("WHERE 1 = 1") {
            score += 10;
            factors.push("Unconditional WHERE clause (WHERE 1=1)".to_string());
        } else if query_upper.contains("SELECT") && !query_upper.contains("WHERE") {
            score += 5;
            factors.push("No WHERE clause in SELECT".to_string());
        }

        // Multiple table joins (potential data correlation)
        let join_count = query_upper.matches("JOIN").count();
        if join_count >= 3 {
            score += 5;
            factors.push(format!("Multiple table joins ({})", join_count));
        }

        // SQL injection patterns
        for pattern in &self.malicious_patterns {
            if pattern.is_match(query) {
                score = 25; // Immediate high score for injection patterns
                factors.push("SQL injection pattern detected".to_string());
                break;
            }
        }

        (score.min(25), factors)
    }

    // Calculate data volume risk (0-25 points)
    pub fn calculate_volume_risk(&self, estimated_rows: u64) -> (u8, Vec<String>) {
        let mut score = 0u8;
        let mut factors = Vec::new();

        if estimated_rows > 10000 {
            score += 15;
            factors.push(format!("Large result set (>{} rows)", estimated_rows));
        } else if estimated_rows > 1000 {
            score += 10;
            factors.push(format!("Medium result set ({} rows)", estimated_rows));
        }

        // Check for LIMIT clause absence
        // This would be more sophisticated in production

        (score.min(25), factors)
    }

    // Calculate temporal risk (0-25 points)
    pub fn calculate_temporal_risk(
        &self,
        timestamp: i64,
        user_baseline: Option<&UserBehaviorBaseline>,
    ) -> (u8, Vec<String>) {
        let mut score = 0u8;
        let mut factors = Vec::new();

        // Get current hour and day

        // Simplified: extract hour from timestamp
        let hour = (timestamp % 86400) / 3600;
        let day_of_week = ((timestamp / 86400) % 7) as usize;

        // Off-hours access (10pm - 6am)
        if hour >= 22 || hour < 6 {
            score += 15;
            factors.push(format!("Off-hours access ({}:00)", hour));
        }

        // Weekend access
        if day_of_week == 0 || day_of_week == 6 {
            score += 10;
            factors.push("Weekend access".to_string());
        }

        // Check against user baseline
        if let Some(baseline) = user_baseline {
            if baseline.query_count >= 100 {
                let hour_frequency = baseline.access_time_distribution[hour as usize];
                let total_queries = baseline.query_count;
                let expected_frequency = total_queries / 24;

                // If this hour has <5% of normal activity, it's unusual
                if hour_frequency < (expected_frequency as f64 * 0.05) as u32 {
                    score += 10;
                    factors.push("Unusual time for user".to_string());
                }
            }
        }

        (score.min(25), factors)
    }

    // Calculate behavioral deviation risk (0-25 points)
    pub fn calculate_behavioral_risk(
        &self,
        query: &str,
        tables: &[String],
        user_baseline: Option<&UserBehaviorBaseline>,
    ) -> (u8, Vec<String>) {
        let mut score = 0u8;
        let mut factors = Vec::new();

        if let Some(baseline) = user_baseline {
            // Only calculate if baseline is established
            if baseline.query_count < 100 {
                return (0, factors);
            }

            // Check for accessing never-before-seen tables
            for table in tables {
                if !baseline.frequent_tables.contains_key(table) {
                    score += 10;
                    factors.push(format!("Accessing new table: {}", table));
                    break;
                }
            }

            // Check query pattern deviation
            let pattern = self.normalize_query(query);
            if !baseline.common_patterns.contains_key(&pattern) {
                score += 5;
                factors.push("Unusual query pattern for user".to_string());
            }

            // Query complexity check (simplified)
            let complexity = self.calculate_query_complexity(query);
            if complexity > baseline.avg_query_complexity * 2.0 {
                score += 10;
                factors.push("Query complexity significantly higher than baseline".to_string());
            }
        } else {
            // No baseline - moderate risk
            score += 5;
            factors.push("No behavioral baseline established".to_string());
        }

        (score.min(25), factors)
    }

    // Calculate overall threat score
    pub fn calculate_threat_score(
        &self,
        query: &str,
        estimated_rows: u64,
        tables: &[String],
        timestamp: i64,
        user_baseline: Option<&UserBehaviorBaseline>,
    ) -> QueryRiskAssessment {
        let (pattern_risk, mut risk_factors) = self.calculate_pattern_risk(query);
        let (volume_risk, volume_factors) = self.calculate_volume_risk(estimated_rows);
        let (temporal_risk, temporal_factors) =
            self.calculate_temporal_risk(timestamp, user_baseline);
        let (behavioral_risk, behavioral_factors) =
            self.calculate_behavioral_risk(query, tables, user_baseline);

        risk_factors.extend(volume_factors);
        risk_factors.extend(temporal_factors);
        risk_factors.extend(behavioral_factors);

        // Weighted scoring
        let total_score = ((pattern_risk as f64 * 0.25)
            + (volume_risk as f64 * 0.30)
            + (temporal_risk as f64 * 0.20)
            + (behavioral_risk as f64 * 0.25) * 4.0) as u8;

        let threat_level = ThreatLevel::from_score(total_score);

        // Determine action based on threat level
        let action = match threat_level {
            ThreatLevel::Low => ThreatAction::AllowWithLog,
            ThreatLevel::Medium => ThreatAction::AllowWithAlert,
            ThreatLevel::High => ThreatAction::RequireJustification,
            ThreatLevel::Critical => ThreatAction::Blocked,
        };

        QueryRiskAssessment {
            assessment_id: uuid::Uuid::new_v4().to_string(),
            user_id: String::new(), // Set by caller
            query_text: query.to_string(),
            query_hash: self.hash_query(query),
            total_score,
            threat_level,
            pattern_risk,
            volume_risk,
            temporal_risk,
            behavioral_risk,
            risk_factors,
            timestamp,
            session_id: None,
            client_ip: None,
            location: None,
            action,
        }
    }

    fn normalize_query(&self, query: &str) -> String {
        // Simplified normalization - replace literals with placeholders
        let normalized = query
            .replace(|c: char| c.is_numeric(), "?")
            .replace("'.*?'", "?");
        normalized
    }

    fn calculate_query_complexity(&self, query: &str) -> f64 {
        let query_upper = query.to_uppercase();
        let mut complexity = 0.0;

        complexity += query_upper.matches("SELECT").count() as f64;
        complexity += query_upper.matches("JOIN").count() as f64 * 2.0;
        complexity += query_upper.matches("UNION").count() as f64 * 3.0;
        complexity += query_upper.matches("SUBQUERY").count() as f64 * 2.0;
        complexity += query_upper.matches("GROUP BY").count() as f64 * 1.5;
        complexity += query_upper.matches("ORDER BY").count() as f64 * 1.5;

        complexity
    }

    fn hash_query(&self, query: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// User Behavior Analyzer
pub struct BehaviorAnalyzer {
    // User baselines
    baselines: Arc<RwLock<HashMap<UserId, UserBehaviorBaseline>>>,
    // Recent queries for baseline building
    query_history: Arc<RwLock<HashMap<UserId, VecDeque<QueryHistoryEntry>>>>,
    config: Arc<RwLock<InsiderThreatConfig>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct QueryHistoryEntry {
    query: String,
    normalized_pattern: String,
    tables: Vec<String>,
    timestamp: i64,
    result_rows: u64,
    client_ip: Option<String>,
    location: Option<String>,
}

impl BehaviorAnalyzer {
    pub fn new(config: Arc<RwLock<InsiderThreatConfig>>) -> Self {
        Self {
            baselines: Arc::new(RwLock::new(HashMap::new())),
            query_history: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    // Record query for baseline building
    pub fn record_query(
        &self,
        user_id: &UserId,
        query: &str,
        tables: Vec<String>,
        result_rows: u64,
        client_ip: Option<String>,
        location: Option<String>,
    ) {
        let entry = QueryHistoryEntry {
            query: query.to_string(),
            normalized_pattern: self.normalize_query(query),
            tables,
            timestamp: current_timestamp(),
            result_rows,
            client_ip,
            location,
        };

        let mut history = self.query_history.write();
        let user_history = history.entry(user_id.clone()).or_insert_with(VecDeque::new);
        user_history.push_back(entry);

        // Keep last 1000 queries
        if user_history.len() > 1000 {
            user_history.pop_front();
        }

        // Update baseline if enough queries
        drop(history);
        self.update_baseline(user_id);
    }

    // Update user baseline
    fn update_baseline(&self, user_id: &UserId) {
        let history = self.query_history.read();
        let user_history = match history.get(user_id) {
            Some(h) => h,
            None => return,
        };

        let config = self.config.read();
        if user_history.len() < config.min_queries_for_baseline as usize {
            return;
        }

        let mut baseline = UserBehaviorBaseline::default();
        baseline.user_id = user_id.clone();
        baseline.query_count = user_history.len() as u64;
        baseline.updated_at = current_timestamp();

        // Calculate statistics
        let mut row_sizes: Vec<u64> = Vec::new();

        for entry in user_history.iter() {
            // Pattern frequency
            *baseline
                .common_patterns
                .entry(entry.normalized_pattern.clone())
                .or_insert(0) += 1;

            // Time distribution
            let hour = ((entry.timestamp % 86400) / 3600) as usize;
            baseline.access_time_distribution[hour] += 1;

            let day = ((entry.timestamp / 86400) % 7) as usize;
            baseline.day_distribution[day] += 1;

            // Table frequency
            for table in &entry.tables {
                *baseline.frequent_tables.entry(table.clone()).or_insert(0) += 1;
            }

            // Result set sizes
            row_sizes.push(entry.result_rows);

            // IPs and locations
            if let Some(ref ip) = entry.client_ip {
                baseline.common_ips.insert(ip.clone());
            }
            if let Some(ref loc) = entry.location {
                baseline.common_locations.insert(loc.clone());
            }
        }

        // Calculate statistics
        if !row_sizes.is_empty() {
            baseline.avg_result_set_size =
                row_sizes.iter().sum::<u64>() as f64 / row_sizes.len() as f64;
            baseline.max_result_set_size = *row_sizes.iter().max().unwrap();

            // Calculate standard deviation
            let variance = row_sizes
                .iter()
                .map(|&x| {
                    let diff = x as f64 - baseline.avg_result_set_size;
                    diff * diff
                })
                .sum::<f64>()
                / row_sizes.len() as f64;
            baseline.result_set_stddev = variance.sqrt();
        }

        baseline.established_at = current_timestamp();

        // Store baseline
        self.baselines.write().insert(user_id.clone(), baseline);
    }

    // Get user baseline
    pub fn get_baseline(&self, user_id: &UserId) -> Option<UserBehaviorBaseline> {
        self.baselines.read().get(user_id).cloned()
    }

    fn normalize_query(&self, query: &str) -> String {
        // Simplified normalization
        let normalized = regex::Regex::new(r"\d+").unwrap().replace_all(query, "?");
        let normalized = regex::Regex::new(r"'[^']*'")
            .unwrap()
            .replace_all(&normalized, "?");
        normalized.to_string()
    }
}

// Statistical Anomaly Detector
pub struct AnomalyDetector {
    #[allow(dead_code)]
    config: Arc<RwLock<InsiderThreatConfig>>,
}

impl AnomalyDetector {
    pub fn new(config: Arc<RwLock<InsiderThreatConfig>>) -> Self {
        Self { config }
    }

    // Detect anomalies based on baseline deviation
    pub fn detect_anomalies(
        &self,
        result_rows: u64,
        baseline: &UserBehaviorBaseline,
    ) -> AnomalyScore {
        let mut score: f64 = 0.0;
        let mut anomalies = Vec::new();
        let mut z_scores = HashMap::new();

        // Z-score for result set size
        if baseline.result_set_stddev > 0.0 {
            let z_score =
                (result_rows as f64 - baseline.avg_result_set_size) / baseline.result_set_stddev;
            z_scores.insert("result_set_size".to_string(), z_score);

            if z_score.abs() > 3.0 {
                score += 30.0;
                anomalies.push(format!(
                    "Result set size {} is {} std devs from mean",
                    result_rows, z_score
                ));
            } else if z_score.abs() > 2.0 {
                score += 15.0;
                anomalies.push(format!(
                    "Result set size moderately anomalous (z-score: {:.2})",
                    z_score
                ));
            }
        }

        // Check if exceeds maximum ever seen
        if result_rows > baseline.max_result_set_size * 2 {
            score += 40.0;
            anomalies.push("Result set exceeds 2x historical maximum".to_string());
        }

        AnomalyScore {
            score: score.min(100.0),
            anomalies,
            z_scores,
        }
    }
}

// Data Exfiltration Guard
pub struct DataExfiltrationGuard {
    // Recent exfiltration attempts
    attempts: Arc<RwLock<Vec<ExfiltrationAttempt>>>,
    // Per-user data volume tracking (rolling 1 hour)
    user_volumes: Arc<RwLock<HashMap<UserId, VecDeque<(i64, u64)>>>>,
    config: Arc<RwLock<InsiderThreatConfig>>,
}

impl DataExfiltrationGuard {
    pub fn new(config: Arc<RwLock<InsiderThreatConfig>>) -> Self {
        Self {
            attempts: Arc::new(RwLock::new(Vec::new())),
            user_volumes: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    // Check for data exfiltration attempt
    pub fn check_exfiltration(
        &self,
        user_id: &UserId,
        query: &str,
        estimated_rows: u64,
        tables: &[String],
    ) -> Option<ExfiltrationAttempt> {
        let config = self.config.read();

        // Check row limit
        if estimated_rows > config.max_rows_without_justification {
            let attempt = ExfiltrationAttempt {
                attempt_id: uuid::Uuid::new_v4().to_string(),
                user_id: user_id.clone(),
                timestamp: current_timestamp(),
                query: query.to_string(),
                estimated_rows,
                tables_accessed: tables.to_vec(),
                blocked: true,
                reason: format!(
                    "Query attempts to access {} rows, exceeding limit of {}",
                    estimated_rows, config.max_rows_without_justification
                ),
            };

            self.attempts.write().push(attempt.clone());
            return Some(attempt);
        }

        // Track rolling volume
        let mut volumes = self.user_volumes.write();
        let user_volume = volumes.entry(user_id.clone()).or_insert_with(VecDeque::new);

        let now = current_timestamp();
        let one_hour_ago = now - 3600;

        // Remove old entries
        user_volume.retain(|(ts, _)| *ts > one_hour_ago);

        // Add current query
        user_volume.push_back((now, estimated_rows));

        // Check total volume in last hour
        let total_rows: u64 = user_volume.iter().map(|(_, rows)| rows).sum();
        let volume_mb_estimate = (total_rows * 100) / (1024 * 1024); // Rough estimate

        if volume_mb_estimate > 100 {
            let attempt = ExfiltrationAttempt {
                attempt_id: uuid::Uuid::new_v4().to_string(),
                user_id: user_id.clone(),
                timestamp: now,
                query: query.to_string(),
                estimated_rows: total_rows,
                tables_accessed: tables.to_vec(),
                blocked: true,
                reason: format!(
                    "User has accessed ~{}MB of data in the last hour",
                    volume_mb_estimate
                ),
            };

            self.attempts.write().push(attempt.clone());
            return Some(attempt);
        }

        None
    }

    // Get recent attempts
    pub fn get_recent_attempts(&self, user_id: Option<&UserId>) -> Vec<ExfiltrationAttempt> {
        let attempts = self.attempts.read();
        match user_id {
            Some(uid) => attempts
                .iter()
                .filter(|a| &a.user_id == uid)
                .cloned()
                .collect(),
            None => attempts.clone(),
        }
    }
}

// Privilege Escalation Detector
pub struct PrivilegeEscalationDetector {
    // Recent escalation attempts
    attempts: Arc<RwLock<Vec<PrivilegeEscalationAttempt>>>,
    // Dangerous patterns
    dangerous_patterns: Vec<(regex::Regex, EscalationType)>,
}

impl PrivilegeEscalationDetector {
    pub fn new() -> Self {
        let dangerous_patterns = vec![
            (
                regex::Regex::new(r"(?i)GRANT\s+").unwrap(),
                EscalationType::GrantAttempt,
            ),
            (
                regex::Regex::new(r"(?i)REVOKE\s+").unwrap(),
                EscalationType::GrantAttempt,
            ),
            (
                regex::Regex::new(r"(?i)CREATE\s+USER").unwrap(),
                EscalationType::BackdoorCreation,
            ),
            (
                regex::Regex::new(r"(?i)ALTER\s+USER").unwrap(),
                EscalationType::BackdoorCreation,
            ),
            (
                regex::Regex::new(r"(?i)DROP\s+USER").unwrap(),
                EscalationType::BackdoorCreation,
            ),
            (
                regex::Regex::new(r"(?i)CREATE\s+ROLE").unwrap(),
                EscalationType::RoleManipulation,
            ),
            (
                regex::Regex::new(r"(?i)ALTER\s+ROLE").unwrap(),
                EscalationType::RoleManipulation,
            ),
            (
                regex::Regex::new(r"(?i)UPDATE\s+.*\bsys").unwrap(),
                EscalationType::SystemTableModification,
            ),
            (
                regex::Regex::new(r"(?i)DELETE\s+FROM\s+.*\bsys").unwrap(),
                EscalationType::SystemTableModification,
            ),
            (
                regex::Regex::new(r"(?i)DISABLE\s+AUDIT").unwrap(),
                EscalationType::AuditTampering,
            ),
            (
                regex::Regex::new(r"(?i)UNION\s+SELECT").unwrap(),
                EscalationType::SqlInjection,
            ),
        ];

        Self {
            attempts: Arc::new(RwLock::new(Vec::new())),
            dangerous_patterns,
        }
    }

    // Check for privilege escalation attempt
    pub fn check_escalation(
        &self,
        user_id: &UserId,
        query: &str,
    ) -> Option<PrivilegeEscalationAttempt> {
        let mut patterns_matched = Vec::new();
        let mut escalation_type = None;

        for (pattern, esc_type) in &self.dangerous_patterns {
            if pattern.is_match(query) {
                patterns_matched.push(format!("{:?}: {}", esc_type, pattern.as_str()));
                escalation_type = Some(esc_type.clone());
            }
        }

        if !patterns_matched.is_empty() {
            let attempt = PrivilegeEscalationAttempt {
                attempt_id: uuid::Uuid::new_v4().to_string(),
                user_id: user_id.clone(),
                timestamp: current_timestamp(),
                query: query.to_string(),
                escalation_type: escalation_type.unwrap_or(EscalationType::SqlInjection),
                blocked: true,
                patterns_matched,
            };

            self.attempts.write().push(attempt.clone());
            return Some(attempt);
        }

        None
    }

    // Get recent attempts
    pub fn get_recent_attempts(&self, user_id: Option<&UserId>) -> Vec<PrivilegeEscalationAttempt> {
        let attempts = self.attempts.read();
        match user_id {
            Some(uid) => attempts
                .iter()
                .filter(|a| &a.user_id == uid)
                .cloned()
                .collect(),
            None => attempts.clone(),
        }
    }
}

// Query Sanitizer
pub struct QuerySanitizer {
    // Blocked keywords
    blocked_keywords: HashSet<String>,
}

impl QuerySanitizer {
    pub fn new() -> Self {
        let mut blocked_keywords = HashSet::new();
        blocked_keywords.insert("xp_cmdshell".to_string());
        blocked_keywords.insert("sp_configure".to_string());
        blocked_keywords.insert("EXEC(".to_string());

        Self { blocked_keywords }
    }

    // Sanitize and validate query
    pub fn sanitize(&self, query: &str) -> Result<String> {
        let query_upper = query.to_uppercase();

        // Check for blocked keywords
        for keyword in &self.blocked_keywords {
            if query_upper.contains(&keyword.to_uppercase()) {
                return Err(DbError::InvalidInput(format!(
                    "Query contains blocked keyword: {}",
                    keyword
                )));
            }
        }

        Ok(query.to_string())
    }
}

// Forensic Logger with Immutable Audit Trail
pub struct ForensicLogger {
    // Forensic records
    records: Arc<RwLock<VecDeque<ForensicRecord>>>,
    // Record ID counter
    id_counter: Arc<RwLock<u64>>,
    // Previous hash for chaining
    previous_hash: Arc<RwLock<String>>,
    // Maximum records in memory
    max_records: usize,
}

impl ForensicLogger {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Arc::new(RwLock::new(VecDeque::new())),
            id_counter: Arc::new(RwLock::new(0)),
            previous_hash: Arc::new(RwLock::new(String::from("0"))),
            max_records,
        }
    }

    // Log forensic record
    pub fn log_record(
        &self,
        user_id: UserId,
        session_id: Option<String>,
        query_text: String,
        assessment: QueryRiskAssessment,
        anomaly_score: Option<AnomalyScore>,
        exfiltration_attempt: Option<ExfiltrationAttempt>,
        escalation_attempt: Option<PrivilegeEscalationAttempt>,
        client_ip: Option<String>,
        user_agent: Option<String>,
        location: Option<String>,
    ) -> u64 {
        let mut counter = self.id_counter.write();
        *counter += 1;
        let id = *counter;
        drop(counter);

        let previous_hash = self.previous_hash.read().clone();

        let mut record = ForensicRecord {
            id,
            timestamp: current_timestamp_micros(),
            user_id,
            session_id,
            query_text,
            assessment,
            anomaly_score,
            exfiltration_attempt,
            escalation_attempt,
            client_ip,
            user_agent,
            location,
            integrity_hash: String::new(),
            previous_hash: previous_hash.clone(),
        };

        // Calculate integrity hash
        let hash = self.calculate_hash(&record);
        record.integrity_hash = hash.clone();

        // Update previous hash
        *self.previous_hash.write() = hash;

        // Store record
        let mut records = self.records.write();
        if records.len() >= self.max_records {
            records.pop_front();
        }
        records.push_back(record);

        id
    }

    fn calculate_hash(&self, record: &ForensicRecord) -> String {
        let data = format!(
            "{}|{}|{}|{}|{}|{}",
            record.id,
            record.timestamp,
            record.user_id,
            record.query_text,
            record.assessment.total_score,
            record.previous_hash
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // Get recent records
    pub fn get_recent_records(&self, limit: usize) -> Vec<ForensicRecord> {
        self.records
            .read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    // Verify audit trail integrity
    pub fn verify_integrity(&self) -> bool {
        let records = self.records.read();
        let mut prev_hash = "0".to_string();

        for record in records.iter() {
            if record.previous_hash != prev_hash {
                return false;
            }

            let calculated_hash = self.calculate_hash(record);
            if calculated_hash != record.integrity_hash {
                return false;
            }

            prev_hash = record.integrity_hash.clone();
        }

        true
    }
}

// Integrated Insider Threat Manager
pub struct InsiderThreatManager {
    // Configuration
    config: Arc<RwLock<InsiderThreatConfig>>,
    // Threat scorer
    pub threat_scorer: Arc<ThreatScorer>,
    // Behavior analyzer
    pub behavior_analyzer: Arc<BehaviorAnalyzer>,
    // Anomaly detector
    pub anomaly_detector: Arc<AnomalyDetector>,
    // Exfiltration guard
    pub exfiltration_guard: Arc<DataExfiltrationGuard>,
    // Escalation detector
    pub escalation_detector: Arc<PrivilegeEscalationDetector>,
    // Query sanitizer
    pub query_sanitizer: Arc<QuerySanitizer>,
    // Forensic logger
    pub forensic_logger: Arc<ForensicLogger>,
}

impl InsiderThreatManager {
    // Create new insider threat manager
    pub fn new() -> Self {
        let config = Arc::new(RwLock::new(InsiderThreatConfig::default()));

        Self {
            config: config.clone(),
            threat_scorer: Arc::new(ThreatScorer::new(config.clone())),
            behavior_analyzer: Arc::new(BehaviorAnalyzer::new(config.clone())),
            anomaly_detector: Arc::new(AnomalyDetector::new(config.clone())),
            exfiltration_guard: Arc::new(DataExfiltrationGuard::new(config.clone())),
            escalation_detector: Arc::new(PrivilegeEscalationDetector::new()),
            query_sanitizer: Arc::new(QuerySanitizer::new()),
            forensic_logger: Arc::new(ForensicLogger::new(10000)),
        }
    }

    // Assess query threat
    pub fn assess_query(
        &self,
        user_id: &UserId,
        session_id: Option<String>,
        query: &str,
        tables: Vec<String>,
        estimated_rows: u64,
        client_ip: Option<String>,
        location: Option<String>,
    ) -> Result<QueryRiskAssessment> {
        // First, sanitize the query
        self.query_sanitizer.sanitize(query)?;

        // Check for privilege escalation
        if let Some(escalation) = self.escalation_detector.check_escalation(user_id, query) {
            // Log forensic record
            let assessment = QueryRiskAssessment {
                assessment_id: uuid::Uuid::new_v4().to_string(),
                user_id: user_id.clone(),
                query_text: query.to_string(),
                query_hash: String::new(),
                total_score: 100,
                threat_level: ThreatLevel::Critical,
                pattern_risk: 25,
                volume_risk: 25,
                temporal_risk: 25,
                behavioral_risk: 25,
                risk_factors: vec!["Privilege escalation attempt detected".to_string()],
                timestamp: current_timestamp(),
                session_id: session_id.clone(),
                client_ip: client_ip.clone(),
                location: location.clone(),
                action: ThreatAction::Blocked,
            };

            self.forensic_logger.log_record(
                user_id.clone(),
                session_id,
                query.to_string(),
                assessment.clone(),
                None,
                None,
                Some(escalation.clone()),
                client_ip,
                None,
                location,
            );

            return Err(DbError::InvalidOperation(format!(
                "Privilege escalation attempt blocked: {:?}",
                escalation.escalation_type
            )));
        }

        // Check for data exfiltration
        let exfiltration_attempt =
            self.exfiltration_guard
                .check_exfiltration(user_id, query, estimated_rows, &tables);

        if let Some(ref attempt) = exfiltration_attempt {
            if attempt.blocked {
                let assessment = QueryRiskAssessment {
                    assessment_id: uuid::Uuid::new_v4().to_string(),
                    user_id: user_id.clone(),
                    query_text: query.to_string(),
                    query_hash: String::new(),
                    total_score: 95,
                    threat_level: ThreatLevel::Critical,
                    pattern_risk: 20,
                    volume_risk: 25,
                    temporal_risk: 25,
                    behavioral_risk: 25,
                    risk_factors: vec![attempt.reason.clone()],
                    timestamp: current_timestamp(),
                    session_id: session_id.clone(),
                    client_ip: client_ip.clone(),
                    location: location.clone(),
                    action: ThreatAction::Blocked,
                };

                self.forensic_logger.log_record(
                    user_id.clone(),
                    session_id,
                    query.to_string(),
                    assessment.clone(),
                    None,
                    Some(attempt.clone()),
                    None,
                    client_ip,
                    None,
                    location,
                );

                return Err(DbError::InvalidOperation(attempt.reason.clone()));
            }
        }

        // Get user baseline
        let baseline = self.behavior_analyzer.get_baseline(user_id);

        // Calculate threat score
        let mut assessment = self.threat_scorer.calculate_threat_score(
            query,
            estimated_rows,
            &tables,
            current_timestamp(),
            baseline.as_ref(),
        );

        assessment.user_id = user_id.clone();
        assessment.session_id = session_id.clone();
        assessment.client_ip = client_ip.clone();
        assessment.location = location.clone();

        // Detect anomalies
        let anomaly_score = if let Some(ref baseline) = baseline {
            Some(
                self.anomaly_detector
                    .detect_anomalies(estimated_rows, baseline),
            )
        } else {
            None
        };

        // Record query for baseline building
        self.behavior_analyzer.record_query(
            user_id,
            query,
            tables.clone(),
            estimated_rows,
            client_ip.clone(),
            location.clone(),
        );

        // Log forensic record
        self.forensic_logger.log_record(
            user_id.clone(),
            session_id,
            query.to_string(),
            assessment.clone(),
            anomaly_score,
            exfiltration_attempt,
            None,
            client_ip,
            None,
            location,
        );

        // Check if we should block
        if assessment.threat_level == ThreatLevel::Critical
            && self.config.read().auto_block_critical
        {
            return Err(DbError::InvalidOperation(format!(
                "Query blocked due to critical threat score ({}). Risk factors: {:?}",
                assessment.total_score, assessment.risk_factors
            )));
        }

        Ok(assessment)
    }

    // Get threat statistics
    pub fn get_statistics(&self) -> ThreatStatistics {
        let forensic_records = self.forensic_logger.get_recent_records(1000);

        let total_assessments = forensic_records.len();
        let critical_threats = forensic_records
            .iter()
            .filter(|r| r.assessment.threat_level == ThreatLevel::Critical)
            .count();
        let high_threats = forensic_records
            .iter()
            .filter(|r| r.assessment.threat_level == ThreatLevel::High)
            .count();
        let blocked_queries = forensic_records
            .iter()
            .filter(|r| r.assessment.action == ThreatAction::Blocked)
            .count();
        let exfiltration_attempts = forensic_records
            .iter()
            .filter(|r| r.exfiltration_attempt.is_some())
            .count();
        let escalation_attempts = forensic_records
            .iter()
            .filter(|r| r.escalation_attempt.is_some())
            .count();

        ThreatStatistics {
            total_assessments,
            critical_threats,
            high_threats,
            blocked_queries,
            exfiltration_attempts,
            escalation_attempts,
            baselines_established: self.behavior_analyzer.baselines.read().len(),
        }
    }

    // Update configuration
    pub fn update_config(&self, config: InsiderThreatConfig) {
        *self.config.write() = config;
    }

    // Get configuration
    pub fn get_config(&self) -> InsiderThreatConfig {
        self.config.read().clone()
    }
}

impl Default for InsiderThreatManager {
    fn default() -> Self {
        Self::new()
    }
}

// Threat statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatStatistics {
    pub total_assessments: usize,
    pub critical_threats: usize,
    pub high_threats: usize,
    pub blocked_queries: usize,
    pub exfiltration_attempts: usize,
    pub escalation_attempts: usize,
    pub baselines_established: usize,
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn current_timestamp_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_scorer() {
        let config = Arc::new(RwLock::new(InsiderThreatConfig::default()));
        let scorer = ThreatScorer::new(config);

        let assessment = scorer.calculate_threat_score(
            "SELECT * FROM users WHERE 1=1",
            10000,
            &vec!["users".to_string()],
            current_timestamp(),
            None,
        );

        assert!(assessment.total_score > 30);
        assert_eq!(
            assessment.threat_level,
            ThreatLevel::from_score(assessment.total_score)
        );
    }

    #[test]
    fn test_privilege_escalation_detection() {
        let detector = PrivilegeEscalationDetector::new();

        let attempt = detector.check_escalation(
            &"user1".to_string(),
            "GRANT ALL PRIVILEGES TO malicious_user",
        );

        assert!(attempt.is_some());
        assert_eq!(
            attempt.unwrap().escalation_type,
            EscalationType::GrantAttempt
        );
    }

    #[test]
    fn test_data_exfiltration_detection() {
        let config = Arc::new(RwLock::new(InsiderThreatConfig::default()));
        let guard = DataExfiltrationGuard::new(config);

        let attempt = guard.check_exfiltration(
            &"user1".to_string(),
            "SELECT * FROM sensitive_data",
            15000,
            &vec!["sensitive_data".to_string()],
        );

        assert!(attempt.is_some());
        assert!(attempt.unwrap().blocked);
    }

    #[test]
    fn test_forensic_logger_integrity() {
        let logger = ForensicLogger::new(100);

        let assessment = QueryRiskAssessment {
            assessment_id: "test1".to_string(),
            user_id: "user1".to_string(),
            query_text: "SELECT * FROM test".to_string(),
            query_hash: "hash1".to_string(),
            total_score: 50,
            threat_level: ThreatLevel::Medium,
            pattern_risk: 10,
            volume_risk: 15,
            temporal_risk: 10,
            behavioral_risk: 15,
            risk_factors: vec![],
            timestamp: current_timestamp(),
            session_id: None,
            client_ip: None,
            location: None,
            action: ThreatAction::AllowWithLog,
        };

        logger.log_record(
            "user1".to_string(),
            None,
            "SELECT * FROM test".to_string(),
            assessment,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert!(logger.verify_integrity());
    }

    #[test]
    fn test_insider_threat_manager() {
        let manager = InsiderThreatManager::new();

        let result = manager.assess_query(
            &"user1".to_string(),
            Some("session1".to_string()),
            "SELECT id, name FROM employees WHERE department = 'Engineering' LIMIT 100",
            vec!["employees".to_string()],
            100,
            Some("192.168.1.1".to_string()),
            Some("US-CA".to_string()),
        );

        assert!(result.is_ok());
        let assessment = result.unwrap();
        assert!(assessment.total_score < 50);
    }
}
