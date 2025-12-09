/// Filter Operators Module
///
/// Basic stream transformation operators: filter, map, and flatmap.

use super::pipeline::StreamOperator;
use super::super::Event;
use crate::error::Result;

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
