// Integration test for BackupScheduler
// This file demonstrates the scheduler capabilities

#[cfg(test)]
mod scheduler_integration_tests {
    use rusty_db::backup::{
        BackupConfig, BackupManager, BackupScheduler, BackupType, BackupWindow, CronSchedule,
        RetentionPolicy, ScheduledBackup,
    };
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_scheduler_creation() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let backup_manager = Arc::new(BackupManager::new(backup_config, retention_policy).unwrap());

        let scheduler_path = PathBuf::from("/tmp/rustydb_scheduler_test");
        let scheduler = BackupScheduler::new(backup_manager, scheduler_path);

        assert!(scheduler.is_ok());
    }

    #[test]
    fn test_add_scheduled_job() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let backup_manager = Arc::new(BackupManager::new(backup_config, retention_policy).unwrap());

        let scheduler_path = PathBuf::from("/tmp/rustydb_scheduler_test_jobs");
        let scheduler = BackupScheduler::new(backup_manager, scheduler_path).unwrap();

        // Create a daily backup schedule
        let schedule = CronSchedule::daily(2, 0); // 2:00 AM daily
        let job = ScheduledBackup::new(
            "daily_backup".to_string(),
            "Daily Full Backup".to_string(),
            "testdb".to_string(),
            BackupType::Full,
            schedule,
        );

        assert!(scheduler.add_job(job).is_ok());

        // Verify job was added
        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_id, "daily_backup");
    }

    #[test]
    fn test_backup_window() {
        let mut window = BackupWindow::business_hours();
        window.enabled = true;

        // Window should have business hours configuration
        assert_eq!(window.start_hour, 9);
        assert_eq!(window.end_hour, 17);
        assert_eq!(window.days_of_week.len(), 5); // Mon-Fri
    }

    #[test]
    fn test_cron_parsing() {
        // Test various cron expressions
        let hourly = CronSchedule::parse("0 * * * *");
        assert!(hourly.is_ok());

        let daily = CronSchedule::parse("0 2 * * *");
        assert!(daily.is_ok());

        let weekly = CronSchedule::parse("0 2 * * 0"); // Sunday 2AM
        assert!(weekly.is_ok());

        let invalid = CronSchedule::parse("invalid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_scheduler_enable_disable() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let backup_manager = Arc::new(BackupManager::new(backup_config, retention_policy).unwrap());

        let scheduler_path = PathBuf::from("/tmp/rustydb_scheduler_test_enable");
        let scheduler = BackupScheduler::new(backup_manager, scheduler_path).unwrap();

        assert!(scheduler.is_enabled());

        scheduler.set_enabled(false);
        assert!(!scheduler.is_enabled());

        scheduler.set_enabled(true);
        assert!(scheduler.is_enabled());
    }

    #[test]
    fn test_multiple_backup_windows() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let backup_manager = Arc::new(BackupManager::new(backup_config, retention_policy).unwrap());

        let scheduler_path = PathBuf::from("/tmp/rustydb_scheduler_test_windows");
        let scheduler = BackupScheduler::new(backup_manager, scheduler_path).unwrap();

        // Add business hours window
        let business_window = BackupWindow::business_hours();
        scheduler.add_window(business_window).unwrap();

        // Add off-hours window
        let offhours_window = BackupWindow::off_hours();
        scheduler.add_window(offhours_window).unwrap();

        // Create jobs for each window
        let schedule1 = CronSchedule::daily(10, 0); // 10 AM
        let mut job1 = ScheduledBackup::new(
            "business_backup".to_string(),
            "Business Hours Backup".to_string(),
            "testdb".to_string(),
            BackupType::Incremental,
            schedule1,
        );
        job1.window_id = Some("business".to_string());
        scheduler.add_job(job1).unwrap();

        let schedule2 = CronSchedule::daily(22, 0); // 10 PM
        let mut job2 = ScheduledBackup::new(
            "offhours_backup".to_string(),
            "Off-Hours Full Backup".to_string(),
            "testdb".to_string(),
            BackupType::Full,
            schedule2,
        );
        job2.window_id = Some("offhours".to_string());
        scheduler.add_job(job2).unwrap();

        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 2);
    }

    #[test]
    fn test_execution_history() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let backup_manager = Arc::new(BackupManager::new(backup_config, retention_policy).unwrap());

        let scheduler_path = PathBuf::from("/tmp/rustydb_scheduler_test_history");
        let scheduler = BackupScheduler::new(backup_manager, scheduler_path).unwrap();

        // Get execution history (should be empty initially)
        let history = scheduler.get_execution_history(Some(10));
        assert!(history.is_empty());
    }
}
