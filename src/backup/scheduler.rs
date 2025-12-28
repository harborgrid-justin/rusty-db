// Backup Scheduler - Enterprise-grade backup scheduling with cron-like syntax
// Automated backup orchestration with retention policy enforcement

use crate::error::DbError;
use crate::Result;
use chrono::{Datelike, Timelike};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{BackupManager, BackupType, RetentionPolicy};

// Cron-like schedule expression parser
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CronSchedule {
    pub minute: CronField,      // 0-59
    pub hour: CronField,        // 0-23
    pub day_of_month: CronField, // 1-31
    pub month: CronField,       // 1-12
    pub day_of_week: CronField, // 0-6 (Sunday = 0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CronField {
    Any,                          // *
    Specific(u32),                // 5
    Range(u32, u32),              // 5-10
    List(Vec<u32>),               // 1,5,10
    Step(Box<CronField>, u32),    // */5 or 0-30/5
}

impl CronSchedule {
    // Parse cron expression (minute hour day month weekday)
    pub fn parse(expr: &str) -> Result<Self> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(DbError::InvalidInput(
                "Cron expression must have 5 fields".to_string(),
            ));
        }

        Ok(Self {
            minute: Self::parse_field(parts[0], 0, 59)?,
            hour: Self::parse_field(parts[1], 0, 23)?,
            day_of_month: Self::parse_field(parts[2], 1, 31)?,
            month: Self::parse_field(parts[3], 1, 12)?,
            day_of_week: Self::parse_field(parts[4], 0, 6)?,
        })
    }

    fn parse_field(field: &str, min: u32, max: u32) -> Result<CronField> {
        if field == "*" {
            return Ok(CronField::Any);
        }

        // Handle step expressions (*/5 or 0-30/5)
        if let Some((base, step)) = field.split_once('/') {
            let step_val = step.parse::<u32>().map_err(|_| {
                DbError::InvalidInput(format!("Invalid step value: {}", step))
            })?;
            let base_field = Self::parse_field(base, min, max)?;
            return Ok(CronField::Step(Box::new(base_field), step_val));
        }

        // Handle ranges (5-10)
        if let Some((start, end)) = field.split_once('-') {
            let start_val = start.parse::<u32>().map_err(|_| {
                DbError::InvalidInput(format!("Invalid range start: {}", start))
            })?;
            let end_val = end.parse::<u32>().map_err(|_| {
                DbError::InvalidInput(format!("Invalid range end: {}", end))
            })?;
            if start_val < min || end_val > max {
                return Err(DbError::InvalidInput(format!(
                    "Range out of bounds: {}-{}",
                    start_val, end_val
                )));
            }
            return Ok(CronField::Range(start_val, end_val));
        }

        // Handle lists (1,5,10)
        if field.contains(',') {
            let values: Result<Vec<u32>> = field
                .split(',')
                .map(|v| {
                    v.parse::<u32>()
                        .map_err(|_| DbError::InvalidInput(format!("Invalid list value: {}", v)))
                })
                .collect();
            return Ok(CronField::List(values?));
        }

        // Single value
        let value = field.parse::<u32>().map_err(|_| {
            DbError::InvalidInput(format!("Invalid field value: {}", field))
        })?;
        if value < min || value > max {
            return Err(DbError::InvalidInput(format!(
                "Value out of bounds: {}",
                value
            )));
        }
        Ok(CronField::Specific(value))
    }

    // Check if a value matches the cron field
    fn matches_field(field: &CronField, value: u32, min: u32, max: u32) -> bool {
        match field {
            CronField::Any => true,
            CronField::Specific(v) => *v == value,
            CronField::Range(start, end) => value >= *start && value <= *end,
            CronField::List(values) => values.contains(&value),
            CronField::Step(base, step) => {
                if !Self::matches_field(base, value, min, max) {
                    return false;
                }
                match base.as_ref() {
                    CronField::Any => (value - min) % step == 0,
                    CronField::Range(start, _) => (value - start) % step == 0,
                    _ => value % step == 0,
                }
            }
        }
    }

    // Check if the schedule matches a given timestamp
    pub fn matches(&self, time: SystemTime) -> bool {
        let datetime = chrono::DateTime::<chrono::Utc>::from(time);

        Self::matches_field(&self.minute, datetime.minute(), 0, 59)
            && Self::matches_field(&self.hour, datetime.hour(), 0, 23)
            && Self::matches_field(&self.day_of_month, datetime.day(), 1, 31)
            && Self::matches_field(&self.month, datetime.month(), 1, 12)
            && Self::matches_field(&self.day_of_week, datetime.weekday().num_days_from_sunday(), 0, 6)
    }

    // Calculate next execution time
    pub fn next_execution(&self, after: SystemTime) -> Option<SystemTime> {
        // Simple implementation: check next 365 days, minute by minute
        let mut current = after;
        for _ in 0..(365 * 24 * 60) {
            current = current + Duration::from_secs(60);
            if self.matches(current) {
                return Some(current);
            }
        }
        None
    }

    // Common schedule presets
    pub fn hourly() -> Self {
        Self::parse("0 * * * *").unwrap()
    }

    pub fn daily(hour: u32, minute: u32) -> Self {
        Self::parse(&format!("{} {} * * *", minute, hour)).unwrap()
    }

    pub fn weekly(day: u32, hour: u32, minute: u32) -> Self {
        Self::parse(&format!("{} {} * * {}", minute, hour, day)).unwrap()
    }

    pub fn monthly(day: u32, hour: u32, minute: u32) -> Self {
        Self::parse(&format!("{} {} {} * *", minute, hour, day)).unwrap()
    }
}

// Backup window - time period when backups are allowed to run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupWindow {
    pub window_id: String,
    pub name: String,
    pub start_hour: u8,   // 0-23
    pub start_minute: u8, // 0-59
    pub end_hour: u8,     // 0-23
    pub end_minute: u8,   // 0-59
    pub days_of_week: HashSet<u8>, // 0-6 (Sunday = 0)
    pub enabled: bool,
}

impl BackupWindow {
    pub fn new(window_id: String, name: String) -> Self {
        Self {
            window_id,
            name,
            start_hour: 0,
            start_minute: 0,
            end_hour: 23,
            end_minute: 59,
            days_of_week: (0..7).collect(),
            enabled: true,
        }
    }

    // Check if current time is within the backup window
    pub fn is_active(&self, time: SystemTime) -> bool {
        if !self.enabled {
            return false;
        }

        let datetime = chrono::DateTime::<chrono::Utc>::from(time);
        let weekday = datetime.weekday().num_days_from_sunday() as u8;

        if !self.days_of_week.contains(&weekday) {
            return false;
        }

        let current_minutes = datetime.hour() as u16 * 60 + datetime.minute() as u16;
        let start_minutes = self.start_hour as u16 * 60 + self.start_minute as u16;
        let end_minutes = self.end_hour as u16 * 60 + self.end_minute as u16;

        if start_minutes <= end_minutes {
            current_minutes >= start_minutes && current_minutes <= end_minutes
        } else {
            // Window crosses midnight
            current_minutes >= start_minutes || current_minutes <= end_minutes
        }
    }

    // Business hours window (Mon-Fri 9am-5pm)
    pub fn business_hours() -> Self {
        let mut window = Self::new("business".to_string(), "Business Hours".to_string());
        window.start_hour = 9;
        window.end_hour = 17;
        window.days_of_week = vec![1, 2, 3, 4, 5].into_iter().collect(); // Mon-Fri
        window
    }

    // Off-hours window (Mon-Fri 6pm-6am, all weekend)
    pub fn off_hours() -> Self {
        let mut window = Self::new("offhours".to_string(), "Off Hours".to_string());
        window.start_hour = 18;
        window.end_hour = 6;
        window.days_of_week = vec![1, 2, 3, 4, 5].into_iter().collect(); // Mon-Fri
        window
    }
}

// Scheduled backup job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledBackup {
    pub job_id: String,
    pub name: String,
    pub database_name: String,
    pub backup_type: BackupType,
    pub schedule: CronSchedule,
    pub window_id: Option<String>,
    pub retention_policy: RetentionPolicy,
    pub enabled: bool,
    pub last_execution: Option<SystemTime>,
    pub next_execution: Option<SystemTime>,
    pub max_duration: Option<Duration>,
    pub priority: u8, // 0-255, higher = more important
    pub tags: HashMap<String, String>,
}

impl ScheduledBackup {
    pub fn new(
        job_id: String,
        name: String,
        database_name: String,
        backup_type: BackupType,
        schedule: CronSchedule,
    ) -> Self {
        let next_execution = schedule.next_execution(SystemTime::now());
        Self {
            job_id,
            name,
            database_name,
            backup_type,
            schedule,
            window_id: None,
            retention_policy: RetentionPolicy::default(),
            enabled: true,
            last_execution: None,
            next_execution,
            max_duration: Some(Duration::from_secs(3600)), // 1 hour default
            priority: 100,
            tags: HashMap::new(),
        }
    }

    pub fn is_due(&self, current_time: SystemTime) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(next) = self.next_execution {
            current_time >= next
        } else {
            false
        }
    }

    pub fn calculate_next_execution(&mut self) {
        let after = self.last_execution.unwrap_or_else(SystemTime::now);
        self.next_execution = self.schedule.next_execution(after);
    }
}

// Backup execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupExecution {
    pub execution_id: String,
    pub job_id: String,
    pub backup_id: Option<String>,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

// Backup scheduler managing all scheduled jobs
pub struct BackupScheduler {
    jobs: Arc<RwLock<HashMap<String, ScheduledBackup>>>,
    windows: Arc<RwLock<HashMap<String, BackupWindow>>>,
    execution_history: Arc<RwLock<BTreeMap<SystemTime, BackupExecution>>>,
    backup_manager: Arc<BackupManager>,
    config_path: PathBuf,
    enabled: Arc<Mutex<bool>>,
}

impl BackupScheduler {
    pub fn new(backup_manager: Arc<BackupManager>, config_path: PathBuf) -> Result<Self> {
        create_dir_all(&config_path)?;

        let scheduler = Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            windows: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(BTreeMap::new())),
            backup_manager,
            config_path,
            enabled: Arc::new(Mutex::new(true)),
        };

        // Load persisted configuration
        scheduler.load_config()?;

        Ok(scheduler)
    }

    // Add or update a scheduled backup job
    pub fn add_job(&self, job: ScheduledBackup) -> Result<()> {
        let job_id = job.job_id.clone();
        self.jobs.write().insert(job_id.clone(), job);
        self.save_config()?;
        Ok(())
    }

    // Remove a scheduled backup job
    pub fn remove_job(&self, job_id: &str) -> Result<()> {
        self.jobs
            .write()
            .remove(job_id)
            .ok_or_else(|| DbError::NotFound(format!("Job not found: {}", job_id)))?;
        self.save_config()?;
        Ok(())
    }

    // Get a scheduled backup job
    pub fn get_job(&self, job_id: &str) -> Option<ScheduledBackup> {
        self.jobs.read().get(job_id).cloned()
    }

    // List all scheduled jobs
    pub fn list_jobs(&self) -> Vec<ScheduledBackup> {
        self.jobs.read().values().cloned().collect()
    }

    // Add or update a backup window
    pub fn add_window(&self, window: BackupWindow) -> Result<()> {
        let window_id = window.window_id.clone();
        self.windows.write().insert(window_id, window);
        self.save_config()?;
        Ok(())
    }

    // Remove a backup window
    pub fn remove_window(&self, window_id: &str) -> Result<()> {
        self.windows
            .write()
            .remove(window_id)
            .ok_or_else(|| DbError::NotFound(format!("Window not found: {}", window_id)))?;
        self.save_config()?;
        Ok(())
    }

    // Execute all due scheduled backups
    pub fn execute_due_jobs(&self) -> Result<Vec<String>> {
        if !*self.enabled.lock() {
            return Ok(Vec::new());
        }

        let current_time = SystemTime::now();
        let mut executed_jobs = Vec::new();
        let mut jobs = self.jobs.write();
        let windows = self.windows.read();

        // Find all jobs that are due
        let mut due_jobs: Vec<_> = jobs
            .values()
            .filter(|job| job.is_due(current_time))
            .cloned()
            .collect();

        // Sort by priority (higher first)
        due_jobs.sort_by(|a, b| b.priority.cmp(&a.priority));

        for job in due_jobs {
            // Check backup window
            if let Some(window_id) = &job.window_id {
                if let Some(window) = windows.get(window_id) {
                    if !window.is_active(current_time) {
                        // Skip this execution, window is not active
                        continue;
                    }
                }
            }

            // Execute the backup
            match self.execute_job(&job) {
                Ok(backup_id) => {
                    // Update job state
                    if let Some(job_mut) = jobs.get_mut(&job.job_id) {
                        job_mut.last_execution = Some(current_time);
                        job_mut.calculate_next_execution();
                    }
                    executed_jobs.push(backup_id);
                }
                Err(e) => {
                    // Log error but continue with other jobs
                    eprintln!("Failed to execute backup job {}: {}", job.job_id, e);
                }
            }
        }

        self.save_config()?;
        Ok(executed_jobs)
    }

    fn execute_job(&self, job: &ScheduledBackup) -> Result<String> {
        let execution_id = format!(
            "exec-{}-{}",
            job.job_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let start_time = SystemTime::now();

        // Create execution record
        let mut execution = BackupExecution {
            execution_id: execution_id.clone(),
            job_id: job.job_id.clone(),
            backup_id: None,
            start_time,
            end_time: None,
            status: ExecutionStatus::Running,
            error_message: None,
            duration: None,
        };

        // Execute backup based on type
        let backup_result = match job.backup_type {
            BackupType::Full => self.backup_manager.create_full_backup(&job.database_name),
            BackupType::Incremental => {
                // Find the most recent completed backup as parent
                let parent_backup_id = self.find_most_recent_backup(&job.database_name)?;
                self.backup_manager
                    .create_incremental_backup(&job.database_name, &parent_backup_id)
            }
            BackupType::Differential => {
                // Find the most recent full backup as base
                let base_backup_id = self.find_most_recent_full_backup(&job.database_name)?;
                self.backup_manager
                    .create_differential_backup(&job.database_name, &base_backup_id)
            }
            BackupType::ArchiveLog => {
                self.backup_manager
                    .create_archive_log_backup(&job.database_name)
            }
        };

        let end_time = SystemTime::now();
        execution.end_time = Some(end_time);
        execution.duration = end_time.duration_since(start_time).ok();

        match backup_result {
            Ok(backup_id) => {
                execution.backup_id = Some(backup_id.clone());
                execution.status = ExecutionStatus::Completed;
                self.execution_history
                    .write()
                    .insert(start_time, execution);
                Ok(backup_id)
            }
            Err(e) => {
                execution.status = ExecutionStatus::Failed;
                execution.error_message = Some(e.to_string());
                self.execution_history
                    .write()
                    .insert(start_time, execution);
                Err(e)
            }
        }
    }

    // Get execution history
    pub fn get_execution_history(&self, limit: Option<usize>) -> Vec<BackupExecution> {
        let history = self.execution_history.read();
        let mut executions: Vec<_> = history.values().cloned().collect();
        executions.reverse(); // Most recent first
        if let Some(limit) = limit {
            executions.truncate(limit);
        }
        executions
    }

    // Enable/disable scheduler
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock() = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock()
    }

    // Find the most recent completed backup for a database (for incremental backups)
    fn find_most_recent_backup(&self, database_name: &str) -> Result<String> {
        let backups = self.backup_manager.list_backups();

        // Filter backups for this database that are completed
        let mut db_backups: Vec<_> = backups
            .into_iter()
            .filter(|b| {
                b.database_name == database_name
                    && b.is_complete()
            })
            .collect();

        // Sort by start time (most recent first)
        db_backups.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        db_backups
            .first()
            .map(|b| b.backup_id.clone())
            .ok_or_else(|| {
                DbError::BackupError(format!(
                    "No completed backup found for database: {}",
                    database_name
                ))
            })
    }

    // Find the most recent completed full backup for a database (for differential backups)
    fn find_most_recent_full_backup(&self, database_name: &str) -> Result<String> {
        let backups = self.backup_manager.list_backups();

        // Filter full backups for this database that are completed
        let mut full_backups: Vec<_> = backups
            .into_iter()
            .filter(|b| {
                b.database_name == database_name
                    && b.backup_type == BackupType::Full
                    && b.is_complete()
            })
            .collect();

        // Sort by start time (most recent first)
        full_backups.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        full_backups
            .first()
            .map(|b| b.backup_id.clone())
            .ok_or_else(|| {
                DbError::BackupError(format!(
                    "No completed full backup found for database: {}",
                    database_name
                ))
            })
    }

    // Save configuration to disk
    fn save_config(&self) -> Result<()> {
        let config_file = self.config_path.join("scheduler_config.json");
        let config = SchedulerConfig {
            jobs: self.jobs.read().clone(),
            windows: self.windows.read().clone(),
        };
        let json = serde_json::to_string_pretty(&config)?;
        let mut file = File::create(config_file)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    // Load configuration from disk
    fn load_config(&self) -> Result<()> {
        let config_file = self.config_path.join("scheduler_config.json");
        if !config_file.exists() {
            return Ok(());
        }

        let mut file = File::open(config_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: SchedulerConfig = serde_json::from_str(&contents)?;

        *self.jobs.write() = config.jobs;
        *self.windows.write() = config.windows;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchedulerConfig {
    jobs: HashMap<String, ScheduledBackup>,
    windows: HashMap<String, BackupWindow>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_parse_simple() {
        let schedule = CronSchedule::parse("0 0 * * *").unwrap();
        assert_eq!(schedule.minute, CronField::Specific(0));
        assert_eq!(schedule.hour, CronField::Specific(0));
        assert_eq!(schedule.day_of_month, CronField::Any);
    }

    #[test]
    fn test_cron_parse_range() {
        let schedule = CronSchedule::parse("0 9-17 * * 1-5").unwrap();
        assert_eq!(schedule.hour, CronField::Range(9, 17));
        assert_eq!(schedule.day_of_week, CronField::Range(1, 5));
    }

    #[test]
    fn test_cron_parse_step() {
        let schedule = CronSchedule::parse("*/15 * * * *").unwrap();
        assert!(matches!(schedule.minute, CronField::Step(_, 15)));
    }

    #[test]
    fn test_backup_window_active() {
        let mut window = BackupWindow::new("test".to_string(), "Test".to_string());
        window.start_hour = 9;
        window.end_hour = 17;

        // This test would need a specific time to properly validate
        // For now, just ensure it doesn't panic
        let now = SystemTime::now();
        let _ = window.is_active(now);
    }

    #[test]
    fn test_scheduled_backup_creation() {
        let schedule = CronSchedule::daily(2, 0);
        let job = ScheduledBackup::new(
            "job1".to_string(),
            "Daily Backup".to_string(),
            "testdb".to_string(),
            BackupType::Full,
            schedule,
        );
        assert_eq!(job.job_id, "job1");
        assert!(job.enabled);
    }
}
