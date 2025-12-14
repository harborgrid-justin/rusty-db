pub mod event_correlation;
pub mod nfa_matcher;
/// Complex Event Processing (CEP) Module
///
/// Implements Oracle MATCH_RECOGNIZE-like pattern matching on event streams with
/// temporal patterns, event correlation, hierarchies, and NFA-accelerated matching.
///
/// This module is organized into submodules:
/// - `pattern_matching`: Core pattern definitions and matching logic
/// - `event_correlation`: Event correlation engine
/// - `temporal_operators`: Time-based constraints and measurements
/// - `nfa_matcher`: NFA-based optimized pattern matching
///
/// Public API re-exports all necessary types to maintain compatibility.
pub mod pattern_matching;
pub mod temporal_operators;

// Re-export all public types for backward compatibility
pub use pattern_matching::{
    Condition, Pattern, PatternElement, PatternMatcher, PatternSpec, PatternVariable, Quantifier,
};

pub use event_correlation::{CorrelatedEvent, CorrelationEngine, CorrelationRule, EventHierarchy};

pub use temporal_operators::{
    Aggregation, MatchContext, Measure, PatternMatch, SkipStrategy, TemporalConstraint,
};

pub use nfa_matcher::{NFAPatternMatcher, NFA};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_processing::Event;
    use std::sync::Arc;
    use std::time::Duration;

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
        let pattern = Pattern::new(
            "test",
            PatternSpec::Element(PatternElement::new("A", condition.clone())),
        );
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
