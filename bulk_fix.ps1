# PowerShell script to fix common Rust warnings

$fixPairs = @{
    # Remove unused imports
    'use crate::error::{DbError, Result};' = 'use crate::error::Result;'
    'use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};' = 'use std::sync::atomic::{AtomicBool, Ordering};'
    'use tokio::sync::{mpsc, oneshot, RwLock};' = 'use tokio::sync::{mpsc, RwLock};'
    'use tokio::sync::{Semaphore, OwnedSemaphorePermit};' = 'use tokio::sync::Semaphore;'
    'use tokio::time::{sleep, timeout};' = 'use tokio::time::timeout;'
    'use chrono::{DateTime, Utc};' = ''
    'use std::fmt;' = ''
    'use tracing::{debug, info, warn, error};' = 'use tracing::{debug, info, error};'
    
    # Fix variable names
    'let mut accepted = ' = 'let mut _accepted = '
    'let policy = ' = 'let _policy = '
    'let hash = ' = 'let _hash = '
    'let benchmark = ' = 'let _benchmark = '
    'let mut statistics = ' = 'let statistics = '
    'let mut statement_cursor = ' = 'let statement_cursor = '
    'for i in ' = 'for _i in '
    'let state = ' = 'let _state = '
    'let value = ' = 'let _value = '
    'let symbols = ' = 'let _symbols = '
    'let context = ' = 'let _context = '
    'let result = ' = 'let _result = '
    'let last_error = ' = 'let _last_error = '
    'let transaction_id = ' = 'let _transaction_id = '
    'let message = ' = 'let _message = '
    'let source = ' = 'let _source = '
    'let block_data = ' = 'let _block_data = '
    'let interval_ms = ' = 'let _interval_ms = '
    'let handlers = ' = 'let _handlers = '
    'let stats = ' = 'let _stats = '
    'let to_node = ' = 'let _to_node = '
    'let reason = ' = 'let _reason = '
    'let partition_id = ' = 'let _partition_id = '
    'let log_entry = ' = 'let _log_entry = '
    'let logs = ' = 'let _logs = '
    'let replica_id = ' = 'let _replica_id = '
    'let deleted = ' = 'let _deleted = '
    
    # Fix parentheses issues
    '            _ => (false),' = '            _ => false,'
    
    # Remove specific unused imports
    ', Read' = ''
    ', Write' = ''
    ', Declaration' = ''
    ', ExecutionResult' = ''
    ', ScalarFunction, TableFunction' = ''
    ', SystemTime' = ''
    ', ResourceClass' = ''
    ', BTreeMap' = ''
    ', HashSet' = ''
    ', VecDeque' = ''
    ', Instant' = ''
    ', Duration' = ''
    ', Mutex' = ''
    ', oneshot' = ''
    ', OwnedSemaphorePermit' = ''
    ', sleep' = ''
    ', DateTime, Utc' = ''
    ', SessionId' = ''
    ', warn' = ''
    ', UNIX_EPOCH' = ''
    ', AsyncRead, AsyncWrite, BufReader, BufWriter' = ''
    ', AsyncReadExt' = ''
    ', AsyncWriteExt' = ''
    ', ComparisonMask, prefetch_read' = ''
    ', Tuple, Value' = ''
    ', LineString' = ''
    ', Geometry' = ''
    ', BoundingBox' = ''
}

$rustFiles = Get-ChildItem -Path "f:\temp\rusty-db\src" -Recurse -Filter "*.rs"

foreach ($file in $rustFiles) {
    $content = Get-Content $file.FullName -Raw
    $modified = $false
    
    foreach ($pair in $fixPairs.GetEnumerator()) {
        if ($content.Contains($pair.Key)) {
            $content = $content.Replace($pair.Key, $pair.Value)
            $modified = $true
        }
    }
    
    if ($modified) {
        Set-Content $file.FullName $content -NoNewline
        Write-Host "Fixed: $($file.FullName)"
    }
}

Write-Host "Bulk fixes completed!"