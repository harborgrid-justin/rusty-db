// Memory Management Web API

use super::common::*;
use super::debugger::{ComponentBreakdown, MemoryReport};
use super::memory_manager::*;
use super::pressure_manager::MemoryPressureEvent;

pub struct MemoryApi {
    manager: Arc<MemoryManager>,
}

impl MemoryApi {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }

    // Get comprehensive statistics as JSON-compatible structure
    pub fn api_get_stats(&self) -> ComprehensiveMemoryStats {
        self.manager.get_comprehensive_stats()
    }

    // Get memory usage summary
    pub fn api_get_usage_summary(&self) -> UsageSummary {
        let stats = self.manager.get_comprehensive_stats();

        UsageSummary {
            total_memory: stats.total_usage.total_memory,
            used_memory: stats.total_usage.used_memory,
            available_memory: stats.total_usage.available_memory,
            usage_percentage: stats.total_usage.usage_ratio * 100.0,
            pressure_level: format!("{:?}", stats.total_usage.pressure_level),
            slab_usage: stats.slab_stats.bytes_allocated,
            arena_active_contexts: stats.arena_stats.active_contexts,
            large_object_count: stats.large_object_stats.active_objects,
        }
    }

    // Get component breakdown
    pub fn api_get_component_breakdown(&self) -> Vec<ComponentBreakdown> {
        self.manager
            .debugger()
            .generate_report()
            .component_breakdown
    }

    // Detect memory leaks
    pub fn api_detect_leaks(&self, min_age_seconds: u64) -> Vec<LeakReport> {
        self.manager
            .debugger()
            .detect_leaks(Duration::from_secs(min_age_seconds))
    }

    // Enable debugging features
    pub fn api_enable_debugging(&self, feature: &str) -> Result<()> {
        match feature {
            "tracking" => {
                self.manager.debugger().enable_tracking();
                Ok(())
            }
            "leak_detection" => {
                self.manager.debugger().enable_leak_detection();
                Ok(())
            }
            "uaf_detection" => {
                self.manager.debugger().enable_uaf_detection();
                Ok(())
            }
            "guards" => {
                self.manager.debugger().enable_guards();
                Ok(())
            }
            "stack_traces" => {
                self.manager.debugger().enable_stack_traces();
                Ok(())
            }
            _ => Err(DbError::InvalidArgument(format!(
                "Unknown feature: {}",
                feature
            ))),
        }
    }

    // Disable debugging features
    pub fn api_disable_debugging(&self, feature: &str) -> Result<()> {
        match feature {
            "tracking" => {
                self.manager.debugger().disable_tracking();
                Ok(())
            }
            _ => Err(DbError::InvalidArgument(format!(
                "Cannot disable: {}",
                feature
            ))),
        }
    }

    // Get pressure events
    pub fn api_get_pressure_events(&self, count: usize) -> Vec<MemoryPressureEvent> {
        self.manager.pressure_manager().get_recent_events(count)
    }

    // Force emergency memory release
    pub fn api_force_emergency_release(&self) -> Result<()> {
        self.manager.pressure_manager().emergency_release()
    }

    // Set memory limit
    pub fn api_set_memory_limit(&self, limit_bytes: u64) {
        self.manager
            .pressure_manager()
            .set_total_memory(limit_bytes);
    }

    // Generate full memory report
    pub fn api_generate_report(&self) -> MemoryReport {
        self.manager.debugger().generate_report()
    }
}

// Usage summary for web display
#[derive(Debug, Clone)]
pub struct UsageSummary {
    pub total_memory: u64,
    pub used_memory: u64,
    pub available_memory: u64,
    pub usage_percentage: f64,
    pub pressure_level: String,
    pub slab_usage: u64,
    pub arena_active_contexts: u64,
    pub large_object_count: u64,
}

// ============================================================================
// UTILITY FUNCTIONS & HELPERS
// ============================================================================
// Note: Utility functions moved to utils.rs module
