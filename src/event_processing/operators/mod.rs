pub mod aggregate_operators;
pub mod approximate;
pub mod filter_operators;
pub mod join_operators;
/// Stream Processing Operators Module
///
/// Implements functional operators for event stream transformations including
/// filter, map, flatmap, aggregations, joins, deduplication, and TopN.
///
/// This module is organized into submodules:
/// - `pipeline`: Core operator infrastructure and pipeline management
/// - `filter_operators`: Basic transformation operators (filter, map, flatmap)
/// - `aggregate_operators`: Aggregation operators (exact and approximate)
/// - `join_operators`: Stream-to-stream join operators
/// - `specialized_operators`: Deduplication, TopN, union operators
/// - `approximate`: Approximate streaming algorithms (HyperLogLog, Count-Min Sketch)
///
/// Public API re-exports all necessary types to maintain compatibility.
pub mod pipeline;
pub mod specialized_operators;

// Re-export all public types for backward compatibility
pub use pipeline::{OperatorPipeline, PipelineMetrics, StreamOperator};

pub use filter_operators::{FilterOperator, FlatMapOperator, MapOperator};

pub use aggregate_operators::{
    AggregationOperator, AggregationType, ApproximateDistinctOperator, ApproximateTopKOperator,
};

pub use join_operators::{JoinType, StreamJoinOperator};

pub use specialized_operators::{DeduplicationOperator, TopNOperator, UnionOperator};

pub use approximate::{CountMinSketch, HeavyHitters, HyperLogLog};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_processing::Event;
    use std::time::Duration;

    #[test]
    fn test_filter_operator() {
        let mut filter = FilterOperator::new("test_filter", |e: &Event| {
            e.get_payload("value")
                .and_then(|v| v.as_i64())
                .map(|v| v > 10)
                .unwrap_or(false)
        });

        let event1 = Event::new("test").with_payload("value", 5i64);
        let event2 = Event::new("test").with_payload("value", 15i64);

        assert!(filter.process(event1).unwrap().is_empty());
        assert_eq!(filter.process(event2).unwrap().len(), 1);

        let (passed, filtered) = filter.stats();
        assert_eq!(passed, 1);
        assert_eq!(filtered, 1);
    }

    #[test]
    fn test_map_operator() {
        let mut map = MapOperator::new("test_map", |mut e: Event| {
            e.event_type = format!("{}_mapped", e.event_type);
            e
        });

        let event = Event::new("test");
        let results = map.process(event).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_type, "test_mapped");
    }

    #[test]
    fn test_aggregation_operator() {
        let mut agg = AggregationOperator::new("test_agg", AggregationType::Count);

        for i in 0..10 {
            let event = Event::new("test").with_payload("value", i as i64);
            agg.process(event).unwrap();
        }

        let result = agg.get_result("__global__").unwrap();
        assert_eq!(result.as_i64(), Some(10));
    }

    #[test]
    fn test_deduplication_operator() {
        let mut dedup = DeduplicationOperator::new(
            "test_dedup",
            vec!["id".to_string()],
            Duration::from_secs(60),
        );

        let event1 = Event::new("test").with_payload("id", "123");
        let event2 = Event::new("test").with_payload("id", "123");
        let event3 = Event::new("test").with_payload("id", "456");

        assert_eq!(dedup.process(event1).unwrap().len(), 1);
        assert_eq!(dedup.process(event2).unwrap().len(), 0); // Duplicate
        assert_eq!(dedup.process(event3).unwrap().len(), 1);

        let (unique, duplicates) = dedup.stats();
        assert_eq!(unique, 2);
        assert_eq!(duplicates, 1);
    }

    #[test]
    fn test_topn_operator() {
        let mut topn = TopNOperator::new("test_topn", 3, "score");

        for i in 0..10 {
            let event = Event::new("test").with_payload("score", i as i64);
            topn.process(event).unwrap();
        }

        let top3 = topn.get_top_n();
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0].get_payload("score").unwrap().as_i64(), Some(9));
    }

    #[test]
    fn test_operator_pipeline() {
        let mut pipeline = OperatorPipeline::new("test_pipeline");

        pipeline.add_operator(Box::new(FilterOperator::new("filter", |e: &Event| {
            e.get_payload("value")
                .and_then(|v| v.as_i64())
                .map(|v| v > 5)
                .unwrap_or(false)
        })));

        pipeline.add_operator(Box::new(MapOperator::new("map", |mut e: Event| {
            if let Some(value) = e.get_payload("value").and_then(|v| v.as_i64()) {
                e = e.with_payload("doubled", value * 2);
            }
            e
        })));

        let event = Event::new("test").with_payload("value", 10i64);
        let results = pipeline.process(event).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].get_payload("doubled").unwrap().as_i64(),
            Some(20)
        );
    }
}
