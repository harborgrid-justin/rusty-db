#!/usr/bin/env python3
"""
Script to add #[allow(dead_code)] attributes to functions/methods/constants
that are flagged as dead code but should be kept for completeness of APIs.
"""

import re
import sys

# List of (file_path, pattern, reason) tuples
FIXES = [
    # Buffer/ARC
    ("src/buffer/arc.rs", r"(\s+)(fn total_size\(&self\) -> usize)", "for monitoring API"),

    # Security
    ("src/security/network_hardening/rate_limiting.rs", r"(const MAX_REQUESTS_PER_SECOND_PER_IP)", "for rate limiting config"),
    ("src/security/network_hardening/rate_limiting.rs", r"(\s+)(fn current_rate\(&self\))", "for rate monitoring"),
    ("src/security/auto_recovery/manager.rs", r"(const MAX_RECOVERY_TIME)", "for recovery config"),
    ("src/security/memory_hardening.rs", r"(const POISON_PATTERN)", "for memory protection"),
    ("src/security/memory_hardening.rs", r"(const RED_ZONE_SIZE)", "for memory protection"),
    ("src/security/memory_hardening.rs", r"(\s+)(fn record_access\(&mut self)", "for memory tracking"),

    # Index
    ("src/index/lsm_index.rs", r"(\s+)(fn estimated_fpr\(&self\))", "for bloom filter stats"),

    # Security authentication
    ("src/security/authentication.rs", r"(\s+)(fn users\(&self\))", "for auth API"),
    ("src/security/authentication.rs", r"(\s+)(fn sessions\(&self\))", "for auth API"),

    # Clustering
    ("src/clustering/load_balancer.rs", r"(\s+)(fn is_expired\(&self\))", "for health check"),
    ("src/clustering/load_balancer.rs", r"(\s+)(fn remove\(&mut self)", "for cleanup operations"),

    # RAC (Real Application Clusters)
    ("src/rac/grd.rs", r"(const AFFINITY_WINDOW)", "for cache fusion config"),
    ("src/rac/grd.rs", r"(const MAX_RESOURCES_PER_MASTER)", "for GRD config"),
    ("src/rac/grd.rs", r"(const GRD_FREEZE_TIMEOUT)", "for GRD config"),
    ("src/rac/interconnect.rs", r"(const MAX_MESSAGE_SIZE)", "for interconnect config"),
    ("src/rac/interconnect.rs", r"(const MESSAGE_QUEUE_SIZE)", "for interconnect config"),
    ("src/rac/interconnect.rs", r"(const RECONNECT_BACKOFF_MS)", "for interconnect config"),
    ("src/rac/interconnect.rs", r"(const MAX_RECONNECT_ATTEMPTS)", "for interconnect config"),
    ("src/rac/interconnect.rs", r"(\s+)(fn update_heartbeat\(&mut self)", "for health monitoring"),
    ("src/rac/interconnect.rs", r"(\s+)(fn update_phi_accrual\(&mut self)", "for failure detection"),
    ("src/rac/interconnect.rs", r"(\s+)(fn receive_message\(&self\))", "for message handling"),
    ("src/rac/interconnect.rs", r"(\s+)(fn average_latency\(&self\))", "for performance monitoring"),
    ("src/rac/recovery.rs", r"(const ELECTION_TIMEOUT)", "for recovery config"),
    ("src/rac/recovery.rs", r"(const LOCK_RECLAIM_TIMEOUT)", "for recovery config"),
    ("src/rac/recovery.rs", r"(\s+)(fn flush\(&mut self\))", "for WAL operations"),
    ("src/rac/parallel_query.rs", r"(const WORKER_TIMEOUT)", "for parallel execution config"),
    ("src/rac/parallel_query.rs", r"(\s+)(fn try_steal_work\(&self\))", "for work stealing"),

    # Monitoring
    ("src/monitoring/profiler.rs", r"(\s+)(fn total_execution_time\(&self\))", "for profiling stats"),

    # Flashback
    ("src/flashback/table_restore.rs", r"(\s+)(fn list_tables\(&self\))", "for flashback API"),
    ("src/flashback/database.rs", r"(\s+)(fn add_log\(&mut self)", "for flashback logging"),
    ("src/flashback/database.rs", r"(\s+)(fn list_all\(&self\))", "for flashback API"),

    # Multitenancy
    ("src/multitenancy/isolation.rs", r"(\s+)(fn deallocate\(&mut self)", "for resource management"),

    # Procedures
    ("src/procedures/packages.rs", r"(\s+)(fn evaluate_initial_value\(&mut self)", "for PL/SQL execution"),
    ("src/procedures/builtins.rs", r"(\s+)(fn add_row\(&mut self)", "for table functions"),

    # Spatial
    ("src/spatial/indexes.rs", r"(\s+)(fn split_node\(&mut self)", "for R-tree operations"),
    ("src/spatial/indexes.rs", r"(\s+)(fn pick_seeds\(&self)", "for R-tree operations"),
    ("src/spatial/indexes.rs", r"(\s+)(fn compute_bbox\(&self\))", "for R-tree operations"),
    ("src/spatial/operators.rs", r"(\s+)(fn linestring_intersects_linestring\()", "for spatial ops"),
    ("src/spatial/operators.rs", r"(\s+)(fn point_to_linestring_distance\()", "for spatial ops"),
    ("src/spatial/operators.rs", r"(\s+)(fn point_to_polygon_distance\()", "for spatial ops"),

    # Optimizer
    ("src/optimizer_pro/transformations.rs", r"(\s+)(fn add_table\(&mut self)", "for query graph"),
    ("src/optimizer_pro/transformations.rs", r"(\s+)(fn add_join\(&mut self)", "for query graph"),
    ("src/optimizer_pro/transformations.rs", r"(\s+)(fn get_connected_tables\(&self\))", "for query graph"),
    ("src/optimizer_pro/hints.rs", r"(\s+)(fn validate\(&self\))", "for hint validation"),

    # ML
    ("src/ml/simd_ops.rs", r"(fn scalar_dot_product\()", "for fallback SIMD"),

    # Event processing
    ("src/event_processing/connectors.rs", r"(\s+)(fn serialize_event\(&self)", "for event serialization"),
    ("src/event_processing/connectors.rs", r"(\s+)(fn deserialize_event\(&self\))", "for event deserialization"),
    ("src/event_processing/cq.rs", r"(\s+)(fn get_checkpoint\(&self\))", "for CQ checkpointing"),
    ("src/event_processing/cq.rs", r"(\s+)(fn optimize\(&mut self\))", "for CQ optimization"),
    ("src/event_processing/streams.rs", r"(\s+)(fn is_empty\(&self\))", "for stream API"),
    ("src/event_processing/streams.rs", r"(\s+)(fn get_committed\(&self\))", "for offset management"),
    ("src/event_processing/windows.rs", r"(\s+)(fn remove_event\(&mut self)", "for window management"),
    ("src/event_processing/windows.rs", r"(\s+)(fn contains\(&self)", "for window API"),

    # Enterprise
    ("src/enterprise/lifecycle.rs", r"(\s+)(fn acquire_connection\(&self\))", "for connection management"),
    ("src/enterprise/lifecycle.rs", r"(\s+)(fn release_connection\(&self\))", "for connection management"),
]

def add_allow_dead_code(file_path, pattern, reason):
    """Add #[allow(dead_code)] attribute before matching pattern."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()

        # Check if pattern exists
        if not re.search(pattern, content, re.MULTILINE):
            print(f"Pattern not found in {file_path}: {pattern}")
            return False

        # Check if already has allow attribute nearby
        context_pattern = r'#\[allow\(dead_code\)\]\s*' + pattern
        if re.search(context_pattern, content, re.MULTILINE):
            print(f"Already fixed: {file_path}")
            return False

        # Add attribute
        def replacement(match):
            indent = match.group(1) if match.lastindex and match.lastindex >= 1 else ""
            original = match.group(0)
            comment = f"/// Reserved {reason}"
            attr = "#[allow(dead_code)]"

            # Check if it's a const or fn
            if 'const ' in original:
                return f"{comment}\n{attr}\n{original}"
            else:
                return f"{indent}{comment}\n{indent}{attr}\n{original}"

        new_content = re.sub(pattern, replacement, content, count=1, flags=re.MULTILINE)

        if new_content != content:
            with open(file_path, 'w') as f:
                f.write(new_content)
            print(f"Fixed: {file_path}")
            return True
        else:
            print(f"No change: {file_path}")
            return False

    except FileNotFoundError:
        print(f"File not found: {file_path}")
        return False
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    fixed_count = 0
    for file_path, pattern, reason in FIXES:
        full_path = f"/home/user/rusty-db/{file_path}"
        if add_allow_dead_code(full_path, pattern, reason):
            fixed_count += 1

    print(f"\nTotal files fixed: {fixed_count}")

if __name__ == "__main__":
    main()
