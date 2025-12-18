# MASTER_FINDINGS.md Additions from EA2

## Section 6.1 - Add after EA3-V2:

#### EA2-V1: MVCC Version Counter Memory Leak
- **Location**: `transaction/mvcc.rs:310-315, 444-476, 507-532` + `transaction/version_store.rs:1-465`
- **Vulnerability Type**: Memory leak - unbounded growth (CWE-401)
- **Exploitability**: High (architectural debt - dual implementations)
- **Impact**: Memory exhaustion → database crash under sustained load
- **Root Cause**: Legacy `VersionStore` still exported in mod.rs (line 140) without global version limits. New `MVCCManager` has fix but not all code migrated.
- **Mitigation**: Remove `version_store.rs` exports, complete migration to `MVCCManager`
- **Affected Agent**: Agent 2 (Transaction Layer)

#### EA2-V2: Lock Manager No Timeout - Indefinite Blocking
- **Location**: `transaction/lock_manager.rs:148-206`
- **Vulnerability Type**: Deadlock vulnerability (CWE-833)
- **Exploitability**: High (normal operation can trigger)
- **Impact**: Service freeze, transaction starvation, effective DoS
- **Root Cause**: `acquire_lock` has NO timeout mechanism - returns immediate error instead of waiting
- **Mitigation**: Add timeout parameter, implement condition variable-based wait queues
- **Affected Agent**: Agent 2 (Transaction Layer)

#### EA2-V3: Lock Upgrade Conversion Deadlock
- **Location**: `transaction/lock_manager.rs:160-183`
- **Vulnerability Type**: Incorrect synchronization, breaks 2PL correctness
- **Exploitability**: Medium (requires concurrent upgrades)
- **Impact**: Both transactions fail when one should succeed
- **Mitigation**: Implement proper lock upgrade queue with grant protocol
- **Affected Agent**: Agent 2 (Transaction Layer)

#### EA2-V4: WAL Group Commit Buffer Unbounded Growth
- **Location**: `transaction/wal.rs:251-299`
- **Vulnerability Type**: Allocation without limits (CWE-770)
- **Exploitability**: High (via concurrent commits)
- **Impact**: Memory exhaustion DoS → database crash
- **Mitigation**: Add `MAX_GROUP_COMMIT_ENTRIES = 10000` limit with error on overflow
- **Affected Agent**: Agent 2 (Transaction Layer)

## Section 7.1 - Replace "(To be populated by agents)" with:

#### EA2-RACE-1: Transaction Commit State Transition Window
- **Location**: `transaction/manager.rs:153-185`
- **Issue**: Commit process releases `active_txns` write lock between setting state to `Committing` and `Committed`
- **Race Window**: Between line 170 (lock release) and line 176 (lock re-acquire), other threads can observe transaction in `Committing` state
- **Impact**: Visibility anomaly - external observers see inconsistent state, `is_active()` returns true for committing transaction
- **Severity**: **HIGH** (violates atomicity guarantees)
- **Exploitation**:
  ```rust
  Thread 1: commit(txn=5)             Thread 2: get_transaction(txn=5)
  ================================    ===================================
  txn.state = Committing
  Release active_txns lock  ←───────  Acquire active_txns lock
  release_all_locks(5)...             Read state = Committing ⚠️
  (takes 10ms)                        Return to application
  Re-acquire lock                     Application sees "still committing"
  txn.state = Committed
  ```
- **Mitigation**: Hold `active_txns` lock through entire commit process or use atomic state transitions
- **Affected Agent**: Agent 2 (Transaction Layer)

#### EA2-RACE-2: Lock Upgrade Simultaneous Detection
- **Location**: `transaction/lock_manager.rs:160-183`
- **Issue**: Two transactions holding S locks can both attempt to upgrade to X simultaneously
- **Race Scenario**: T1 and T2 both hold S on resource R. Both call `acquire_lock(R, Exclusive)`. Both see "other holder" and fail, when one should succeed.
- **Impact**: Conversion deadlock - both transactions fail incorrectly
- **Severity**: **HIGH** (breaks 2PL correctness, causes unnecessary transaction aborts)
- **Mitigation**: Implement upgrade queue - transactions mark themselves as "upgrading" and wait for other S-holders to release
- **Affected Agent**: Agent 2 (Transaction Layer)

#### EA2-RACE-3: WAL Truncate Concurrent Write Race
- **Location**: `transaction/wal_manager.rs:434-494`
- **Issue**: Truncate operation reads entire WAL, filters entries, and rewrites - all without exclusive lock
- **Race Window**: Between `read_all()` and `rename()` (lines 439-491), concurrent writes can append to WAL
- **Impact**: Data loss - newly committed transactions written during truncate may be lost
- **Severity**: **CRITICAL** (data corruption)
- **Exploitation**:
  ```rust
  Thread 1: truncate(before_lsn=1000)  Thread 2: append(entry)
  ==================================   ======================
  flush() - sync pending writes
  entries = read_all()                 -
  filter entries >= 1000               -
  -                                    append(entry, LSN=1005)
  -                                    buffer.push(entry)
  -                                    (not flushed yet)
  write filtered to temp file          -
  rename temp → wal.log  ←──────────   flush()  ⚠️ too late!
  (entry LSN=1005 LOST!)
  ```
- **Mitigation**: Add exclusive lock for truncate operation or use atomic log rotation with sequence numbers
- **Affected Agent**: Agent 2 (Transaction Layer)

## Section 7.2 - Replace "(To be populated by agents)" with:

#### EA2-ERR-1: Lock Acquisition Returns Error Instead of Waiting
- **Location**: `transaction/lock_manager.rs:148-206`
- **Issue**: `acquire_lock()` immediately returns `LockConflict` error instead of waiting for lock to become available
- **Impact**: Applications must implement retry logic externally, no fair queuing, potential livelock under contention
- **Severity**: **HIGH** (breaks expected semantics of lock manager)
- **Expected Behavior**: Lock manager should block/wait with timeout, not return immediate error
- **Mitigation**: Implement proper blocking semantics with timeout
- **Affected Agent**: Agent 2 (Transaction Layer)

#### EA2-ERR-2: Deadlock Detector No Cycle Deduplication
- **Location**: `transaction/deadlock.rs:208-243`
- **Issue**: DFS cycle detection can report same cycle multiple times from different starting nodes
- **Impact**: Same deadlock triggers multiple victim selections, redundant abort operations
- **Severity**: **MEDIUM** (inefficiency, not correctness issue)
- **Example**: Cycle [1→2→3→1] can be detected as [1,2,3,1], [2,3,1,2], and [3,1,2,3]
- **Mitigation**: Add cycle deduplication - normalize cycle representation (e.g., start from min txn_id)
- **Affected Agent**: Agent 2 (Transaction Layer)

#### EA2-ERR-3: Missing Timeout Enforcement in 2PC Prepare Phase
- **Location**: `transaction/two_phase_commit.rs:171-214`
- **Issue**: Prepare phase has timeout configured but not actually enforced - simulation only
- **Impact**: Blocked participants can hang indefinitely
- **Severity**: **HIGH** (distributed transaction correctness)
- **Code Evidence**:
  ```rust
  // Line 186-196: Timeout checked but not enforced
  let elapsed = SystemTime::now()
      .duration_since(participant.last_contact)
      .unwrap_or(Duration::ZERO);
  if elapsed > self.prepare_timeout {
      participant.state = ParticipantState::Failed;
      return Ok(false);
  }
  // ⚠️ But this is checking last_contact, not actual RPC timeout!
  ```
- **Mitigation**: Implement actual network timeout on prepare RPC calls
- **Affected Agent**: Agent 2 (Transaction Layer)

## Section 10 - Update Agent 2 row:

| 2 | Transaction Layer | 11 | 5 | 4 | 2 | 0 |

## Update Total row:

| **Total** | **All** | **21** | **7** | **9** | **4** | **1** |

