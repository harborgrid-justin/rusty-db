use super::super::Event;
/// Specialized Operators Module
///
/// Specialized stream operators: deduplication, TopN, and union.
use super::pipeline::StreamOperator;
use crate::error::Result;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};

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
        use super::super::EventValue;
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
    _stream_count: usize,
}

impl UnionOperator {
    pub fn new(name: impl Into<String>, stream_count: usize) -> Self {
        Self {
            name: name.into(),
            _stream_count: stream_count,
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
