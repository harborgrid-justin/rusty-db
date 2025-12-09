/// Event Correlation Module
///
/// Event correlation engine for detecting related events across streams.

use super::super::Event;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Event hierarchy for hierarchical pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHierarchy {
    /// Parent event type
    pub parent: String,

    /// Child event types
    pub children: Vec<String>,

    /// Hierarchy level
    pub level: usize,
}

impl EventHierarchy {
    pub fn new(parent: impl Into<String>) -> Self {
        Self {
            parent: parent.into(),
            children: Vec::new(),
            level: 0,
        }
    }

    pub fn add_child(&mut self, child: impl Into<String>) {
        self.children.push(child.into());
    }

    pub fn is_descendant(&self, event_type: &str) -> bool {
        self.children.iter().any(|c| c == event_type)
    }
}

/// Event correlation engine
pub struct CorrelationEngine {
    /// Correlation rules
    rules: Vec<CorrelationRule>,

    /// Correlation buffer
    buffer: HashMap<String, Vec<Event>>,

    /// Correlation window
    window: Duration,
}

impl CorrelationEngine {
    pub fn new(window: Duration) -> Self {
        Self {
            rules: Vec::new(),
            buffer: HashMap::new(),
            window,
        }
    }

    pub fn add_rule(&mut self, rule: CorrelationRule) {
        self.rules.push(rule);
    }

    pub fn correlate(&mut self, event: Event) -> Vec<CorrelatedEvent> {
        let correlation_key = event.correlation_id.clone().unwrap_or_else(|| event.id.to_string());

        // Add to buffer
        self.buffer
            .entry(correlation_key.clone())
            .or_insert_with(Vec::new)
            .push(event.clone());

        // Check correlation rules
        let mut correlated = Vec::new();

        for rule in &self.rules {
            if let Some(events) = self.buffer.get(&correlation_key) {
                if rule.matches(events) {
                    correlated.push(CorrelatedEvent {
                        correlation_id: correlation_key.clone(),
                        events: events.clone(),
                        rule_name: rule.name.clone(),
                    });
                }
            }
        }

        // Clean old events from buffer
        self.cleanup_buffer();

        correlated
    }

    fn cleanup_buffer(&mut self) {
        let now = SystemTime::now();
        self.buffer.retain(|_, events| {
            events.retain(|e| {
                if let Ok(age) = now.duration_since(e.event_time) {
                    age < self.window
                } else {
                    false
                }
            });
            !events.is_empty()
        });
    }
}

/// Correlation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationRule {
    pub name: String,
    pub event_types: Vec<String>,
    pub min_events: usize,
}

impl CorrelationRule {
    pub fn new(name: impl Into<String>, event_types: Vec<String>) -> Self {
        let min_events = event_types.len();
        Self {
            name: name.into(),
            event_types,
            min_events,
        }
    }

    pub fn matches(&self, events: &[Event]) -> bool {
        if events.len() < self.min_events {
            return false;
        }

        for event_type in &self.event_types {
            if !events.iter().any(|e| &e.event_type == event_type) {
                return false;
            }
        }

        true
    }
}

/// Correlated event result
#[derive(Debug, Clone)]
pub struct CorrelatedEvent {
    pub correlation_id: String,
    pub events: Vec<Event>,
    pub rule_name: String,
}
