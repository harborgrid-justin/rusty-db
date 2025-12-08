// Windowing for Event Streams
//
// Implements various windowing strategies (tumbling, sliding, session, hopping)
// with watermark-based late arrival handling and custom triggers.

use super::{Event, EventValue, Watermark};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::{Duration, SystemTime};

/// Window identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WindowId(pub u64);

impl WindowId {
    pub fn new(id: u64) -> Self {
        WindowId(id)
    }

    pub fn from_timestamp(timestamp: SystemTime, window_size: Duration) -> Self {
        let duration_since_epoch = timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));
        let window_id = duration_since_epoch.as_millis() / window_size.as_millis();
        WindowId(window_id as u64)
    }
}

/// Window type specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowType {
    /// Tumbling window (fixed-size, non-overlapping)
    Tumbling {
        size: Duration,
    },

    /// Sliding window (fixed-size, overlapping)
    Sliding {
        size: Duration,
        slide: Duration,
    },

    /// Session window (dynamic size based on inactivity gap)
    Session {
        gap: Duration,
    },

    /// Hopping window (fixed-size with fixed hop)
    Hopping {
        size: Duration,
        hop: Duration,
    },

    /// Global window (all events in one window)
    Global,

    /// Custom window
    Custom {
        name: String,
    },
}

impl WindowType {
    /// Get the window ID(s) for an event
    pub fn get_windows(&self, event_time: SystemTime) -> Vec<Window> {
        match self {
            WindowType::Tumbling { size } => {
                let window_id = WindowId::from_timestamp(event_time, *size);
                let start = self.window_start_time(window_id, event_time);
                let end = start + *size;

                vec![Window {
                    id: window_id,
                    start,
                    end,
                    window_type: WindowType::Tumbling { size: *size },
                }]
            }

            WindowType::Sliding { size, slide } => {
                let mut windows = Vec::new();
                let duration_since_epoch = event_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0));

                // Calculate how many windows this event belongs to
                let num_windows = (size.as_millis() / slide.as_millis()).max(1);

                for i in 0..num_windows {
                    let offset = slide.as_millis() * i;
                    let window_start_ms = (duration_since_epoch.as_millis() / slide.as_millis())
                        * slide.as_millis()
                        - offset;

                    let start = SystemTime::UNIX_EPOCH
                        + Duration::from_millis(window_start_ms as u64);
                    let end = start + *size;

                    if event_time >= start && event_time < end {
                        let window_id = WindowId::new(window_start_ms as u64);
                        windows.push(Window {
                            id: window_id,
                            start,
                            end,
                            window_type: WindowType::Sliding {
                                size: *size,
                                slide: *slide,
                            },
                        });
                    }
                }

                windows
            }

            WindowType::Hopping { size, hop } => {
                let duration_since_epoch = event_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0));

                let window_start_ms =
                    (duration_since_epoch.as_millis() / hop.as_millis()) * hop.as_millis();
                let start = SystemTime::UNIX_EPOCH + Duration::from_millis(window_start_ms as u64);
                let end = start + *size;

                let window_id = WindowId::new(window_start_ms as u64);
                vec![Window {
                    id: window_id,
                    start,
                    end,
                    window_type: WindowType::Hopping {
                        size: *size,
                        hop: *hop,
                    },
                }]
            }

            WindowType::Session { gap: _ } => {
                // Session windows are created dynamically
                vec![]
            }

            WindowType::Global => {
                vec![Window {
                    id: WindowId::new(0),
                    start: SystemTime::UNIX_EPOCH,
                    end: SystemTime::UNIX_EPOCH + Duration::from_secs(u64::MAX),
                    window_type: WindowType::Global,
                }]
            }

            WindowType::Custom { .. } => vec![],
        }
    }

    fn window_start_time(&self, window_id: WindowId, event_time: SystemTime) -> SystemTime {
        match self {
            WindowType::Tumbling { size } => {
                let duration_since_epoch = event_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0));
                let window_start_ms =
                    (duration_since_epoch.as_millis() / size.as_millis()) * size.as_millis();
                SystemTime::UNIX_EPOCH + Duration::from_millis(window_start_ms as u64)
            }
            _ => event_time,
        }
    }
}

/// Window instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Window {
    /// Window ID
    pub id: WindowId,

    /// Window start time (inclusive)
    pub start: SystemTime,

    /// Window end time (exclusive)
    pub end: SystemTime,

    /// Window type
    pub window_type: WindowType,
}

impl Window {
    pub fn contains(&self, event_time: SystemTime) -> bool {
        event_time >= self.start && event_time < self.end
    }

    pub fn duration(&self) -> Duration {
        self.end.duration_since(self.start).unwrap_or(Duration::from_secs(0))
    }
}

/// Window state holding events
pub struct WindowState {
    /// Window metadata
    pub window: Window,

    /// Events in this window
    events: Vec<Event>,

    /// Aggregated state
    aggregates: HashMap<String, EventValue>,

    /// Window creation time
    created_at: SystemTime,

    /// Last update time
    updated_at: SystemTime,

    /// Whether the window has been triggered
    triggered: bool,

    /// Number of late events
    late_count: usize,
}

impl WindowState {
    pub fn new(window: Window) -> Self {
        let now = SystemTime::now();
        Self {
            window,
            events: Vec::new(),
            aggregates: HashMap::new(),
            created_at: now,
            updated_at: now,
            triggered: false,
            late_count: 0,
        }
    }

    /// Add an event to the window
    pub fn add_event(&mut self, event: Event, is_late: bool) {
        if is_late {
            self.late_count += 1;
        }
        self.events.push(event);
        self.updated_at = SystemTime::now();
    }

    /// Get all events in the window
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Set an aggregate value
    pub fn set_aggregate(&mut self, name: String, value: EventValue) {
        self.aggregates.insert(name, value);
    }

    /// Get an aggregate value
    pub fn get_aggregate(&self, name: &str) -> Option<&EventValue> {
        self.aggregates.get(name)
    }

    /// Mark window as triggered
    pub fn trigger(&mut self) {
        self.triggered = true;
    }

    /// Check if window is triggered
    pub fn is_triggered(&self) -> bool {
        self.triggered
    }

    /// Get late event count
    pub fn late_event_count(&self) -> usize {
        self.late_count
    }
}

/// Window trigger policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerPolicy {
    /// Trigger when window end time is reached
    OnTime,

    /// Trigger after every N events
    OnCount(usize),

    /// Trigger after specific duration
    OnInterval(Duration),

    /// Trigger on watermark passing window end
    OnWatermark,

    /// Trigger when specific condition is met
    OnCondition {
        description: String,
    },

    /// Composite trigger (any of the conditions)
    Any(Vec<TriggerPolicy>),

    /// Composite trigger (all conditions must be met)
    All(Vec<TriggerPolicy>),

    /// Never trigger (manual triggering only)
    Never,
}

impl TriggerPolicy {
    pub fn should_trigger(
        &self,
        window_state: &WindowState,
        watermark: Option<&Watermark>,
        current_time: SystemTime,
    ) -> bool {
        match self {
            TriggerPolicy::OnTime => current_time >= window_state.window.end,

            TriggerPolicy::OnCount(count) => window_state.event_count() >= *count,

            TriggerPolicy::OnInterval(interval) => {
                if let Ok(elapsed) = current_time.duration_since(window_state.created_at) {
                    elapsed >= *interval
                } else {
                    false
                }
            }

            TriggerPolicy::OnWatermark => {
                if let Some(wm) = watermark {
                    wm.timestamp >= window_state.window.end
                } else {
                    false
                }
            }

            TriggerPolicy::OnCondition { .. } => false, // Custom logic

            TriggerPolicy::Any(policies) => policies
                .iter()
                .any(|p| p.should_trigger(window_state, watermark, current_time)),

            TriggerPolicy::All(policies) => policies
                .iter()
                .all(|p| p.should_trigger(window_state, watermark, current_time)),

            TriggerPolicy::Never => false,
        }
    }
}

/// Eviction policy for window cleanup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Evict when window is triggered
    OnTrigger,

    /// Evict after watermark passes window end + grace period
    OnWatermark {
        grace_period: Duration,
    },

    /// Evict after specific time
    AfterTime(Duration),

    /// Never evict (keep all windows)
    Never,
}

impl EvictionPolicy {
    pub fn should_evict(
        &self,
        window_state: &WindowState,
        watermark: Option<&Watermark>,
        current_time: SystemTime,
    ) -> bool {
        match self {
            EvictionPolicy::OnTrigger => window_state.is_triggered(),

            EvictionPolicy::OnWatermark { grace_period } => {
                if let Some(wm) = watermark {
                    let eviction_time = window_state.window.end + *grace_period;
                    wm.timestamp >= eviction_time
                } else {
                    false
                }
            }

            EvictionPolicy::AfterTime(duration) => {
                if let Ok(elapsed) = current_time.duration_since(window_state.window.end) {
                    elapsed >= *duration
                } else {
                    false
                }
            }

            EvictionPolicy::Never => false,
        }
    }
}

/// Window manager
pub struct WindowManager {
    /// Window type
    window_type: WindowType,

    /// Trigger policy
    trigger_policy: TriggerPolicy,

    /// Eviction policy
    eviction_policy: EvictionPolicy,

    /// Active windows
    windows: BTreeMap<WindowId, WindowState>,

    /// Session windows (for session window type)
    session_windows: HashMap<String, SessionWindowState>,

    /// Current watermark
    watermark: Option<Watermark>,

    /// Allowed lateness
    allowed_lateness: Duration,
}

impl WindowManager {
    pub fn new(window_type: WindowType) -> Self {
        Self {
            window_type,
            trigger_policy: TriggerPolicy::OnWatermark,
            eviction_policy: EvictionPolicy::OnWatermark {
                grace_period: Duration::from_secs(60),
            },
            windows: BTreeMap::new(),
            session_windows: HashMap::new(),
            watermark: None,
            allowed_lateness: Duration::from_secs(60),
        }
    }

    pub fn with_trigger_policy(mut self, policy: TriggerPolicy) -> Self {
        self.trigger_policy = policy;
        self
    }

    pub fn with_eviction_policy(mut self, policy: EvictionPolicy) -> Self {
        self.eviction_policy = policy;
        self
    }

    pub fn with_allowed_lateness(mut self, lateness: Duration) -> Self {
        self.allowed_lateness = lateness;
        self
    }

    /// Add an event to the appropriate window(s)
    pub fn add_event(&mut self, event: Event) -> Result<Vec<WindowId>> {
        let mut assigned_windows = Vec::new();

        match &self.window_type {
            WindowType::Session { gap } => {
                let key = event
                    .partition_key
                    .clone()
                    .unwrap_or_else(|| event.id.to_string());
                assigned_windows.extend(self.add_to_session_window(event, key, *gap)?);
            }
            _ => {
                let windows = self.window_type.get_windows(event.event_time);

                for window in windows {
                    let is_late = self.is_late_event(&event);

                    let window_state = self
                        .windows
                        .entry(window.id)
                        .or_insert_with(|| WindowState::new(window.clone()));

                    window_state.add_event(event.clone(), is_late);
                    assigned_windows.push(window.id);
                }
            }
        }

        Ok(assigned_windows)
    }

    fn add_to_session_window(
        &mut self,
        event: Event,
        key: String,
        gap: Duration,
    ) -> Result<Vec<WindowId>> {
        let session_state = self
            .session_windows
            .entry(key)
            .or_insert_with(|| SessionWindowState::new(gap));

        Ok(session_state.add_event(event))
    }

    fn is_late_event(&self, event: &Event) -> bool {
        if let Some(watermark) = &self.watermark {
            watermark.is_late(event.event_time)
        } else {
            false
        }
    }

    /// Update watermark
    pub fn update_watermark(&mut self, watermark: Watermark) {
        self.watermark = Some(watermark);
    }

    /// Check for windows that should be triggered
    pub fn check_triggers(&mut self) -> Vec<WindowTriggerResult> {
        let mut results = Vec::new();
        let current_time = SystemTime::now();

        for (window_id, window_state) in &mut self.windows {
            if window_state.is_triggered() {
                continue;
            }

            if self.trigger_policy.should_trigger(
                window_state,
                self.watermark.as_ref(),
                current_time,
            ) {
                window_state.trigger();
                results.push(WindowTriggerResult {
                    window_id: *window_id,
                    window: window_state.window.clone(),
                    event_count: window_state.event_count(),
                    late_count: window_state.late_event_count(),
                });
            }
        }

        results
    }

    /// Evict old windows
    pub fn evict_old_windows(&mut self) -> Vec<WindowId> {
        let current_time = SystemTime::now();
        let mut to_evict = Vec::new();

        for (window_id, window_state) in &self.windows {
            if self.eviction_policy.should_evict(
                window_state,
                self.watermark.as_ref(),
                current_time,
            ) {
                to_evict.push(*window_id);
            }
        }

        for window_id in &to_evict {
            self.windows.remove(window_id);
        }

        to_evict
    }

    /// Get a window state
    pub fn get_window(&self, window_id: &WindowId) -> Option<&WindowState> {
        self.windows.get(window_id)
    }

    /// Get a mutable window state
    pub fn get_window_mut(&mut self, window_id: &WindowId) -> Option<&mut WindowState> {
        self.windows.get_mut(window_id)
    }

    /// Get all active windows
    pub fn active_windows(&self) -> Vec<&WindowState> {
        self.windows.values().collect()
    }

    /// Get count of active windows
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}

/// Session window state
struct SessionWindowState {
    gap: Duration,
    windows: Vec<SessionWindow>,
}

impl SessionWindowState {
    fn new(gap: Duration) -> Self {
        Self {
            gap,
            windows: Vec::new(),
        }
    }

    fn add_event(&mut self, event: Event) -> Vec<WindowId> {
        let event_time = event.event_time;

        // Find windows that can accept this event
        let mut merged_windows = Vec::new();
        let mut assigned_windows = Vec::new();

        for (i, window) in self.windows.iter().enumerate() {
            if let Ok(time_since_last) = event_time.duration_since(window.last_event_time) {
                if time_since_last <= self.gap {
                    merged_windows.push(i);
                }
            } else if let Ok(time_until_event) =
                window.last_event_time.duration_since(event_time)
            {
                if time_until_event <= self.gap {
                    merged_windows.push(i);
                }
            }
        }

        if merged_windows.is_empty() {
            // Create new session window
            let window_id = WindowId::new(
                event_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_millis() as u64,
            );
            let mut window = SessionWindow::new(window_id, event_time);
            window.add_event(event);
            assigned_windows.push(window_id);
            self.windows.push(window);
        } else if merged_windows.len() == 1 {
            // Add to existing window
            let window = &mut self.windows[merged_windows[0]];
            assigned_windows.push(window.id);
            window.add_event(event);
        } else {
            // Merge multiple windows
            let first_idx = merged_windows[0];
            let window_id = self.windows[first_idx].id;
            assigned_windows.push(window_id);

            for &idx in merged_windows.iter().rev() {
                if idx != first_idx {
                    let merged = self.windows.remove(idx);
                    self.windows[first_idx].merge(merged);
                }
            }

            self.windows[first_idx].add_event(event);
        }

        assigned_windows
    }
}

/// Session window
struct SessionWindow {
    id: WindowId,
    start_time: SystemTime,
    last_event_time: SystemTime,
    events: Vec<Event>,
}

impl SessionWindow {
    fn new(id: WindowId, start_time: SystemTime) -> Self {
        Self {
            id,
            start_time,
            last_event_time: start_time,
            events: Vec::new(),
        }
    }

    fn add_event(&mut self, event: Event) {
        if event.event_time > self.last_event_time {
            self.last_event_time = event.event_time;
        }
        self.events.push(event);
    }

    fn merge(&mut self, other: SessionWindow) {
        if other.start_time < self.start_time {
            self.start_time = other.start_time;
        }
        if other.last_event_time > self.last_event_time {
            self.last_event_time = other.last_event_time;
        }
        self.events.extend(other.events);
    }
}

/// Window trigger result
#[derive(Debug, Clone)]
pub struct WindowTriggerResult {
    pub window_id: WindowId,
    pub window: Window,
    pub event_count: usize,
    pub late_count: usize,
}

/// Windowed aggregator
pub struct WindowedAggregator<F>
where
    F: Fn(&[Event]) -> EventValue,
{
    window_manager: WindowManager,
    aggregate_fn: F,
    results: HashMap<WindowId, EventValue>,
}

impl<F> WindowedAggregator<F>
where
    F: Fn(&[Event]) -> EventValue,
{
    pub fn new(window_type: WindowType, aggregate_fn: F) -> Self {
        Self {
            window_manager: WindowManager::new(window_type),
            aggregate_fn,
            results: HashMap::new(),
        }
    }

    pub fn add_event(&mut self, event: Event) -> Result<()> {
        self.window_manager.add_event(event)?;
        Ok(())
    }

    pub fn update_watermark(&mut self, watermark: Watermark) {
        self.window_manager.update_watermark(watermark);
    }

    pub fn compute_aggregates(&mut self) -> Vec<(WindowId, EventValue)> {
        let mut results = Vec::new();

        for triggered in self.window_manager.check_triggers() {
            if let Some(window_state) = self.window_manager.get_window(&triggered.window_id) {
                let aggregate_value = (self.aggregate_fn)(window_state.events());
                self.results
                    .insert(triggered.window_id, aggregate_value.clone());
                results.push((triggered.window_id, aggregate_value));
            }
        }

        // Evict old windows
        let evicted = self.window_manager.evict_old_windows();
        for window_id in evicted {
            self.results.remove(&window_id);
        }

        results
    }

    pub fn get_result(&self, window_id: &WindowId) -> Option<&EventValue> {
        self.results.get(window_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tumbling_window() {
        let window_type = WindowType::Tumbling {
            size: Duration::from_secs(60),
        };
        let event = Event::new("test");
        let windows = window_type.get_windows(event.event_time);

        assert_eq!(windows.len(), 1);
        assert!(windows[0].contains(event.event_time));
    }

    #[test]
    fn test_sliding_window() {
        let window_type = WindowType::Sliding {
            size: Duration::from_secs(60),
            slide: Duration::from_secs(30),
        };
        let event = Event::new("test");
        let windows = window_type.get_windows(event.event_time);

        assert!(windows.len() >= 1);
    }

    #[test]
    fn test_window_manager() {
        let window_type = WindowType::Tumbling {
            size: Duration::from_secs(60),
        };
        let mut manager = WindowManager::new(window_type);

        let event = Event::new("test");
        let windows = manager.add_event(event).unwrap();

        assert!(!windows.is_empty());
        assert_eq!(manager.window_count(), 1);
    }

    #[test]
    fn test_trigger_policy() {
        let policy = TriggerPolicy::OnCount(10);
        let window = Window {
            id: WindowId::new(1),
            start: SystemTime::now(),
            end: SystemTime::now() + Duration::from_secs(60),
            window_type: WindowType::Global,
        };
        let mut state = WindowState::new(window);

        assert!(!policy.should_trigger(&state, None, SystemTime::now()));

        for _ in 0..10 {
            state.add_event(Event::new("test"), false);
        }

        assert!(policy.should_trigger(&state, None, SystemTime::now()));
    }

    #[test]
    fn test_windowed_aggregator() {
        let window_type = WindowType::Tumbling {
            size: Duration::from_secs(60),
        };
        let mut aggregator = WindowedAggregator::new(window_type, |events| {
            EventValue::Int64(events.len() as i64)
        });

        aggregator.add_event(Event::new("test")).unwrap();
        aggregator.add_event(Event::new("test")).unwrap();

        let results = aggregator.compute_aggregates();
        // Results depend on trigger policy and watermark
    }
}
