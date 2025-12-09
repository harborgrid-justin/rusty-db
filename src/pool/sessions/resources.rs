//! Session resource control module
use std::time::Duration;

pub struct ResourceLimits {
    pub memory_limit: Option<usize>,
    pub cpu_limit: Option<Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self { memory_limit: None, cpu_limit: None }
    }
}

pub struct ResourceController;
pub struct ResourceGovernor;
pub struct ConsumerGroup;
pub struct ResourcePlan;
