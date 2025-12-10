/// Operator Pipeline Module
///
/// Core streaming operator infrastructure and pipeline management.

use super::super::{Event, EventBatch};
use crate::error::Result;
use std::time::{Duration, SystemTime};

/// Stream operator trait
pub trait StreamOperator: Send + Sync {
    fn process(&mut self, event: Event) -> Result<Vec<Event>>;
    fn name(&self) -> &str;
}

/// Operator pipeline
pub struct OperatorPipeline {
    _name: String,
    operators: Vec<Box<dyn StreamOperator>>,
    metrics: PipelineMetrics,
}

impl OperatorPipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            _name: name.into(),
            operators: Vec::new(),
            metrics: PipelineMetrics::default(),
        }
    }

    pub fn add_operator(&mut self, operator: Box<dyn StreamOperator>) {
        self.operators.push(operator);
    }

    pub fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        let start = SystemTime::now();
        let mut events = vec![event];

        for operator in &mut self.operators {
            let mut next_events = Vec::new();

            for event in events {
                let mut results = operator.process(event)?;
                next_events.append(&mut results);
            }

            events = next_events;

            if events.is_empty() {
                break;
            }
        }

        if let Ok(elapsed) = SystemTime::now().duration_since(start) {
            self.metrics.record_processing(elapsed);
        }

        self.metrics.events_processed += 1;
        self.metrics.events_output += events.len() as u64;

        Ok(events)
    }

    pub fn process_batch(&mut self, batch: EventBatch) -> Result<Vec<Event>> {
        let mut all_results = Vec::new();

        for event in batch.events {
            let mut results = self.process(event)?;
            all_results.append(&mut results);
        }

        Ok(all_results)
    }

    pub fn metrics(&self) -> &PipelineMetrics {
        &self.metrics
    }
}

/// Pipeline metrics
#[derive(Debug, Default, Clone)]
pub struct PipelineMetrics {
    pub events_processed: u64,
    pub events_output: u64,
    pub total_latency_ms: u64,
    pub avg_latency_ms: f64,
}

impl PipelineMetrics {
    pub(crate) fn record_processing(&mut self, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;
        self.total_latency_ms += latency_ms;
        self.avg_latency_ms = self.total_latency_ms as f64 / self.events_processed.max(1) as f64;
    }
}
