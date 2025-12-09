// CTE statistics collection and reporting

use std::collections::HashMap;

/// CTE statistics collector for monitoring and optimization
pub struct CteStatistics {
    execution_counts: HashMap<String, usize>,
    execution_times: HashMap<String, u128>,
    row_counts: HashMap<String, usize>,
    memory_usage: HashMap<String, usize>,
}

impl CteStatistics {
    pub fn new() -> Self {
        Self {
            execution_counts: HashMap::new(),
            execution_times: HashMap::new(),
            row_counts: HashMap::new(),
            memory_usage: HashMap::new(),
        }
    }

    pub fn record_execution(
        &mut self,
        cte_name: &str,
        duration_ms: u128,
        row_count: usize,
        memory_bytes: usize,
    ) {
        *self.execution_counts.entry(cte_name.to_string()).or_insert(0) += 1;
        *self.execution_times.entry(cte_name.to_string()).or_insert(0) += duration_ms;
        self.row_counts.insert(cte_name.to_string(), row_count);
        self.memory_usage.insert(cte_name.to_string(), memory_bytes);
    }

    pub fn get_average_execution_time(&self, cte_name: &str) -> Option<f64> {
        let count = self.execution_counts.get(cte_name)?;
        let total_time = self.execution_times.get(cte_name)?;

        if *count == 0 {
            return None;
        }

        Some(*total_time as f64 / *count as f64)
    }

    pub fn get_total_memory_usage(&self) -> usize {
        self.memory_usage.values().sum()
    }

    pub fn generate_report(&self) -> CteStatisticsReport {
        CteStatisticsReport {
            total_ctes: self.execution_counts.len(),
            total_executions: self.execution_counts.values().sum(),
            total_memory_bytes: self.get_total_memory_usage(),
            cte_details: self.execution_counts.keys()
                .map(|name| {
                    let count = self.execution_counts.get(name).copied().unwrap_or(0);
                    let avg_time = self.get_average_execution_time(name).unwrap_or(0.0);
                    let rows = self.row_counts.get(name).copied().unwrap_or(0);
                    let memory = self.memory_usage.get(name).copied().unwrap_or(0);

                    CteDetail {
                        name: name.clone(),
                        execution_count: count,
                        average_time_ms: avg_time,
                        row_count: rows,
                        memory_bytes: memory,
                    }
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct CteStatisticsReport {
    pub total_ctes: usize,
    pub total_executions: usize,
    pub total_memory_bytes: usize,
    pub cte_details: Vec<CteDetail>,
}

#[derive(Debug)]
pub struct CteDetail {
    pub name: String,
    pub execution_count: usize,
    pub average_time_ms: f64,
    pub row_count: usize,
    pub memory_bytes: usize,
}
