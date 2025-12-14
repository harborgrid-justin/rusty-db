/// Pattern Matching Module
///
/// Core pattern definitions and matching logic for Complex Event Processing (CEP).
use super::super::{Event, EventValue};
use super::temporal_operators::{
    MatchContext, Measure, PatternMatch, SkipStrategy, TemporalConstraint,
};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Pattern definition for complex event processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Pattern name
    pub name: String,

    /// Pattern specification
    pub spec: PatternSpec,

    /// Pattern variables
    pub variables: HashMap<String, PatternVariable>,

    /// Temporal constraints
    pub temporal: Option<TemporalConstraint>,

    /// Output measures
    pub measures: Vec<Measure>,

    /// Skip strategy for overlapping matches
    pub skip_strategy: SkipStrategy,
}

impl Pattern {
    pub fn new(name: impl Into<String>, spec: PatternSpec) -> Self {
        Self {
            name: name.into(),
            spec,
            variables: HashMap::new(),
            temporal: None,
            measures: Vec::new(),
            skip_strategy: SkipStrategy::PastLastRow,
        }
    }

    pub fn with_variable(mut self, name: impl Into<String>, var: PatternVariable) -> Self {
        self.variables.insert(name.into(), var);
        self
    }

    pub fn with_temporal(mut self, temporal: TemporalConstraint) -> Self {
        self.temporal = Some(temporal);
        self
    }

    pub fn with_measure(mut self, measure: Measure) -> Self {
        self.measures.push(measure);
        self
    }
}

/// Pattern specification (similar to MATCH_RECOGNIZE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternSpec {
    /// Match a sequence of patterns in order
    Sequence(Vec<PatternElement>),

    /// Match any of the patterns
    Any(Vec<PatternSpec>),

    /// Match all patterns (in any order)
    All(Vec<PatternSpec>),

    /// Match a single pattern element
    Element(PatternElement),

    /// Match pattern followed by another
    FollowedBy {
        first: Box<PatternSpec>,
        second: Box<PatternSpec>,
        strict: bool, // If true, no events in between
    },

    /// Repeat pattern
    Repeat {
        pattern: Box<PatternSpec>,
        min: usize,
        max: Option<usize>,
    },

    /// Optional pattern
    Optional(Box<PatternSpec>),

    /// Negation (pattern should not occur)
    Not(Box<PatternSpec>),
}

/// Pattern element representing a single event constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternElement {
    /// Variable name for this element
    pub variable: String,

    /// Condition to match
    pub condition: Condition,

    /// Quantifier
    pub quantifier: Quantifier,
}

impl PatternElement {
    pub fn new(variable: impl Into<String>, condition: Condition) -> Self {
        Self {
            variable: variable.into(),
            condition,
            quantifier: Quantifier::ExactlyOne,
        }
    }

    pub fn with_quantifier(mut self, quantifier: Quantifier) -> Self {
        self.quantifier = quantifier;
        self
    }
}

/// Quantifier for pattern elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Quantifier {
    /// Exactly one occurrence
    ExactlyOne,

    /// Zero or one occurrence
    ZeroOrOne,

    /// One or more occurrences
    OneOrMore,

    /// Zero or more occurrences
    ZeroOrMore,

    /// Specific range of occurrences
    Range { min: usize, max: Option<usize> },
}

/// Condition for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Event type matches
    EventType(String),

    /// Field equals value
    FieldEquals { field: String, value: EventValue },

    /// Field greater than value
    FieldGreaterThan { field: String, value: EventValue },

    /// Field less than value
    FieldLessThan { field: String, value: EventValue },

    /// Field matches regex
    FieldMatches { field: String, pattern: String },

    /// Field exists
    FieldExists(String),

    /// Compound AND condition
    And(Vec<Condition>),

    /// Compound OR condition
    Or(Vec<Condition>),

    /// NOT condition
    Not(Box<Condition>),

    /// Custom predicate
    Custom { name: String },
}

impl Condition {
    pub fn evaluate(&self, event: &Event, context: &MatchContext) -> bool {
        match self {
            Condition::EventType(event_type) => &event.event_type == event_type,

            Condition::FieldEquals { field, value } => {
                if let Some(event_value) = event.get_payload(field) {
                    event_value == value
                } else {
                    false
                }
            }

            Condition::FieldGreaterThan { field, value } => {
                if let Some(event_value) = event.get_payload(field) {
                    Self::compare_greater(event_value, value)
                } else {
                    false
                }
            }

            Condition::FieldLessThan { field, value } => {
                if let Some(event_value) = event.get_payload(field) {
                    Self::compare_less(event_value, value)
                } else {
                    false
                }
            }

            Condition::FieldMatches { field, pattern } => {
                if let Some(event_value) = event.get_payload(field) {
                    if let Some(s) = event_value.as_str() {
                        // Simplified regex matching
                        s.contains(pattern)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            Condition::FieldExists(field) => event.get_payload(field).is_some(),

            Condition::And(conditions) => conditions.iter().all(|c| c.evaluate(event, context)),

            Condition::Or(conditions) => conditions.iter().any(|c| c.evaluate(event, context)),

            Condition::Not(condition) => !condition.evaluate(event, context),

            Condition::Custom { .. } => false, // Custom predicates handled externally
        }
    }

    fn compare_greater(a: &EventValue, b: &EventValue) -> bool {
        match (a, b) {
            (EventValue::Int64(a), EventValue::Int64(b)) => a > b,
            (EventValue::Float64(a), EventValue::Float64(b)) => a > b,
            (EventValue::String(a), EventValue::String(b)) => a > b,
            _ => false,
        }
    }

    fn compare_less(a: &EventValue, b: &EventValue) -> bool {
        match (a, b) {
            (EventValue::Int64(a), EventValue::Int64(b)) => a < b,
            (EventValue::Float64(a), EventValue::Float64(b)) => a < b,
            (EventValue::String(a), EventValue::String(b)) => a < b,
            _ => false,
        }
    }
}

/// Pattern variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternVariable {
    /// Variable name
    pub name: String,

    /// Event type constraint
    pub event_type: Option<String>,

    /// Additional constraints
    pub constraints: Vec<Condition>,
}

impl PatternVariable {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            event_type: None,
            constraints: Vec::new(),
        }
    }

    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn with_constraint(mut self, constraint: Condition) -> Self {
        self.constraints.push(constraint);
        self
    }
}

/// Pattern matcher engine
pub struct PatternMatcher {
    /// Patterns to match
    patterns: Vec<Arc<Pattern>>,

    /// Event buffer for pattern matching
    buffer: VecDeque<Event>,

    /// Maximum buffer size
    max_buffer_size: usize,

    /// Active match contexts
    active_contexts: Vec<MatchContext>,

    /// GPU acceleration enabled
    gpu_enabled: bool,
}

impl PatternMatcher {
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            patterns: Vec::new(),
            buffer: VecDeque::new(),
            max_buffer_size,
            active_contexts: Vec::new(),
            gpu_enabled: false,
        }
    }

    pub fn with_gpu(mut self, enabled: bool) -> Self {
        self.gpu_enabled = enabled;
        self
    }

    /// Register a pattern
    pub fn register_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(Arc::new(pattern));
    }

    /// Process an event and find pattern matches
    pub fn process_event(&mut self, event: Event) -> Result<Vec<PatternMatch>> {
        // Add event to buffer
        self.buffer.push_back(event.clone());

        // Trim buffer if needed
        while self.buffer.len() > self.max_buffer_size {
            self.buffer.pop_front();
        }

        let mut matches = Vec::new();

        // Try to match each pattern
        for pattern in &self.patterns {
            if let Some(pattern_matches) = self.match_pattern(pattern)? {
                matches.extend(pattern_matches);
            }
        }

        Ok(matches)
    }

    fn match_pattern(&self, pattern: &Arc<Pattern>) -> Result<Option<Vec<PatternMatch>>> {
        if self.gpu_enabled {
            self.match_pattern_gpu(pattern)
        } else {
            self.match_pattern_cpu(pattern)
        }
    }

    fn match_pattern_cpu(&self, pattern: &Arc<Pattern>) -> Result<Option<Vec<PatternMatch>>> {
        let mut matches = Vec::new();

        // Simple sequential pattern matching
        let events: Vec<&Event> = self.buffer.iter().collect();

        for start_idx in 0..events.len() {
            let mut context = MatchContext::new(pattern.clone());

            if let Some(pattern_match) =
                self.try_match_from_index(&events, start_idx, &pattern.spec, &mut context)
            {
                // Check temporal constraints
                let matched_events: Vec<&Event> = pattern_match.events.values().flatten().collect();

                if let Some(temporal) = &pattern.temporal {
                    if !temporal.evaluate(&matched_events) {
                        continue;
                    }
                }

                matches.push(pattern_match);

                // Apply skip strategy
                match pattern.skip_strategy {
                    SkipStrategy::ToNextMatch | SkipStrategy::PastLastRow => break,
                    _ => {}
                }
            }
        }

        if matches.is_empty() {
            Ok(None)
        } else {
            Ok(Some(matches))
        }
    }

    fn try_match_from_index(
        &self,
        events: &[&Event],
        start_idx: usize,
        spec: &PatternSpec,
        context: &mut MatchContext,
    ) -> Option<PatternMatch> {
        use std::time::SystemTime;

        match spec {
            PatternSpec::Element(element) => {
                if start_idx >= events.len() {
                    return None;
                }

                let event = events[start_idx];
                if element.condition.evaluate(event, context) {
                    let mut pattern_match = PatternMatch::new();
                    pattern_match
                        .events
                        .insert(element.variable.clone(), vec![(*event).clone()]);
                    pattern_match.start_time = event.event_time;
                    pattern_match.end_time = event.event_time;

                    // Compute measures
                    for measure in &context.pattern.measures {
                        let value = measure.compute(&[event]);
                        pattern_match.measures.insert(measure.name.clone(), value);
                    }

                    Some(pattern_match)
                } else {
                    None
                }
            }

            PatternSpec::Sequence(elements) => {
                let mut current_idx = start_idx;
                let mut all_events = HashMap::new();
                let mut start_time = None;
                let mut end_time = None;

                for element in elements {
                    if current_idx >= events.len() {
                        return None;
                    }

                    let event = events[current_idx];
                    if element.condition.evaluate(event, context) {
                        all_events
                            .entry(element.variable.clone())
                            .or_insert_with(Vec::new)
                            .push((*event).clone());

                        if start_time.is_none() {
                            start_time = Some(event.event_time);
                        }
                        end_time = Some(event.event_time);

                        current_idx += 1;
                    } else {
                        return None;
                    }
                }

                let mut pattern_match = PatternMatch::new();
                pattern_match.events = all_events;
                pattern_match.start_time = start_time.unwrap_or_else(SystemTime::now);
                pattern_match.end_time = end_time.unwrap_or_else(SystemTime::now);

                Some(pattern_match)
            }

            _ => None, // Simplified for other pattern types
        }
    }

    fn match_pattern_gpu(&self, pattern: &Arc<Pattern>) -> Result<Option<Vec<PatternMatch>>> {
        // GPU-accelerated pattern matching would use CUDA/OpenCL here
        // For now, fall back to CPU implementation
        self.match_pattern_cpu(pattern)
    }

    /// Clear the event buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.active_contexts.clear();
    }

    /// Get buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }
}
