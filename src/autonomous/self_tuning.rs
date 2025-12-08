//! Self-Tuning Database Engine
//!
//! Provides autonomous parameter tuning and optimization using ML-driven approaches.
//! This module implements continuous optimization loops with reinforcement learning
//! concepts to automatically adjust database parameters for optimal performance.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::sleep;
use crate::Result;
use crate::error::DbError;

/// Aggressiveness level for autonomous tuning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggressivenessLevel {
    /// Conservative - only make safe, well-tested changes
    Conservative,
    /// Moderate - balance between safety and performance
    Moderate,
    /// Aggressive - prioritize performance over caution
    Aggressive,
}

/// Database parameter that can be tuned
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TunableParameter {
    BufferPoolSize,
    SortAreaSize,
    WorkerThreads,
    MaxConnections,
    CheckpointInterval,
    WalBufferSize,
    SharedBuffers,
    EffectiveCacheSize,
    RandomPageCost,
    SeqPageCost,
    DefaultStatisticsTarget,
    MaintenanceWorkMem,
}

/// Parameter value type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Duration(Duration),
}

impl ParameterValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ParameterValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ParameterValue::Float(v) => Some(*v),
            ParameterValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }
}

/// Parameter configuration with constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConfig {
    pub parameter: TunableParameter,
    pub current_value: ParameterValue,
    pub min_value: ParameterValue,
    pub max_value: ParameterValue,
    pub step_size: ParameterValue,
    pub requires_restart: bool,
    pub last_changed: SystemTime,
}

/// Performance metrics for evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub queries_per_second: f64,
    pub avg_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub buffer_hit_rate: f64,
    pub cpu_usage: f64,
    pub memory_usage_mb: usize,
    pub disk_io_rate: f64,
    pub cache_hit_rate: f64,
    pub lock_wait_time_ms: f64,
    pub transaction_throughput: f64,
    pub timestamp: SystemTime,
}

impl PerformanceMetrics {
    /// Calculate overall performance score (0.0 to 1.0)
    pub fn calculate_score(&self) -> f64 {
        let mut score = 0.0;
        let mut weight_sum = 0.0;

        // Throughput (weight: 0.3)
        if self.queries_per_second > 0.0 {
            score += (self.queries_per_second.min(1000.0) / 1000.0) * 0.3;
            weight_sum += 0.3;
        }

        // Response time (weight: 0.25, inverted)
        if self.avg_response_time_ms > 0.0 {
            let normalized = (1000.0 - self.avg_response_time_ms.min(1000.0)) / 1000.0;
            score += normalized * 0.25;
            weight_sum += 0.25;
        }

        // Buffer hit rate (weight: 0.2)
        score += self.buffer_hit_rate * 0.2;
        weight_sum += 0.2;

        // Cache hit rate (weight: 0.15)
        score += self.cache_hit_rate * 0.15;
        weight_sum += 0.15;

        // Lock wait time (weight: 0.1, inverted)
        if self.lock_wait_time_ms >= 0.0 {
            let normalized = (100.0 - self.lock_wait_time_ms.min(100.0)) / 100.0;
            score += normalized * 0.1;
            weight_sum += 0.1;
        }

        if weight_sum > 0.0 {
            score / weight_sum
        } else {
            0.0
        }
    }
}

/// Tuning action taken by the auto-tuner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningAction {
    pub parameter: TunableParameter,
    pub old_value: ParameterValue,
    pub new_value: ParameterValue,
    pub reason: String,
    pub timestamp: SystemTime,
    pub expected_improvement: f64,
}

/// Tuning result after applying an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningResult {
    pub action: TuningAction,
    pub before_metrics: PerformanceMetrics,
    pub after_metrics: PerformanceMetrics,
    pub actual_improvement: f64,
    pub rollback_performed: bool,
    pub timestamp: SystemTime,
}

/// Workload characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadCharacteristics {
    pub read_write_ratio: f64,  // 0.0 = all writes, 1.0 = all reads
    pub avg_query_complexity: f64,
    pub concurrent_connections: usize,
    pub transaction_rate: f64,
    pub table_scan_ratio: f64,
    pub index_scan_ratio: f64,
    pub memory_intensive: bool,
    pub io_intensive: bool,
    pub cpu_intensive: bool,
}

/// Reinforcement learning state
#[derive(Debug, Clone)]
struct RLState {
    workload: WorkloadCharacteristics,
    parameters: HashMap<TunableParameter, ParameterValue>,
    performance_score: f64,
}

/// Q-Learning agent for parameter optimization
struct QLearningAgent {
    q_table: HashMap<String, HashMap<String, f64>>,
    learning_rate: f64,
    discount_factor: f64,
    epsilon: f64,  // Exploration rate
    epsilon_decay: f64,
}

impl QLearningAgent {
    fn new() -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate: 0.1,
            discount_factor: 0.95,
            epsilon: 0.2,
            epsilon_decay: 0.995,
        }
    }

    fn state_key(&self, state: &RLState) -> String {
        // Discretize state for Q-table
        format!(
            "rw:{:.1}_qc:{:.1}_cc:{}_cpu:{}_mem:{}_io:{}",
            state.workload.read_write_ratio,
            state.workload.avg_query_complexity,
            state.workload.concurrent_connections / 10,
            state.workload.cpu_intensive,
            state.workload.memory_intensive,
            state.workload.io_intensive
        )
    }

    fn action_key(parameter: &TunableParameter, direction: i32) -> String {
        format!("{:?}_{}", parameter, if direction > 0 { "inc" } else { "dec" })
    }

    fn select_action(
        &mut self,
        state: &RLState,
        available_parameters: &[TunableParameter],
    ) -> Option<(TunableParameter, i32)> {
        if available_parameters.is_empty() {
            return None;
        }

        let state_key = self.state_key(state);

        // Epsilon-greedy exploration
        if rand::random::<f64>() < self.epsilon {
            // Explore: random action
            let param = available_parameters[rand::random::<usize>() % available_parameters.len()].clone();
            let direction = if rand::random::<bool>() { 1 } else { -1 };
            return Some((param, direction));
        }

        // Exploit: choose best known action
        let q_values = self.q_table.entry(state_key.clone()).or_insert_with(HashMap::new);

        let mut best_action: Option<(TunableParameter, i32)> = None;
        let mut best_q = f64::NEG_INFINITY;

        for param in available_parameters {
            for &direction in &[1i32, -1i32] {
                let action_key = Self::action_key(param, direction);
                let q = *q_values.get(&action_key).unwrap_or(&0.0);

                if q > best_q {
                    best_q = q;
                    best_action = Some((param.clone(), direction));
                }
            }
        }

        best_action.or_else(|| {
            // Fallback to random if no Q-values learned yet
            let param = available_parameters[0].clone();
            Some((param, 1))
        })
    }

    fn update_q_value(
        &mut self,
        state: &RLState,
        action: (TunableParameter, i32),
        reward: f64,
        next_state: &RLState,
    ) {
        let state_key = self.state_key(state);
        let action_key = Self::action_key(&action.0, action.1);

        let current_q = *self.q_table
            .entry(state_key.clone())
            .or_insert_with(HashMap::new)
            .entry(action_key.clone())
            .or_insert(0.0);

        // Find max Q-value for next state
        let next_state_key = self.state_key(next_state);
        let max_next_q = self.q_table
            .get(&next_state_key)
            .map(|actions| {
                actions.values().copied().fold(f64::NEG_INFINITY, f64::max)
            })
            .unwrap_or(0.0);

        // Q-learning update rule
        let new_q = current_q + self.learning_rate * (reward + self.discount_factor * max_next_q - current_q);

        self.q_table
            .entry(state_key)
            .or_insert_with(HashMap::new)
            .insert(action_key, new_q);

        // Decay exploration rate
        self.epsilon *= self.epsilon_decay;
        self.epsilon = self.epsilon.max(0.01); // Minimum exploration
    }
}

/// Regression detector for identifying performance degradation
struct RegressionDetector {
    metric_history: VecDeque<PerformanceMetrics>,
    window_size: usize,
    threshold: f64,
}

impl RegressionDetector {
    fn new(window_size: usize, threshold: f64) -> Self {
        Self {
            metric_history: VecDeque::with_capacity(window_size),
            window_size,
            threshold,
        }
    }

    fn add_metrics(&mut self, metrics: PerformanceMetrics) {
        if self.metric_history.len() >= self.window_size {
            self.metric_history.pop_front();
        }
        self.metric_history.push_back(metrics);
    }

    fn detect_regression(&self) -> Option<f64> {
        if self.metric_history.len() < self.window_size {
            return None;
        }

        let recent_count = self.window_size / 3;
        let historical_count = self.window_size - recent_count;

        if recent_count == 0 || historical_count == 0 {
            return None;
        }

        let historical_scores: Vec<f64> = self.metric_history
            .iter()
            .take(historical_count)
            .map(|m| m.calculate_score())
            .collect();

        let recent_scores: Vec<f64> = self.metric_history
            .iter()
            .skip(historical_count)
            .map(|m| m.calculate_score())
            .collect();

        let historical_avg = historical_scores.iter().sum::<f64>() / historical_scores.len() as f64;
        let recent_avg = recent_scores.iter().sum::<f64>() / recent_scores.len() as f64;

        let degradation = (historical_avg - recent_avg) / historical_avg;

        if degradation > self.threshold {
            Some(degradation)
        } else {
            None
        }
    }
}

/// Statistics gatherer for automatic statistics collection
pub struct StatisticsGatherer {
    table_stats: HashMap<String, TableStatistics>,
    last_gather_time: HashMap<String, SystemTime>,
    auto_gather_threshold: f64,  // Percentage of data change
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStatistics {
    pub table_name: String,
    pub row_count: usize,
    pub avg_row_size: usize,
    pub total_size_mb: usize,
    pub index_count: usize,
    pub last_analyzed: SystemTime,
    pub modification_count: usize,  // Rows modified since last analysis
}

impl StatisticsGatherer {
    pub fn new(auto_gather_threshold: f64) -> Self {
        Self {
            table_stats: HashMap::new(),
            last_gather_time: HashMap::new(),
            auto_gather_threshold,
        }
    }

    pub fn should_gather_stats(&self, table_name: &str, modifications: usize) -> bool {
        if let Some(stats) = self.table_stats.get(table_name) {
            let change_ratio = modifications as f64 / stats.row_count.max(1) as f64;
            change_ratio > self.auto_gather_threshold
        } else {
            true  // No stats exist, should gather
        }
    }

    pub fn update_stats(&mut self, table_name: String, stats: TableStatistics) {
        self.last_gather_time.insert(table_name.clone(), SystemTime::now());
        self.table_stats.insert(table_name, stats);
    }

    pub fn get_stats(&self, table_name: &str) -> Option<&TableStatistics> {
        self.table_stats.get(table_name)
    }
}

/// Main auto-tuner orchestrator
pub struct AutoTuner {
    aggressiveness: AggressivenessLevel,
    parameters: Arc<RwLock<HashMap<TunableParameter, ParameterConfig>>>,
    performance_history: Arc<RwLock<VecDeque<PerformanceMetrics>>>,
    tuning_history: Arc<RwLock<Vec<TuningResult>>>,
    rl_agent: Arc<RwLock<QLearningAgent>>,
    regression_detector: Arc<RwLock<RegressionDetector>>,
    statistics_gatherer: Arc<RwLock<StatisticsGatherer>>,
    enabled: Arc<RwLock<bool>>,
    optimization_interval: Duration,
}

impl AutoTuner {
    pub fn new(aggressiveness: AggressivenessLevel) -> Self {
        let optimization_interval = match aggressiveness {
            AggressivenessLevel::Conservative => Duration::from_secs(300),  // 5 minutes
            AggressivenessLevel::Moderate => Duration::from_secs(180),      // 3 minutes
            AggressivenessLevel::Aggressive => Duration::from_secs(60),     // 1 minute
        };

        Self {
            aggressiveness,
            parameters: Arc::new(RwLock::new(Self::initialize_parameters())),
            performance_history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            tuning_history: Arc::new(RwLock::new(Vec::new())),
            rl_agent: Arc::new(RwLock::new(QLearningAgent::new())),
            regression_detector: Arc::new(RwLock::new(RegressionDetector::new(30, 0.1))),
            statistics_gatherer: Arc::new(RwLock::new(StatisticsGatherer::new(0.1))),
            enabled: Arc::new(RwLock::new(true)),
            optimization_interval,
        }
    }

    fn initialize_parameters() -> HashMap<TunableParameter, ParameterConfig> {
        let mut params = HashMap::new();

        params.insert(
            TunableParameter::BufferPoolSize,
            ParameterConfig {
                parameter: TunableParameter::BufferPoolSize,
                current_value: ParameterValue::Integer(1000),
                min_value: ParameterValue::Integer(100),
                max_value: ParameterValue::Integer(100000),
                step_size: ParameterValue::Integer(100),
                requires_restart: false,
                last_changed: SystemTime::now(),
            },
        );

        params.insert(
            TunableParameter::SortAreaSize,
            ParameterConfig {
                parameter: TunableParameter::SortAreaSize,
                current_value: ParameterValue::Integer(65536),
                min_value: ParameterValue::Integer(4096),
                max_value: ParameterValue::Integer(1048576),
                step_size: ParameterValue::Integer(8192),
                requires_restart: false,
                last_changed: SystemTime::now(),
            },
        );

        params.insert(
            TunableParameter::WorkerThreads,
            ParameterConfig {
                parameter: TunableParameter::WorkerThreads,
                current_value: ParameterValue::Integer(4),
                min_value: ParameterValue::Integer(1),
                max_value: ParameterValue::Integer(64),
                step_size: ParameterValue::Integer(1),
                requires_restart: true,
                last_changed: SystemTime::now(),
            },
        );

        params.insert(
            TunableParameter::MaxConnections,
            ParameterConfig {
                parameter: TunableParameter::MaxConnections,
                current_value: ParameterValue::Integer(100),
                min_value: ParameterValue::Integer(10),
                max_value: ParameterValue::Integer(1000),
                step_size: ParameterValue::Integer(10),
                requires_restart: true,
                last_changed: SystemTime::now(),
            },
        );

        params
    }

    pub fn enable(&self) {
        *self.enabled.write() = true;
    }

    pub fn disable(&self) {
        *self.enabled.write() = false;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    pub fn set_aggressiveness(&mut self, level: AggressivenessLevel) {
        self.aggressiveness = level;
        self.optimization_interval = match level {
            AggressivenessLevel::Conservative => Duration::from_secs(300),
            AggressivenessLevel::Moderate => Duration::from_secs(180),
            AggressivenessLevel::Aggressive => Duration::from_secs(60),
        };
    }

    pub fn record_metrics(&self, metrics: PerformanceMetrics) {
        let mut history = self.performance_history.write();
        if history.len() >= 1000 {
            history.pop_front();
        }
        history.push_back(metrics.clone());

        self.regression_detector.write().add_metrics(metrics);
    }

    pub fn analyze_workload(&self) -> WorkloadCharacteristics {
        let history = self.performance_history.read();

        if history.is_empty() {
            return WorkloadCharacteristics {
                read_write_ratio: 0.5,
                avg_query_complexity: 0.5,
                concurrent_connections: 10,
                transaction_rate: 10.0,
                table_scan_ratio: 0.3,
                index_scan_ratio: 0.7,
                memory_intensive: false,
                io_intensive: false,
                cpu_intensive: false,
            };
        }

        let recent_count = 10.min(history.len());
        let recent: Vec<_> = history.iter().rev().take(recent_count).collect();

        let avg_qps: f64 = recent.iter().map(|m| m.queries_per_second).sum::<f64>() / recent_count as f64;
        let avg_cpu: f64 = recent.iter().map(|m| m.cpu_usage).sum::<f64>() / recent_count as f64;
        let avg_mem: usize = recent.iter().map(|m| m.memory_usage_mb).sum::<usize>() / recent_count;
        let avg_io: f64 = recent.iter().map(|m| m.disk_io_rate).sum::<f64>() / recent_count as f64;

        WorkloadCharacteristics {
            read_write_ratio: 0.7,  // Placeholder - would be tracked from query types
            avg_query_complexity: avg_qps / 1000.0,
            concurrent_connections: 20,  // Placeholder
            transaction_rate: avg_qps,
            table_scan_ratio: 0.3,  // Placeholder
            index_scan_ratio: 0.7,  // Placeholder
            memory_intensive: avg_mem > 2048,
            io_intensive: avg_io > 1000.0,
            cpu_intensive: avg_cpu > 0.7,
        }
    }

    pub fn recommend_tuning(&self, workload: &WorkloadCharacteristics) -> Vec<TuningAction> {
        let mut recommendations = Vec::new();
        let params = self.parameters.read();

        // Buffer pool recommendations
        if let Some(buffer_config) = params.get(&TunableParameter::BufferPoolSize) {
            if workload.memory_intensive && !workload.io_intensive {
                let current = buffer_config.current_value.as_i64().unwrap_or(1000);
                let new_value = (current as f64 * 1.2) as i64;

                recommendations.push(TuningAction {
                    parameter: TunableParameter::BufferPoolSize,
                    old_value: buffer_config.current_value.clone(),
                    new_value: ParameterValue::Integer(new_value),
                    reason: "Workload is memory-intensive, increasing buffer pool".to_string(),
                    timestamp: SystemTime::now(),
                    expected_improvement: 0.15,
                });
            }
        }

        // Worker thread recommendations
        if let Some(worker_config) = params.get(&TunableParameter::WorkerThreads) {
            if workload.concurrent_connections > 50 && workload.cpu_intensive {
                let current = worker_config.current_value.as_i64().unwrap_or(4);
                let new_value = (current + 2).min(16);

                recommendations.push(TuningAction {
                    parameter: TunableParameter::WorkerThreads,
                    old_value: worker_config.current_value.clone(),
                    new_value: ParameterValue::Integer(new_value),
                    reason: "High concurrency with CPU-intensive workload".to_string(),
                    timestamp: SystemTime::now(),
                    expected_improvement: 0.2,
                });
            }
        }

        // Sort area recommendations
        if let Some(sort_config) = params.get(&TunableParameter::SortAreaSize) {
            if workload.avg_query_complexity > 0.7 {
                let current = sort_config.current_value.as_i64().unwrap_or(65536);
                let new_value = (current as f64 * 1.5) as i64;

                recommendations.push(TuningAction {
                    parameter: TunableParameter::SortAreaSize,
                    old_value: sort_config.current_value.clone(),
                    new_value: ParameterValue::Integer(new_value),
                    reason: "Complex queries benefit from larger sort area".to_string(),
                    timestamp: SystemTime::now(),
                    expected_improvement: 0.1,
                });
            }
        }

        recommendations
    }

    pub fn apply_tuning(&self, action: &TuningAction) -> Result<()> {
        let mut params = self.parameters.write();

        if let Some(config) = params.get_mut(&action.parameter) {
            config.current_value = action.new_value.clone();
            config.last_changed = SystemTime::now();
        }

        Ok(())
    }

    pub fn rollback_tuning(&self, action: &TuningAction) -> Result<()> {
        let mut params = self.parameters.write();

        if let Some(config) = params.get_mut(&action.parameter) {
            config.current_value = action.old_value.clone();
            config.last_changed = SystemTime::now();
        }

        Ok(())
    }

    pub async fn start_optimization_loop(self: Arc<Self>) {
        let mut interval = tokio::time::interval(self.optimization_interval);

        loop {
            interval.tick().await;

            if !self.is_enabled() {
                continue;
            }

            // Check for regression
            let regression = self.regression_detector.read().detect_regression();
            if let Some(degradation) = regression {
                tracing::warn!("Performance regression detected: {:.2}%", degradation * 100.0);

                // Rollback recent changes if regression is severe
                if degradation > 0.2 {
                    if let Err(e) = self.auto_rollback().await {
                        tracing::error!("Failed to rollback tuning: {}", e);
                    }
                }
                continue;
            }

            // Analyze workload
            let workload = self.analyze_workload();

            // Get recommendations from RL agent
            let available_params = vec![
                TunableParameter::BufferPoolSize,
                TunableParameter::SortAreaSize,
            ];

            let current_state = self.build_rl_state(&workload);

            let action_selection = self.rl_agent.write().select_action(&current_state, &available_params);
            if let Some((param, direction)) = action_selection {
                // Apply action
                if let Some(action) = self.create_action_from_rl(param, direction, &workload) {
                    match self.apply_and_evaluate(action).await {
                        Ok(result) => {
                            // Update RL agent with reward
                            let reward = result.actual_improvement;
                            let new_state = self.build_rl_state(&self.analyze_workload());
                            self.rl_agent.write().update_q_value(
                                &current_state,
                                (result.action.parameter.clone(), direction),
                                reward,
                                &new_state,
                            );

                            self.tuning_history.write().push(result);
                        }
                        Err(e) => {
                            tracing::error!("Failed to apply tuning: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn build_rl_state(&self, workload: &WorkloadCharacteristics) -> RLState {
        let params_map: HashMap<TunableParameter, ParameterValue> = self.parameters
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.current_value.clone()))
            .collect();

        let performance_score = self.performance_history
            .read()
            .back()
            .map(|m| m.calculate_score())
            .unwrap_or(0.0);

        RLState {
            workload: workload.clone(),
            parameters: params_map,
            performance_score,
        }
    }

    fn create_action_from_rl(
        &self,
        parameter: TunableParameter,
        direction: i32,
        workload: &WorkloadCharacteristics,
    ) -> Option<TuningAction> {
        let params = self.parameters.read();
        let config = params.get(&parameter)?;

        let current = config.current_value.as_i64()?;
        let step = config.step_size.as_i64()?;
        let new_value = if direction > 0 {
            current + step
        } else {
            current - step
        };

        let new_value = new_value.max(config.min_value.as_i64()?).min(config.max_value.as_i64()?);

        Some(TuningAction {
            parameter: parameter.clone(),
            old_value: config.current_value.clone(),
            new_value: ParameterValue::Integer(new_value),
            reason: format!("RL-driven {} of {:?}", if direction > 0 { "increase" } else { "decrease" }, parameter),
            timestamp: SystemTime::now(),
            expected_improvement: 0.05,
        })
    }

    async fn apply_and_evaluate(&self, action: TuningAction) -> Result<TuningResult> {
        let before_metrics = self.performance_history
            .read()
            .back()
            .cloned()
            .unwrap_or_default();

        // Apply tuning
        self.apply_tuning(&action)?;

        // Wait for effect to stabilize
        sleep(Duration::from_secs(10)).await;

        let after_metrics = self.performance_history
            .read()
            .back()
            .cloned()
            .unwrap_or_default();

        let actual_improvement = after_metrics.calculate_score() - before_metrics.calculate_score();

        let rollback_performed = if actual_improvement < -0.05 {
            // Performance degraded, rollback
            self.rollback_tuning(&action)?;
            true
        } else {
            false
        };

        Ok(TuningResult {
            action,
            before_metrics,
            after_metrics,
            actual_improvement,
            rollback_performed,
            timestamp: SystemTime::now(),
        })
    }

    async fn auto_rollback(&self) -> Result<()> {
        // Find recent successful tunings (clone the data we need)
        let recent_actions: Vec<_> = {
            let history = self.tuning_history.read();
            history
                .iter()
                .rev()
                .take(5)
                .filter(|r| !r.rollback_performed && r.actual_improvement > 0.0)
                .cloned()
                .collect()
        };

        // Rollback in reverse order
        for result in recent_actions.iter().rev() {
            self.rollback_tuning(&result.action)?;
            tracing::info!("Rolled back tuning for {:?}", result.action.parameter);
        }

        Ok(())
    }

    pub fn get_tuning_report(&self) -> TuningReport {
        let history = self.tuning_history.read();
        let total_tunings = history.len();
        let successful_tunings = history.iter().filter(|r| r.actual_improvement > 0.0).count();
        let rollbacks = history.iter().filter(|r| r.rollback_performed).count();

        let avg_improvement = if !history.is_empty() {
            history.iter().map(|r| r.actual_improvement).sum::<f64>() / history.len() as f64
        } else {
            0.0
        };

        TuningReport {
            total_tunings,
            successful_tunings,
            rollbacks,
            avg_improvement,
            current_performance_score: self.performance_history
                .read()
                .back()
                .map(|m| m.calculate_score())
                .unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningReport {
    pub total_tunings: usize,
    pub successful_tunings: usize,
    pub rollbacks: usize,
    pub avg_improvement: f64,
    pub current_performance_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_score() {
        let metrics = PerformanceMetrics {
            queries_per_second: 100.0,
            avg_response_time_ms: 50.0,
            buffer_hit_rate: 0.95,
            cache_hit_rate: 0.90,
            lock_wait_time_ms: 5.0,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_auto_tuner_creation() {
        let tuner = AutoTuner::new(AggressivenessLevel::Moderate);
        assert!(tuner.is_enabled());
    }

    #[test]
    fn test_statistics_gatherer() {
        let gatherer = StatisticsGatherer::new(0.1);
        assert!(gatherer.should_gather_stats("test_table", 1000));
    }
}


