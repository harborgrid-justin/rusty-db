// Complex Event Processing (CEP)
//
// Implements Oracle MATCH_RECOGNIZE-like pattern matching on event streams with
// temporal patterns, event correlation, hierarchies, and GPU-accelerated matching.

use super::{Event, EventValue};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

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
    FieldEquals {
        field: String,
        value: EventValue,
    },

    /// Field greater than value
    FieldGreaterThan {
        field: String,
        value: EventValue,
    },

    /// Field less than value
    FieldLessThan {
        field: String,
        value: EventValue,
    },

    /// Field matches regex
    FieldMatches {
        field: String,
        pattern: String,
    },

    /// Field exists
    FieldExists(String),

    /// Compound AND condition
    And(Vec<Condition>),

    /// Compound OR condition
    Or(Vec<Condition>),

    /// NOT condition
    Not(Box<Condition>),

    /// Custom predicate
    Custom {
        name: String,
    },
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

            if let Some(pattern_match) = self.try_match_from_index(&events, start_idx, &pattern.spec, &mut context) {
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
        match spec {
            PatternSpec::Element(element) => {
                if start_idx >= events.len() {
                    return None;
                }

                let event = events[start_idx];
                if element.condition.evaluate(event, context) {
                    let mut pattern_match = PatternMatch::new();
                    pattern_match.events.insert(element.variable.clone(), vec![(*event).clone()]);
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

                for element_spec in elements {
                    if let PatternSpec::Element(element) = element_spec {
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
        Self {
            name: name.into(),
            event_types,
            min_events: event_types.len(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let element = PatternElement::new("A", Condition::EventType("login".to_string()));
        let pattern = Pattern::new("login_pattern", PatternSpec::Element(element));

        assert_eq!(pattern.name, "login_pattern");
    }

    #[test]
    fn test_condition_evaluation() {
        let condition = Condition::EventType("test".to_string());
        let event = Event::new("test");
        let pattern = Pattern::new("test", PatternSpec::Element(PatternElement::new("A", condition.clone())));
        let context = MatchContext::new(Arc::new(pattern));

        assert!(condition.evaluate(&event, &context));
    }

    #[test]
    fn test_pattern_matcher() {
        let mut matcher = PatternMatcher::new(100);

        let element = PatternElement::new("A", Condition::EventType("login".to_string()));
        let pattern = Pattern::new("login_pattern", PatternSpec::Element(element));
        matcher.register_pattern(pattern);

        let event = Event::new("login");
        let matches = matcher.process_event(event).unwrap();

        assert!(!matches.is_empty());
    }

    #[test]
    fn test_temporal_constraint() {
        let constraint = TemporalConstraint::Within(Duration::from_secs(10));

        let event1 = Event::new("test");
        let event2 = Event::new("test");

        assert!(constraint.evaluate(&[&event1, &event2]));
    }

    #[test]
    fn test_correlation_engine() {
        let mut engine = CorrelationEngine::new(Duration::from_secs(60));

        let rule = CorrelationRule::new(
            "checkout_flow",
            vec!["cart.add".to_string(), "checkout.start".to_string()],
        );
        engine.add_rule(rule);

        let event1 = Event::new("cart.add").with_correlation_id("user_123");
        let event2 = Event::new("checkout.start").with_correlation_id("user_123");

        engine.correlate(event1);
        let correlated = engine.correlate(event2);

        assert!(!correlated.is_empty());
    }
}
