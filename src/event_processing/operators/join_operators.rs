/// Join Operators Module
///
/// Stream-to-stream join operators with windowing support.

use super::super::Event;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

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
    _join_type: JoinType,
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
            _join_type: join_type,
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
