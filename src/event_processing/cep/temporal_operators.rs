/// Temporal Operators Module
///
/// Time-based constraints, measurements, and match contexts for CEP.

use super::super::{Event, EventValue};
use super::pattern_matching::Pattern;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Temporal constraint for patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalConstraint {
    /// Pattern must complete within duration
    Within(Duration),

    /// Events must be within duration of each other
    WithinEach(Duration),

    /// Events must occur after a specific time
    After(SystemTime),

    /// Events must occur before a specific time
    Before(SystemTime),

    /// Composite temporal constraints
    And(Vec<TemporalConstraint>),
}

impl TemporalConstraint {
    pub fn evaluate(&self, events: &[&Event]) -> bool {
        if events.is_empty() {
            return false;
        }

        match self {
            TemporalConstraint::Within(duration) => {
                let first_time = events[0].event_time;
                let last_time = events.last().unwrap().event_time;

                if let Ok(diff) = last_time.duration_since(first_time) {
                    diff <= *duration
                } else {
                    false
                }
            }

            TemporalConstraint::WithinEach(duration) => {
                for window in events.windows(2) {
                    if let Ok(diff) = window[1].event_time.duration_since(window[0].event_time) {
                        if diff > *duration {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }

            TemporalConstraint::After(time) => events.iter().all(|e| e.event_time >= *time),

            TemporalConstraint::Before(time) => events.iter().all(|e| e.event_time <= *time),

            TemporalConstraint::And(constraints) => {
                constraints.iter().all(|c| c.evaluate(events))
            }
        }
    }
}

/// Measure to extract from pattern matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measure {
    /// Measure name
    pub name: String,

    /// Aggregation function
    pub aggregation: Aggregation,

    /// Field to aggregate
    pub field: Option<String>,
}

impl Measure {
    pub fn new(name: impl Into<String>, aggregation: Aggregation) -> Self {
        Self {
            name: name.into(),
            aggregation,
            field: None,
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn compute(&self, events: &[&Event]) -> EventValue {
        match &self.aggregation {
            Aggregation::Count => EventValue::Int64(events.len() as i64),

            Aggregation::First => {
                if let Some(event) = events.first() {
                    if let Some(field) = &self.field {
                        event.get_payload(field).cloned().unwrap_or(EventValue::Null)
                    } else {
                        EventValue::Null
                    }
                } else {
                    EventValue::Null
                }
            }

            Aggregation::Last => {
                if let Some(event) = events.last() {
                    if let Some(field) = &self.field {
                        event.get_payload(field).cloned().unwrap_or(EventValue::Null)
                    } else {
                        EventValue::Null
                    }
                } else {
                    EventValue::Null
                }
            }

            Aggregation::Sum => {
                if let Some(field) = &self.field {
                    let sum: f64 = events
                        .iter()
                        .filter_map(|e| e.get_payload(field))
                        .filter_map(|v| v.as_f64())
                        .sum();
                    EventValue::Float64(sum)
                } else {
                    EventValue::Null
                }
            }

            Aggregation::Avg => {
                if let Some(field) = &self.field {
                    let values: Vec<f64> = events
                        .iter()
                        .filter_map(|e| e.get_payload(field))
                        .filter_map(|v| v.as_f64())
                        .collect();

                    if !values.is_empty() {
                        let sum: f64 = values.iter().sum();
                        EventValue::Float64(sum / values.len() as f64)
                    } else {
                        EventValue::Null
                    }
                } else {
                    EventValue::Null
                }
            }

            Aggregation::Min => {
                if let Some(field) = &self.field {
                    let min = events
                        .iter()
                        .filter_map(|e| e.get_payload(field))
                        .filter_map(|v| v.as_f64())
                        .min_by(|a, b| a.partial_cmp(b).unwrap());

                    min.map(EventValue::Float64).unwrap_or(EventValue::Null)
                } else {
                    EventValue::Null
                }
            }

            Aggregation::Max => {
                if let Some(field) = &self.field {
                    let max = events
                        .iter()
                        .filter_map(|e| e.get_payload(field))
                        .filter_map(|v| v.as_f64())
                        .max_by(|a, b| a.partial_cmp(b).unwrap());

                    max.map(EventValue::Float64).unwrap_or(EventValue::Null)
                } else {
                    EventValue::Null
                }
            }
        }
    }
}

/// Aggregation functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Aggregation {
    Count,
    First,
    Last,
    Sum,
    Avg,
    Min,
    Max,
}

/// Skip strategy for overlapping matches
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SkipStrategy {
    /// Skip to the next row after match start
    PastLastRow,

    /// Skip to the first row of the next match
    ToNextMatch,

    /// Skip to the first row after the current match
    ToFirstRow,

    /// No skip, find all overlapping matches
    NoSkip,
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Matched events by variable name
    pub events: HashMap<String, Vec<Event>>,

    /// Computed measures
    pub measures: HashMap<String, EventValue>,

    /// Match start time
    pub start_time: SystemTime,

    /// Match end time
    pub end_time: SystemTime,
}

impl PatternMatch {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            measures: HashMap::new(),
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
        }
    }
}

/// Match context for pattern evaluation
pub struct MatchContext {
    /// Matched events so far
    pub matched: Vec<Event>,

    /// Variable bindings
    pub bindings: HashMap<String, Vec<Event>>,

    /// Pattern being matched
    pub pattern: Arc<Pattern>,
}

impl MatchContext {
    pub fn new(pattern: Arc<Pattern>) -> Self {
        Self {
            matched: Vec::new(),
            bindings: HashMap::new(),
            pattern,
        }
    }

    pub fn add_binding(&mut self, variable: String, event: Event) {
        self.bindings.entry(variable).or_insert_with(Vec::new).push(event);
    }

    pub fn get_binding(&self, variable: &str) -> Option<&Vec<Event>> {
        self.bindings.get(variable)
    }
}
