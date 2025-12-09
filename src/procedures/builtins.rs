// Built-in Packages for RustyDB
//
// This module provides Oracle-compatible built-in packages like DBMS_OUTPUT,
// DBMS_SQL, UTL_FILE, and DBMS_SCHEDULER for enterprise database operations.

use tokio::time::sleep;
use std::collections::VecDeque;
use crate::{Result, DbError};
use crate::procedures::runtime::RuntimeValue;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{Write, BufRead, BufReader};

// ============================================================================
// DBMS_OUTPUT - Text output buffering
// ============================================================================

// DBMS_OUTPUT package for managing text output from procedures
pub struct DbmsOutput {
    buffer: Arc<RwLock<VecDeque<String>>>,
    enabled: Arc<RwLock<bool>>,
    buffer_size: Arc<RwLock<usize>>,
}

impl DbmsOutput {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            enabled: Arc::new(RwLock::new(false)),
            buffer_size: Arc::new(RwLock::new(20000)),
        }
    }

    // Enable output buffering
    pub fn enable(&self, buffersize: Option<usize>) {
        let mut enabled = self.enabled.write();
        let mut size = self.buffer_size.write();

        *enabled = true;
        if let Some(new_size) = buffersize {
            *size = new_size.min(1_000_000); // Max 1MB
        }
    }

    // Disable output buffering
    pub fn disable(&self) {
        let mut enabled = self.enabled.write();
        let mut buffer = self.buffer.write();

        *enabled = false;
        buffer.clear();
    }

    // Put a line into the buffer
    pub fn put_line(&self, line: String) -> Result<()> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Ok(());
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        let buffer_size = self.buffer_size.read();

        // Check buffer size
        let current_size: usize = buffer.iter().map(|s| s.len()).sum();
        if current_size + line.len() > *buffer_size {
            return Err(DbError::Runtime("Output buffer overflow".to_string()));
        }

        buffer.push_back(line);
        Ok(())
    }

    // Put text without newline
    pub fn put(&self, text: String) -> Result<()> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Ok(());
        }
        drop(enabled);

        let mut buffer = self.buffer.write();

        if let Some(last) = buffer.back_mut() {
            last.push_str(&text);
        } else {
            buffer.push_back(text);
        }

        Ok(())
    }

    // Add a newline to the current line
    pub fn new_line(&self) -> Result<()> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Ok(());
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        buffer.push_back(String::new());

        Ok(())
    }

    // Get a line from the buffer
    pub fn get_line(&self) -> Result<Option<String>> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Err(DbError::Runtime("DBMS_OUTPUT is not enabled".to_string()));
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        Ok(buffer.pop_front())
    }

    // Get multiple lines from the buffer
    pub fn get_lines(&self, numlines: usize) -> Result<Vec<String>> {
        let enabled = self.enabled.read();
        if !*enabled {
            return Err(DbError::Runtime("DBMS_OUTPUT is not enabled".to_string()));
        }
        drop(enabled);

        let mut buffer = self.buffer.write();
        let mut lines = Vec::new();

        for _ in 0..numlines {
            if let Some(line) = buffer.pop_front() {
                lines.push(line);
            } else {
                break;
            }
        }

        Ok(lines)
    }
}

impl Default for DbmsOutput {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DBMS_SQL - Dynamic SQL execution
// ============================================================================

// DBMS_SQL package for dynamic SQL operations
pub struct DbmsSql {
    cursors: Arc<RwLock<HashMap<i32, DynamicCursor>>>,
    next_cursor_id: Arc<RwLock<i32>>,
}

// Dynamic cursor for DBMS_SQL
#[derive(Debug, Clone)]
struct DynamicCursor {
    id: i32,
    sql: Option<String>,
    parsed: bool,
    executed: bool,
    bind_variables: HashMap<String, RuntimeValue>,
    define_columns: HashMap<usize, String>,
}

impl DbmsSql {
    pub fn new() -> Self {
        Self {
            cursors: Arc::new(RwLock::new(HashMap::new())),
            next_cursor_id: Arc::new(RwLock::new(1)),
        }
    }

    // Open a new cursor
    pub fn open_cursor(&self) -> i32 {
        let mut next_id = self.next_cursor_id.write();
        let mut cursors = self.cursors.write();

        let cursor_id = *next_id;
        *next_id += 1;

        let cursor = DynamicCursor {
            id: cursor_id,
            sql: None,
            parsed: false,
            executed: false,
            bind_variables: HashMap::new(),
            define_columns: HashMap::new(),
        };

        cursors.insert(cursor_id, cursor);
        cursor_id
    }

    // Parse SQL statement
    pub fn parse(&self, cursor_id: i32, sql: String) -> Result<()> {
        let mut cursors = self.cursors.write();

        let cursor = cursors.get_mut(&cursor_id).ok_or_else(||
            DbError::NotFound(format!("Cursor {} not found", cursor_id))
        )?;

        cursor.sql = Some(sql);
        cursor.parsed = true;

        Ok(())
    }

    // Bind a variable by name
    pub fn bind_variable(&self, cursor_id: i32, name: String, value: RuntimeValue) -> Result<()> {
        let mut cursors = self.cursors.write();

        let cursor = cursors.get_mut(&cursor_id).ok_or_else(||
            DbError::NotFound(format!("Cursor {} not found", cursor_id))
        )?;

        cursor.bind_variables.insert(name, value);

        Ok(())
    }

    // Execute the SQL statement
    pub fn execute(&self, cursor_id: i32) -> Result<usize> {
        let mut cursors = self.cursors.write();

        let cursor = cursors.get_mut(&cursor_id).ok_or_else(||
            DbError::NotFound(format!("Cursor {} not found", cursor_id))
        )?;

        if !cursor.parsed {
            return Err(DbError::InvalidInput("Cursor has not been parsed".to_string()));
        }

        // TODO: Execute the SQL statement
        cursor.executed = true;

        Ok(0) // Return rows affected
    }

    // Close a cursor
    pub fn close_cursor(&self, cursor_id: i32) -> Result<()> {
        let mut cursors = self.cursors.write();

        if cursors.remove(&cursor_id).is_none() {
            return Err(DbError::NotFound(format!("Cursor {} not found", cursor_id)));
        }

        Ok(())
    }

    // Check if cursor is open
    pub fn is_open(&self, cursor_id: i32) -> bool {
        let cursors = self.cursors.read();
        cursors.contains_key(&cursor_id)
    }
}

impl Default for DbmsSql {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// UTL_FILE - File I/O operations
// ============================================================================

// UTL_FILE package for file operations
pub struct UtlFile {
    file_handles: Arc<RwLock<HashMap<i32, FileHandle>>>,
    next_handle_id: Arc<RwLock<i32>>,
    directories: Arc<RwLock<HashMap<String, PathBuf>>>,
}

// File handle
struct FileHandle {
    id: i32,
    directory: String,
    filename: String,
    mode: FileMode,
    file: Option<File>,
}

// File mode
#[derive(Debug, Clone, PartialEq)]
enum FileMode {
    Read,
    Write,
    Append,
}

impl UtlFile {
    pub fn new() -> Self {
        Self {
            file_handles: Arc::new(RwLock::new(HashMap::new())),
            next_handle_id: Arc::new(RwLock::new(1)),
            directories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Register a directory for file operations
    pub fn add_directory(&self, alias: String, path: PathBuf) {
        let mut directories = self.directories.write();
        directories.insert(alias, path);
    }

    // Open a file
    pub fn fopen(&self, directory: String, filename: String, mode: String) -> Result<i32> {
        let directories = self.directories.read();
        let dir_path = directories.get(&directory).ok_or_else(||
            DbError::NotFound(format!("Directory '{}' not found", directory))
        )?;

        let file_path = dir_path.join(&filename);

        let file_mode = match mode.to_uppercase().as_str() {
            "R" => FileMode::Read,
            "W" => FileMode::Write,
            "A" => FileMode::Append,
            _ => return Err(DbError::InvalidInput(format!("Invalid file mode: {}", mode))),
        };

        let file = match file_mode {
            FileMode::Read => File::open(&file_path)
                .map_err(|e| DbError::IoError(e.to_string()))?,
            FileMode::Write => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&file_path)
                .map_err(|e| DbError::IoError(e.to_string()))?,
            FileMode::Append => OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(&file_path)
                .map_err(|e| DbError::IoError(e.to_string()))?,
        };

        let mut next_id = self.next_handle_id.write();
        let mut handles = self.file_handles.write();

        let handle_id = *next_id;
        *next_id += 1;

        let handle = FileHandle {
            id: handle_id,
            directory: directory.clone(),
            filename: filename.clone(),
            mode: file_mode,
            file: Some(file),
        };

        handles.insert(handle_id, handle);
        Ok(handle_id)
    }

    // Write a line to file
    pub fn put_line(&self, handle_id: i32, text: String) -> Result<()> {
        let mut handles = self.file_handles.write();

        let handle = handles.get_mut(&handle_id).ok_or_else(||
            DbError::NotFound(format!("File handle {} not found", handle_id))
        )?;

        if handle.mode == FileMode::Read {
            return Err(DbError::InvalidInput("Cannot write to file opened for reading".to_string()));
        }

        if let Some(ref mut file) = handle.file {
            writeln!(file, "{}", text)
                .map_err(|e| DbError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    // Read a line from file
    pub fn get_line(&self, handle_id: i32) -> Result<String> {
        let mut handles = self.file_handles.write();

        let handle = handles.get_mut(&handle_id).ok_or_else(||
            DbError::NotFound(format!("File handle {} not found", handle_id))
        )?;

        if handle.mode != FileMode::Read {
            return Err(DbError::InvalidInput("Cannot read from file opened for writing".to_string()));
        }

        if let Some(ref mut file) = handle.file {
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            reader.read_line(&mut line)
                .map_err(|e| DbError::IoError(e.to_string()))?;

            // Remove trailing newline
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }

            Ok(line)
        } else {
            Err(DbError::Runtime("File not open".to_string()))
        }
    }

    // Close a file
    pub fn fclose(&self, handle_id: i32) -> Result<()> {
        let mut handles = self.file_handles.write();

        if handles.remove(&handle_id).is_none() {
            return Err(DbError::NotFound(format!("File handle {} not found", handle_id)));
        }

        Ok(())
    }

    // Check if file is open
    pub fn is_open(&self, handle_id: i32) -> bool {
        let handles = self.file_handles.read();
        handles.contains_key(&handle_id)
    }
}

impl Default for UtlFile {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DBMS_SCHEDULER - Job scheduling
// ============================================================================

// DBMS_SCHEDULER package for job scheduling
pub struct DbmsScheduler {
    jobs: Arc<RwLock<HashMap<String, ScheduledJob>>>,
}

// Scheduled job
#[derive(Debug, Clone)]
struct ScheduledJob {
    name: String,
    job_type: JobType,
    job_action: String,
    schedule: Schedule,
    enabled: bool,
    auto_drop: bool,
    comments: Option<String>,
}

// Job type
#[derive(Debug, Clone)]
enum JobType {
    PlSqlBlock,
    StoredProcedure,
    Executable,
}

// Schedule definition
#[derive(Debug, Clone)]
enum Schedule {
    // Run once at a specific time
    Once { run_date: String },
    // Recurring with interval
    Recurring { interval: String },
    // Calendar expression
    Calendar { expression: String },
}

impl DbmsScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Create a job
    pub fn create_job(
        &self,
        name: String,
        job_type: String,
        job_action: String,
        startdate: Option<String>,
        repeatinterval: Option<String>,
        enabled: bool,
    ) -> Result<()> {
        let mut jobs = self.jobs.write();

        if jobs.contains_key(&name) {
            return Err(DbError::AlreadyExists(
                format!("Job '{}' already exists", name)
            ));
        }

        let job_type_enum = match job_type.to_uppercase().as_str() {
            "PLSQL_BLOCK" => JobType::PlSqlBlock,
            "STORED_PROCEDURE" => JobType::StoredProcedure,
            "EXECUTABLE" => JobType::Executable,
            _ => return Err(DbError::InvalidInput(format!("Invalid job type: {}", job_type))),
        };

        let schedule = if let Some(interval) = repeatinterval {
            Schedule::Recurring { interval }
        } else if let Some(date) = startdate {
            Schedule::Once { run_date: date }
        } else {
            return Err(DbError::InvalidInput("Must specify either start_date or repeat_interval".to_string()));
        };

        let job = ScheduledJob {
            name: name.clone(),
            job_type: job_type_enum,
            job_action,
            schedule,
            enabled,
            auto_drop: false,
            comments: None,
        };

        jobs.insert(name, job);
        Ok(())
    }

    // Enable a job
    pub fn enable_job(&self, job_name: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        let job = jobs.get_mut(job_name).ok_or_else(||
            DbError::NotFound(format!("Job '{}' not found", job_name))
        )?;

        job.enabled = true;
        Ok(())
    }

    // Disable a job
    pub fn disable_job(&self, job_name: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        let job = jobs.get_mut(job_name).ok_or_else(||
            DbError::NotFound(format!("Job '{}' not found", job_name))
        )?;

        job.enabled = false;
        Ok(())
    }

    // Drop a job
    pub fn drop_job(&self, job_name: &str) -> Result<()> {
        let mut jobs = self.jobs.write();

        if jobs.remove(job_name).is_none() {
            return Err(DbError::NotFound(format!("Job '{}' not found", job_name)));
        }

        Ok(())
    }

    // Run a job immediately
    pub fn run_job(&self, job_name: &str) -> Result<()> {
        let jobs = self.jobs.read();

        let job = jobs.get(job_name).ok_or_else(||
            DbError::NotFound(format!("Job '{}' not found", job_name))
        )?;

        if !job.enabled {
            return Err(DbError::InvalidInput(format!("Job '{}' is disabled", job_name)));
        }

        // TODO: Execute the job action
        // For now, just log that it would run
        println!("Would execute job: {} - {}", job.name, job.job_action);

        Ok(())
    }

    // List all jobs
    pub fn list_jobs(&self) -> Vec<String> {
        let jobs = self.jobs.read();
        jobs.keys().cloned().collect()
    }
}

impl Default for DbmsScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DBMS_LOCK - Lock management
// ============================================================================

// DBMS_LOCK package for user-defined locks
pub struct DbmsLock {
    locks: Arc<RwLock<HashMap<String, LockHandle>>>,
}

// Lock handle
#[derive(Debug, Clone)]
struct LockHandle {
    id: String,
    mode: LockMode,
    timeout: Option<i32>,
}

// Lock mode
#[derive(Debug, Clone, PartialEq)]
enum LockMode {
    Exclusive,
    Shared,
    Update,
}

impl DbmsLock {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Request a lock
    pub fn request(
        &self,
        lockid: String,
        lockmode: String,
        timeout: Option<i32>,
    ) -> Result<i32> {
        let mode = match lockmode.to_uppercase().as_str() {
            "EXCLUSIVE" | "X" => LockMode::Exclusive,
            "SHARED" | "S" => LockMode::Shared,
            "UPDATE" | "U" => LockMode::Update,
            _ => return Err(DbError::InvalidInput(format!("Invalid lock mode: {}", lockmode))),
        };

        let mut locks = self.locks.write();

        // Check if lock already exists
        if locks.contains_key(&lockid) {
            return Ok(1); // Lock already held
        }

        let handle = LockHandle {
            id: lockid.clone(),
            mode,
            timeout,
        };

        locks.insert(lockid, handle);
        Ok(0) // Success
    }

    // Release a lock
    pub fn release(&self, lock_id: &str) -> Result<i32> {
        let mut locks = self.locks.write();

        if locks.remove(lock_id).is_some() {
            Ok(0) // Success
        } else {
            Ok(4) // Lock not held
        }
    }

    // Sleep for specified seconds
    pub fn sleep(&self, seconds: f64) -> Result<()> {
        let duration = std::time::Duration::from_secs_f64(seconds);
        std::thread::sleep(duration);
        Ok(())
    }
}

impl Default for DbmsLock {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Built-in Packages Manager
// ============================================================================

// Manager for all built-in packages
pub struct BuiltInPackages {
    pub dbms_output: DbmsOutput,
    pub dbms_sql: DbmsSql,
    pub utl_file: UtlFile,
    pub dbms_scheduler: DbmsScheduler,
    pub dbms_lock: DbmsLock,
}

impl BuiltInPackages {
    pub fn new() -> Self {
        Self {
            dbms_output: DbmsOutput::new(),
            dbms_sql: DbmsSql::new(),
            utl_file: UtlFile::new(),
            dbms_scheduler: DbmsScheduler::new(),
            dbms_lock: DbmsLock::new(),
        }
    }

    // Initialize built-in packages with configuration
    pub fn initialize(&self) {
        // Set up default directories for UTL_FILE
        self.utl_file.add_directory(
            "TEMP".to_string(),
            PathBuf::from("/tmp"),
        );
    }
}

impl Default for BuiltInPackages {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::Duration;

    #[test]
    fn test_dbms_output() -> Result<()> {
        let output = DbmsOutput::new();

        output.enable(Some(1000));
        output.put_line("Hello, World!".to_string())?;
        output.put_line("Line 2".to_string())?;

        let line1 = output.get_line()?;
        assert_eq!(line1, Some("Hello, World!".to_string()));

        let line2 = output.get_line()?;
        assert_eq!(line2, Some("Line 2".to_string()));

        Ok(())
    }

    #[test]
    fn test_dbms_sql() -> Result<()> {
        let dbms_sql = DbmsSql::new();

        let cursor_id = dbms_sql.open_cursor();
        assert!(cursor_id > 0);

        dbms_sql.parse(cursor_id, "SELECT * FROM test".to_string())?;
        dbms_sql.bind_variable(cursor_id, "var1".to_string(), RuntimeValue::Integer(42))?;

        assert!(dbms_sql.is_open(cursor_id));

        dbms_sql.close_cursor(cursor_id)?;
        assert!(!dbms_sql.is_open(cursor_id));

        Ok(())
    }

    #[test]
    fn test_dbms_scheduler() -> Result<()> {
        let scheduler = DbmsScheduler::new();

        scheduler.create_job(
            "test_job".to_string(),
            "PLSQL_BLOCK".to_string(),
            "BEGIN NULL; END;".to_string(),
            None,
            Some("FREQ=DAILY".to_string()),
            true,
        )?;

        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 1);

        scheduler.disable_job("test_job")?;
        scheduler.drop_job("test_job")?;

        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 0);

        Ok(())
    }

    #[test]
    fn test_dbms_lock() -> Result<()> {
        let lock_mgr = DbmsLock::new();

        let result = lock_mgr.request(
            "my_lock".to_string(),
            "EXCLUSIVE".to_string(),
            Some(10),
        )?;

        assert_eq!(result, 0); // Success

        let release_result = lock_mgr.release("my_lock")?;
        assert_eq!(release_result, 0); // Success

        Ok(())
    }
}
