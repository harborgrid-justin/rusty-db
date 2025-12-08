// Stream Processing Operators
//
// Implements functional operators for event stream transformations including
// filter, map, flatmap, aggregations, joins, deduplication, and TopN.

use super::{Event, EventBatch, EventValue};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Stream operator trait
pub trait StreamOperator: Send + Sync {
    fn process(&mut self, event: Event) -> Result<Vec<Event>>;
    fn name(&self) -> &str;
}

/// Filter operator
pub struct FilterOperator<F>
where
    F: Fn(&Event) -> bool + Send + Sync,
{
    name: String,
    predicate: F,
    filtered_count: u64,
    passed_count: u64,
}

impl<F> FilterOperator<F>
where
    F: Fn(&Event) -> bool + Send + Sync,
{
    pub fn new(name: impl Into<String>, predicate: F) -> Self {
        Self {
            name: name.into(),
            predicate,
            filtered_count: 0,
            passed_count: 0,
        }
    }

    pub fn stats(&self) -> (u64, u64) {
        (self.passed_count, self.filtered_count)
    }
}

impl<F> StreamOperator for FilterOperator<F>
where
    F: Fn(&Event) -> bool + Send + Sync,
{
    fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        if (self.predicate)(&event) {
            self.passed_count += 1;
            Ok(vec![event])
        } else {
            self.filtered_count += 1;
            Ok(vec![])
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Map operator
pub struct MapOperator<F>
where
    F: Fn(Event) -> Event + Send + Sync,
{
    name: String,
    transform: F,
    processed_count: u64,
}

impl<F> MapOperator<F>
where
    F: Fn(Event) -> Event + Send + Sync,
{
    pub fn new(name: impl Into<String>, transform: F) -> Self {
        Self {
            name: name.into(),
            transform,
            processed_count: 0,
        }
    }
}

impl<F> StreamOperator for MapOperator<F>
where
    F: Fn(Event) -> Event + Send + Sync,
{
    fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        self.processed_count += 1;
        Ok(vec![(self.transform)(event)])
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// FlatMap operator
pub struct FlatMapOperator<F>
where
    F: Fn(Event) -> Vec<Event> + Send + Sync,
{
    name: String,
    transform: F,
    input_count: u64,
    output_count: u64,
}

impl<F> FlatMapOperator<F>
where
    F: Fn(Event) -> Vec<Event> + Send + Sync,
{
    pub fn new(name: impl Into<String>, transform: F) -> Self {
        Self {
            name: name.into(),
            transform,
            input_count: 0,
            output_count: 0,
        }
    }

    pub fn expansion_ratio(&self) -> f64 {
        if self.input_count > 0 {
            self.output_count as f64 / self.input_count as f64
        } else {
            0.0
        }
    }
}

impl<F> StreamOperator for FlatMapOperator<F>
where
    F: Fn(Event) -> Vec<Event> + Send + Sync,
{
    fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        self.input_count += 1;
        let results = (self.transform)(event);
        self.output_count += results.len() as u64;
        Ok(results)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Aggregation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationType {
    Count,
    Sum(String),       // field name
    Avg(String),       // field name
    Min(String),       // field name
    Max(String),       // field name
    First(String),     // field name
    Last(String),      // field name
    Collect(String),   // field name
    CountDistinct(String), // field name
}

/// Aggregation operator
pub struct AggregationOperator {
    name: String,
    aggregation_type: AggregationType,
    group_by: Option<String>,
    state: HashMap<String, AggregationState>,
}

impl AggregationOperator {
    pub fn new(name: impl Into<String>, aggregation_type: AggregationType) -> Self {
        Self {
            name: name.into(),
            aggregation_type,
            group_by: None,
            state: HashMap::new(),
        }
    }

    pub fn with_group_by(mut self, field: impl Into<String>) -> Self {
        self.group_by = Some(field.into());
        self
    }

    fn get_group_key(&self, event: &Event) -> String {
        if let Some(field) = &self.group_by {
            event
                .get_payload(field)
                .and_then(|v| v.as_str())
                .unwrap_or("__default__")
                .to_string()
        } else {
            "__global__".to_string()
        }
    }

    pub fn get_result(&self, group_key: &str) -> Option<EventValue> {
        self.state.get(group_key).map(|s| s.get_value())
    }

    pub fn get_all_results(&self) -> HashMap<String, EventValue> {
        self.state
            .iter()
            .map(|(k, v)| (k.clone(), v.get_value()))
            .collect()
    }
}

impl StreamOperator for AggregationOperator {
    fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        let group_key = self.get_group_key(&event);

        let state = self
            .state
            .entry(group_key.clone())
            .or_insert_with(|| AggregationState::new(self.aggregation_type.clone()));

        state.update(&event);

        // Create output event with aggregation result
        let mut output = Event::new("aggregation.result");
        output = output.with_payload("group_key", group_key.clone());
        output = output.with_payload("value", state.get_value());
        output = output.with_source(self.name.clone());

        Ok(vec![output])
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Aggregation state
#[derive(Clone)]
struct AggregationState {
    aggregation_type: AggregationType,
    count: u64,
    sum: f64,
    min: Option<f64>,
    max: Option<f64>,
    first: Option<EventValue>,
    last: Option<EventValue>,
    values: Vec<EventValue>,
    distinct: HashSet<u64>,
}

impl AggregationState {
    fn new(aggregation_type: AggregationType) -> Self {
        Self {
            aggregation_type,
            count: 0,
            sum: 0.0,
            min: None,
            max: None,
            first: None,
            last: None,
            values: Vec::new(),
            distinct: HashSet::new(),
        }
    }

    fn update(&mut self, event: &Event) {
        self.count += 1;

        match &self.aggregation_type {
            AggregationType::Count => {}

            AggregationType::Sum(field) | AggregationType::Avg(field) => {
                if let Some(value) = event.get_payload(field) {
                    if let Some(num) = value.as_f64() {
                        self.sum += num;
                    }
                }
            }

            AggregationType::Min(field) => {
                if let Some(value) = event.get_payload(field) {
                    if let Some(num) = value.as_f64() {
                        self.min = Some(self.min.map_or(num, |m| m.min(num)));
                    }
                }
            }

            AggregationType::Max(field) => {
                if let Some(value) = event.get_payload(field) {
                    if let Some(num) = value.as_f64() {
                        self.max = Some(self.max.map_or(num, |m| m.max(num)));
                    }
                }
            }

            AggregationType::First(field) => {
                if self.first.is_none() {
                    if let Some(value) = event.get_payload(field) {
                        self.first = Some(value.clone());
                    }
                }
            }

            AggregationType::Last(field) => {
                if let Some(value) = event.get_payload(field) {
                    self.last = Some(value.clone());
                }
            }

            AggregationType::Collect(field) => {
                if let Some(value) = event.get_payload(field) {
                    self.values.push(value.clone());
                }
            }

            AggregationType::CountDistinct(field) => {
                if let Some(value) = event.get_payload(field) {
                    let hash = Self::hash_value(value);
                    self.distinct.insert(hash);
                }
            }
        }
    }

    fn get_value(&self) -> EventValue {
        match &self.aggregation_type {
            AggregationType::Count => EventValue::Int64(self.count as i64),

            AggregationType::Sum(_) => EventValue::Float64(self.sum),

            AggregationType::Avg(_) => {
                if self.count > 0 {
                    EventValue::Float64(self.sum / self.count as f64)
                } else {
                    EventValue::Null
                }
            }

            AggregationType::Min(_) => self.min.map(EventValue::Float64).unwrap_or(EventValue::Null),

            AggregationType::Max(_) => self.max.map(EventValue::Float64).unwrap_or(EventValue::Null),

            AggregationType::First(_) => self.first.clone().unwrap_or(EventValue::Null),

            AggregationType::Last(_) => self.last.clone().unwrap_or(EventValue::Null),

            AggregationType::Collect(_) => EventValue::Array(self.values.clone()),

            AggregationType::CountDistinct(_) => EventValue::Int64(self.distinct.len() as i64),
        }
    }

    fn hash_value(value: &EventValue) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();

        match value {
            EventValue::Null => 0.hash(&mut hasher),
            EventValue::Bool(b) => b.hash(&mut hasher),
            EventValue::Int64(i) => i.hash(&mut hasher),
            EventValue::Float64(f) => f.to_bits().hash(&mut hasher),
            EventValue::String(s) => s.hash(&mut hasher),
            EventValue::Bytes(b) => b.hash(&mut hasher),
            EventValue::Timestamp(t) => {
                if let Ok(duration) = t.duration_since(SystemTime::UNIX_EPOCH) {
                    duration.as_secs().hash(&mut hasher);
                }
            }
            _ => {}
        }

        hasher.finish()
    }
}

/// Join type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
}

/// Stream-stream join operator
pub struct StreamJoinOperator {
    name: String,
    join_type: JoinType,
    left_key: String,
    right_key: String,
    left_buffer: VecDeque<Event>,
    right_buffer: VecDeque<Event>,
    window_size: Duration,
    max_buffer_size: usize,
}

impl StreamJoinOperator {
    pub fn new(
        name: impl Into<String>,
        join_type: JoinType,
        left_key: impl Into<String>,
        right_key: impl Into<String>,
        window_size: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            join_type,
            left_key: left_key.into(),
            right_key: right_key.into(),
            left_buffer: VecDeque::new(),
            right_buffer: VecDeque::new(),
            window_size,
            max_buffer_size: 10000,
        }
    }

    pub fn process_left(&mut self, event: Event) -> Result<Vec<Event>> {
        self.cleanup_buffers();
        self.left_buffer.push_back(event.clone());

        let mut results = Vec::new();
        let left_key_value = event.get_payload(&self.left_key).cloned();

        if let Some(key) = left_key_value {
            for right_event in &self.right_buffer {
                if let Some(right_key_value) = right_event.get_payload(&self.right_key) {
                    if &key == right_key_value {
                        results.push(self.create_joined_event(&event, right_event));
                    }
                }
            }
        }

        Ok(results)
    }

    pub fn process_right(&mut self, event: Event) -> Result<Vec<Event>> {
        self.cleanup_buffers();
        self.right_buffer.push_back(event.clone());

        let mut results = Vec::new();
        let right_key_value = event.get_payload(&self.right_key).cloned();

        if let Some(key) = right_key_value {
            for left_event in &self.left_buffer {
                if let Some(left_key_value) = left_event.get_payload(&self.left_key) {
                    if &key == left_key_value {
                        results.push(self.create_joined_event(left_event, &event));
                    }
                }
            }
        }

        Ok(results)
    }

    fn create_joined_event(&self, left: &Event, right: &Event) -> Event {
        let mut joined = Event::new("join.result");

        // Add left event payload with "left_" prefix
        for (key, value) in &left.payload {
            joined = joined.with_payload(format!("left_{}", key), value.clone());
        }

        // Add right event payload with "right_" prefix
        for (key, value) in &right.payload {
            joined = joined.with_payload(format!("right_{}", key), value.clone());
        }

        joined = joined.with_source(self.name.clone());
        joined
    }

    fn cleanup_buffers(&mut self) {
        let now = SystemTime::now();

        self.left_buffer.retain(|e| {
            if let Ok(age) = now.duration_since(e.event_time) {
                age < self.window_size
            } else {
                false
            }
        });

        self.right_buffer.retain(|e| {
            if let Ok(age) = now.duration_since(e.event_time) {
                age < self.window_size
            } else {
                false
            }
        });

        // Also enforce max buffer size
        while self.left_buffer.len() > self.max_buffer_size {
            self.left_buffer.pop_front();
        }

        while self.right_buffer.len() > self.max_buffer_size {
            self.right_buffer.pop_front();
        }
    }
}

/// Deduplication operator
pub struct DeduplicationOperator {
    name: String,
    key_fields: Vec<String>,
    seen: HashMap<u64, SystemTime>,
    window_size: Duration,
    duplicate_count: u64,
    unique_count: u64,
}

impl DeduplicationOperator {
    pub fn new(name: impl Into<String>, key_fields: Vec<String>, window_size: Duration) -> Self {
        Self {
            name: name.into(),
            key_fields,
            seen: HashMap::new(),
            window_size,
            duplicate_count: 0,
            unique_count: 0,
        }
    }

    fn compute_key(&self, event: &Event) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        for field in &self.key_fields {
            if let Some(value) = event.get_payload(field) {
                match value {
                    EventValue::String(s) => s.hash(&mut hasher),
                    EventValue::Int64(i) => i.hash(&mut hasher),
                    EventValue::Bool(b) => b.hash(&mut hasher),
                    _ => {}
                }
            }
        }

        hasher.finish()
    }

    fn cleanup(&mut self) {
        let now = SystemTime::now();
        self.seen.retain(|_, timestamp| {
            if let Ok(age) = now.duration_since(*timestamp) {
                age < self.window_size
            } else {
                false
            }
        });
    }

    pub fn stats(&self) -> (u64, u64) {
        (self.unique_count, self.duplicate_count)
    }
}

impl StreamOperator for DeduplicationOperator {
    fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        self.cleanup();

        let key = self.compute_key(&event);

        if self.seen.contains_key(&key) {
            self.duplicate_count += 1;
            Ok(vec![]) // Duplicate, filter out
        } else {
            self.seen.insert(key, SystemTime::now());
            self.unique_count += 1;
            Ok(vec![event])
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// TopN operator
pub struct TopNOperator {
    name: String,
    n: usize,
    sort_field: String,
    descending: bool,
    top_events: BTreeMap<i64, Vec<Event>>, // score -> events
    total_processed: u64,
}

impl TopNOperator {
    pub fn new(name: impl Into<String>, n: usize, sort_field: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            n,
            sort_field: sort_field.into(),
            descending: true,
            top_events: BTreeMap::new(),
            total_processed: 0,
        }
    }

    pub fn ascending(mut self) -> Self {
        self.descending = false;
        self
    }

    pub fn get_top_n(&self) -> Vec<Event> {
        let mut results = Vec::new();

        if self.descending {
            for events in self.top_events.values().rev() {
                for event in events {
                    results.push(event.clone());
                    if results.len() >= self.n {
                        return results;
                    }
                }
            }
        } else {
            for events in self.top_events.values() {
                for event in events {
                    results.push(event.clone());
                    if results.len() >= self.n {
                        return results;
                    }
                }
            }
        }

        results
    }
}

impl StreamOperator for TopNOperator {
    fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        self.total_processed += 1;

        if let Some(value) = event.get_payload(&self.sort_field) {
            if let Some(score) = value.as_i64() {
                self.top_events
                    .entry(score)
                    .or_insert_with(Vec::new)
                    .push(event.clone());

                // Trim to top N
                let current_count: usize = self.top_events.values().map(|v| v.len()).sum();

                if current_count > self.n * 2 {
                    // Keep some buffer
                    if self.descending {
                        let keys_to_remove: Vec<i64> = self
                            .top_events
                            .keys()
                            .take(self.top_events.len() / 2)
                            .copied()
                            .collect();

                        for key in keys_to_remove {
                            self.top_events.remove(&key);
                        }
                    } else {
                        let keys_to_remove: Vec<i64> = self
                            .top_events
                            .keys()
                            .rev()
                            .take(self.top_events.len() / 2)
                            .copied()
                            .collect();

                        for key in keys_to_remove {
                            self.top_events.remove(&key);
                        }
                    }
                }
            }
        }

        Ok(vec![event])
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Union operator
pub struct UnionOperator {
    name: String,
    stream_count: usize,
}

impl UnionOperator {
    pub fn new(name: impl Into<String>, stream_count: usize) -> Self {
        Self {
            name: name.into(),
            stream_count,
        }
    }
}

impl StreamOperator for UnionOperator {
    fn process(&mut self, event: Event) -> Result<Vec<Event>> {
        Ok(vec![event])
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Operator pipeline
pub struct OperatorPipeline {
    name: String,
    operators: Vec<Box<dyn StreamOperator>>,
    metrics: PipelineMetrics,
}

impl OperatorPipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
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
    fn record_processing(&mut self, latency: Duration) {
        let latency_ms = latency.as_millis() as u64;
        self.total_latency_ms += latency_ms;
        self.avg_latency_ms = self.total_latency_ms as f64 / self.events_processed.max(1) as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(results[0].get_payload("doubled").unwrap().as_i64(), Some(20));
    }
}
