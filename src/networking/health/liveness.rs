// # Liveness and Readiness Probes
//
// Implements liveness probes (is the node alive?), readiness probes (can it serve traffic?),
// and startup probes with configurable thresholds and graceful degradation.

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Probe result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeResult {
    /// Probe succeeded
    Success,

    /// Probe failed
    Failure(String),

    /// Probe timed out
    Timeout,

    /// Probe not yet executed
    Pending,
}

impl ProbeResult {
    /// Check if probe was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ProbeResult::Success)
    }

    /// Check if probe failed
    pub fn is_failure(&self) -> bool {
        matches!(self, ProbeResult::Failure(_) | ProbeResult::Timeout)
    }
}

/// Probe configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    /// Initial delay before first probe
    pub initial_delay: Duration,

    /// Period between probes
    pub period: Duration,

    /// Timeout for each probe
    pub timeout: Duration,

    /// Number of consecutive successes required
    pub success_threshold: u32,

    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(0),
            period: Duration::from_secs(10),
            timeout: Duration::from_secs(1),
            success_threshold: 1,
            failure_threshold: 3,
        }
    }
}

/// Liveness probe trait - checks if the node is alive
#[async_trait]
pub trait LivenessProbe: Send + Sync {
    /// Execute the liveness probe
    async fn probe(&self) -> Result<ProbeResult>;

    /// Get probe configuration
    fn config(&self) -> &ProbeConfig;

    /// Get probe type name
    fn probe_type(&self) -> &str;
}

/// Readiness probe trait - checks if the node can serve traffic
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Execute the readiness probe
    async fn probe(&self) -> Result<ProbeResult>;

    /// Get probe configuration
    fn config(&self) -> &ProbeConfig;

    /// Get probe type name
    fn probe_type(&self) -> &str;
}

/// Startup probe trait - checks if the application has started
#[async_trait]
pub trait StartupProbe: Send + Sync {
    /// Execute the startup probe
    async fn probe(&self) -> Result<ProbeResult>;

    /// Get probe configuration
    fn config(&self) -> &ProbeConfig;

    /// Get probe type name
    fn probe_type(&self) -> &str;
}

/// Simple TCP liveness probe
pub struct TcpLivenessProbe {
    host: String,
    port: u16,
    config: ProbeConfig,
}

impl TcpLivenessProbe {
    /// Create a new TCP liveness probe
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            config: ProbeConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(host: String, port: u16, config: ProbeConfig) -> Self {
        Self { host, port, config }
    }
}

#[async_trait]
impl LivenessProbe for TcpLivenessProbe {
    async fn probe(&self) -> Result<ProbeResult> {
        use tokio::net::TcpStream;
        use tokio::time::timeout;

        let addr = format!("{}:{}", self.host, self.port);

        match timeout(self.config.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => Ok(ProbeResult::Success),
            Ok(Err(e)) => Ok(ProbeResult::Failure(e.to_string())),
            Err(_) => Ok(ProbeResult::Timeout),
        }
    }

    fn config(&self) -> &ProbeConfig {
        &self.config
    }

    fn probe_type(&self) -> &str {
        "tcp_liveness"
    }
}

/// HTTP readiness probe
pub struct HttpReadinessProbe {
    url: String,
    expected_status: u16,
    config: ProbeConfig,
}

impl HttpReadinessProbe {
    /// Create a new HTTP readiness probe
    pub fn new(url: String, expected_status: u16) -> Self {
        Self {
            url,
            expected_status,
            config: ProbeConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(url: String, expected_status: u16, config: ProbeConfig) -> Self {
        Self {
            url,
            expected_status,
            config,
        }
    }
}

#[async_trait]
impl ReadinessProbe for HttpReadinessProbe {
    async fn probe(&self) -> Result<ProbeResult> {
        // For now, just do a basic TCP connection check
        // In production, you'd use a real HTTP client
        use tokio::net::TcpStream;
        use tokio::time::timeout;

        let url_parts: Vec<&str> = self.url.split("://").collect();
        if url_parts.len() < 2 {
            return Ok(ProbeResult::Failure("Invalid URL format".to_string()));
        }

        let host_port: Vec<&str> = url_parts[1].split(':').collect();
        let host = host_port[0].to_string();
        let port = if host_port.len() > 1 {
            host_port[1]
                .split('/')
                .next()
                .unwrap_or("80")
                .parse::<u16>()
                .unwrap_or(80)
        } else {
            if url_parts[0] == "https" {
                443
            } else {
                80
            }
        };

        let addr = format!("{}:{}", host, port);

        match timeout(self.config.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => Ok(ProbeResult::Success),
            Ok(Err(e)) => Ok(ProbeResult::Failure(e.to_string())),
            Err(_) => Ok(ProbeResult::Timeout),
        }
    }

    fn config(&self) -> &ProbeConfig {
        &self.config
    }

    fn probe_type(&self) -> &str {
        "http_readiness"
    }
}

/// Custom liveness probe using a closure
pub struct CustomLivenessProbe {
    probe_fn: Box<dyn Fn() -> Result<bool> + Send + Sync>,
    config: ProbeConfig,
    probe_type_name: String,
}

impl CustomLivenessProbe {
    /// Create a new custom liveness probe
    pub fn new<F>(probe_type_name: String, probe_fn: F) -> Self
    where
        F: Fn() -> Result<bool> + Send + Sync + 'static,
    {
        Self {
            probe_fn: Box::new(probe_fn),
            config: ProbeConfig::default(),
            probe_type_name,
        }
    }

    /// Create with custom configuration
    pub fn with_config<F>(probe_type_name: String, probe_fn: F, config: ProbeConfig) -> Self
    where
        F: Fn() -> Result<bool> + Send + Sync + 'static,
    {
        Self {
            probe_fn: Box::new(probe_fn),
            config,
            probe_type_name,
        }
    }
}

#[async_trait]
impl LivenessProbe for CustomLivenessProbe {
    async fn probe(&self) -> Result<ProbeResult> {
        match (self.probe_fn)() {
            Ok(true) => Ok(ProbeResult::Success),
            Ok(false) => Ok(ProbeResult::Failure("Probe returned false".to_string())),
            Err(e) => Ok(ProbeResult::Failure(e.to_string())),
        }
    }

    fn config(&self) -> &ProbeConfig {
        &self.config
    }

    fn probe_type(&self) -> &str {
        &self.probe_type_name
    }
}

/// Probe executor - manages probe execution and tracks results
pub struct ProbeExecutor {
    /// Consecutive successes
    consecutive_successes: u32,

    /// Consecutive failures
    consecutive_failures: u32,

    /// Last probe time
    last_probe_time: Option<Instant>,

    /// Current state (healthy or not)
    is_healthy: bool,

    /// Probe has started
    has_started: bool,

    /// Start time
    start_time: Instant,
}

impl ProbeExecutor {
    /// Create a new probe executor
    pub fn new() -> Self {
        Self {
            consecutive_successes: 0,
            consecutive_failures: 0,
            last_probe_time: None,
            is_healthy: false,
            has_started: false,
            start_time: Instant::now(),
        }
    }

    /// Execute a liveness probe
    pub async fn execute_liveness<P: LivenessProbe + ?Sized>(&mut self, probe: &P) -> Result<bool> {
        // Check if we should wait for initial delay
        if !self.has_started && self.start_time.elapsed() < probe.config().initial_delay {
            return Ok(self.is_healthy);
        }

        self.has_started = true;

        // Check if enough time has passed since last probe
        if let Some(last_time) = self.last_probe_time {
            if last_time.elapsed() < probe.config().period {
                return Ok(self.is_healthy);
            }
        }

        // Execute the probe
        let result = probe.probe().await?;
        self.last_probe_time = Some(Instant::now());

        // Update state based on result
        self.update_state(result, probe.config());

        Ok(self.is_healthy)
    }

    /// Execute a readiness probe
    pub async fn execute_readiness<P: ReadinessProbe + ?Sized>(
        &mut self,
        probe: &P,
    ) -> Result<bool> {
        // Check if we should wait for initial delay
        if !self.has_started && self.start_time.elapsed() < probe.config().initial_delay {
            return Ok(false); // Not ready yet
        }

        self.has_started = true;

        // Check if enough time has passed since last probe
        if let Some(last_time) = self.last_probe_time {
            if last_time.elapsed() < probe.config().period {
                return Ok(self.is_healthy);
            }
        }

        // Execute the probe
        let result = probe.probe().await?;
        self.last_probe_time = Some(Instant::now());

        // Update state based on result
        self.update_state(result, probe.config());

        Ok(self.is_healthy)
    }

    /// Execute a startup probe
    pub async fn execute_startup<P: StartupProbe + ?Sized>(&mut self, probe: &P) -> Result<bool> {
        // Startup probes run immediately
        if !self.has_started {
            self.has_started = true;
        }

        // Check if enough time has passed since last probe
        if let Some(last_time) = self.last_probe_time {
            if last_time.elapsed() < probe.config().period {
                return Ok(self.is_healthy);
            }
        }

        // Execute the probe
        let result = probe.probe().await?;
        self.last_probe_time = Some(Instant::now());

        // Update state based on result
        self.update_state(result, probe.config());

        Ok(self.is_healthy)
    }

    /// Update state based on probe result
    fn update_state(&mut self, result: ProbeResult, config: &ProbeConfig) {
        match result {
            ProbeResult::Success => {
                self.consecutive_successes += 1;
                self.consecutive_failures = 0;

                if self.consecutive_successes >= config.success_threshold {
                    self.is_healthy = true;
                }
            }
            ProbeResult::Failure(_) | ProbeResult::Timeout => {
                self.consecutive_failures += 1;
                self.consecutive_successes = 0;

                if self.consecutive_failures >= config.failure_threshold {
                    self.is_healthy = false;
                }
            }
            ProbeResult::Pending => {
                // Do nothing
            }
        }
    }

    /// Get current health status
    pub fn is_healthy(&self) -> bool {
        self.is_healthy
    }

    /// Get consecutive successes
    pub fn consecutive_successes(&self) -> u32 {
        self.consecutive_successes
    }

    /// Get consecutive failures
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Reset the executor
    pub fn reset(&mut self) {
        self.consecutive_successes = 0;
        self.consecutive_failures = 0;
        self.is_healthy = false;
        self.has_started = false;
        self.start_time = Instant::now();
    }
}

impl Default for ProbeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined probe manager for a node
pub struct NodeProbeManager {
    /// Liveness probe and executor
    liveness: Option<(Box<dyn LivenessProbe + Send + Sync>, ProbeExecutor)>,

    /// Readiness probe and executor
    readiness: Option<(Box<dyn ReadinessProbe + Send + Sync>, ProbeExecutor)>,

    /// Startup probe and executor
    startup: Option<(Box<dyn StartupProbe + Send + Sync>, ProbeExecutor)>,

    /// Has startup completed
    startup_completed: bool,
}

impl NodeProbeManager {
    /// Create a new node probe manager
    pub fn new() -> Self {
        Self {
            liveness: None,
            readiness: None,
            startup: None,
            startup_completed: false,
        }
    }

    /// Set liveness probe
    pub fn set_liveness_probe(&mut self, probe: Box<dyn LivenessProbe + Send + Sync>) {
        self.liveness = Some((probe, ProbeExecutor::new()));
    }

    /// Set readiness probe
    pub fn set_readiness_probe(&mut self, probe: Box<dyn ReadinessProbe + Send + Sync>) {
        self.readiness = Some((probe, ProbeExecutor::new()));
    }

    /// Set startup probe
    pub fn set_startup_probe(&mut self, probe: Box<dyn StartupProbe + Send + Sync>) {
        self.startup = Some((probe, ProbeExecutor::new()));
    }

    /// Check liveness
    pub async fn check_liveness(&mut self) -> Result<bool> {
        // Don't run liveness checks until startup completes
        if !self.startup_completed && self.startup.is_some() {
            return Ok(true); // Assume alive during startup
        }

        if let Some((probe, executor)) = &mut self.liveness {
            executor.execute_liveness(probe.as_ref()).await
        } else {
            Ok(true) // No probe configured, assume alive
        }
    }

    /// Check readiness
    pub async fn check_readiness(&mut self) -> Result<bool> {
        // Not ready until startup completes
        if !self.startup_completed && self.startup.is_some() {
            return Ok(false);
        }

        if let Some((probe, executor)) = &mut self.readiness {
            executor.execute_readiness(probe.as_ref()).await
        } else {
            Ok(true) // No probe configured, assume ready
        }
    }

    /// Check startup
    pub async fn check_startup(&mut self) -> Result<bool> {
        if self.startup_completed {
            return Ok(true);
        }

        if let Some((probe, executor)) = &mut self.startup {
            let result = executor.execute_startup(probe.as_ref()).await?;
            if result {
                self.startup_completed = true;
            }
            Ok(result)
        } else {
            // No startup probe, consider startup complete
            self.startup_completed = true;
            Ok(true)
        }
    }

    /// Check all probes
    pub async fn check_all(&mut self) -> Result<ProbeStatus> {
        let startup = self.check_startup().await?;
        let liveness = self.check_liveness().await?;
        let readiness = self.check_readiness().await?;

        Ok(ProbeStatus {
            startup_complete: startup,
            is_alive: liveness,
            is_ready: readiness,
        })
    }
}

impl Default for NodeProbeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Overall probe status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeStatus {
    /// Startup has completed
    pub startup_complete: bool,

    /// Node is alive
    pub is_alive: bool,

    /// Node is ready to serve traffic
    pub is_ready: bool,
}

impl ProbeStatus {
    /// Check if node can serve traffic
    pub fn can_serve_traffic(&self) -> bool {
        self.startup_complete && self.is_alive && self.is_ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_result() {
        let success = ProbeResult::Success;
        assert!(success.is_success());
        assert!(!success.is_failure());

        let failure = ProbeResult::Failure("error".to_string());
        assert!(!failure.is_success());
        assert!(failure.is_failure());
    }

    #[test]
    fn test_probe_executor() {
        let mut executor = ProbeExecutor::new();
        assert!(!executor.is_healthy());
        assert_eq!(executor.consecutive_successes(), 0);
    }

    #[tokio::test]
    async fn test_tcp_liveness_probe() {
        let probe = TcpLivenessProbe::new("127.0.0.1".to_string(), 9999);
        assert_eq!(probe.probe_type(), "tcp_liveness");

        // This will likely fail/timeout since nothing is listening
        let result = probe.probe().await.unwrap();
        // Don't assert on specific result as it depends on environment
    }

    #[test]
    fn test_custom_liveness_probe() {
        let probe = CustomLivenessProbe::new("custom".to_string(), || Ok(true));

        assert_eq!(probe.probe_type(), "custom");
    }

    #[tokio::test]
    async fn test_node_probe_manager() {
        let mut manager = NodeProbeManager::new();

        let liveness = Box::new(CustomLivenessProbe::new("test".to_string(), || Ok(true)));

        manager.set_liveness_probe(liveness);

        // Startup should complete immediately since no startup probe
        assert!(manager.check_startup().await.unwrap());

        // Liveness should eventually be true
        assert!(manager.check_liveness().await.is_ok());
    }

    #[tokio::test]
    async fn test_probe_status() {
        let status = ProbeStatus {
            startup_complete: true,
            is_alive: true,
            is_ready: true,
        };

        assert!(status.can_serve_traffic());

        let not_ready = ProbeStatus {
            startup_complete: true,
            is_alive: true,
            is_ready: false,
        };

        assert!(!not_ready.can_serve_traffic());
    }
}
