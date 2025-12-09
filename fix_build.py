#!/usr/bin/env python3
"""
RustyDB Build Fixer
Parses cargo check output and automatically fixes common errors.
"""

import subprocess
import re
import os
import sys
from collections import defaultdict
from pathlib import Path

def run_cargo_check():
    """Run cargo check and capture output."""
    print("Running cargo check...")
    result = subprocess.run(
        ["cargo", "check", "--message-format=short"],
        capture_output=True,
        text=True,
        cwd="/workspaces/rusty-db"
    )
    return result.stderr + result.stdout

def parse_errors(output):
    """Parse cargo check output into structured errors."""
    errors = []

    # Pattern for error messages
    # Format: src/file.rs:line:col: error[E0425]: message
    pattern = r'^([^:]+):(\d+):(\d+):\s+error\[E(\d+)\]:\s+(.+)$'

    for line in output.split('\n'):
        match = re.match(pattern, line)
        if match:
            errors.append({
                'file': match.group(1),
                'line': int(match.group(2)),
                'col': int(match.group(3)),
                'code': f"E{match.group(4)}",
                'message': match.group(5)
            })

    return errors

def fix_underscore_variable(file_path, line_num, var_name):
    """Fix underscore variable by removing underscore from definition."""
    try:
        with open(file_path, 'r') as f:
            lines = f.readlines()

        if line_num <= len(lines):
            line = lines[line_num - 1]
            # Fix the variable definition
            new_line = re.sub(
                rf'\blet\s+(mut\s+)?_{var_name}\b',
                rf'let \1{var_name}',
                line
            )
            if new_line != line:
                lines[line_num - 1] = new_line
                with open(file_path, 'w') as f:
                    f.writelines(lines)
                return True
    except Exception as e:
        print(f"  Error fixing {file_path}:{line_num}: {e}")
    return False

def add_import(file_path, import_statement):
    """Add an import statement to a file if not already present."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()

        # Check if import already exists
        if import_statement in content:
            return False

        lines = content.split('\n')

        # Find the first 'use' statement
        insert_idx = 0
        for i, line in enumerate(lines):
            if line.strip().startswith('use '):
                insert_idx = i
                break
            elif line.strip() and not line.strip().startswith('//') and not line.strip().startswith('/*') and not line.strip().startswith('*') and not line.strip().startswith('#'):
                # Found non-comment, non-attribute line before any use
                insert_idx = i
                break

        lines.insert(insert_idx, import_statement)

        with open(file_path, 'w') as f:
            f.write('\n'.join(lines))
        return True
    except Exception as e:
        print(f"  Error adding import to {file_path}: {e}")
    return False

def fix_missing_type(file_path, type_name):
    """Add import for a missing type."""
    imports = {
        'Duration': 'use std::time::Duration;',
        'Instant': 'use std::time::Instant;',
        'SystemTime': 'use std::time::SystemTime;',
        'HashSet': 'use std::collections::HashSet;',
        'HashMap': 'use std::collections::HashMap;',
        'VecDeque': 'use std::collections::VecDeque;',
        'BTreeMap': 'use std::collections::BTreeMap;',
        'BTreeSet': 'use std::collections::BTreeSet;',
        'BinaryHeap': 'use std::collections::BinaryHeap;',
        'Mutex': 'use std::sync::Mutex;',
        'RwLock': 'use std::sync::RwLock;',
        'Arc': 'use std::sync::Arc;',
        'Rc': 'use std::rc::Rc;',
        'RefCell': 'use std::cell::RefCell;',
        'Cell': 'use std::cell::Cell;',
        'Cow': 'use std::borrow::Cow;',
        'NonZeroU64': 'use std::num::NonZeroU64;',
        'NonZeroU32': 'use std::num::NonZeroU32;',
        'NonZeroUsize': 'use std::num::NonZeroUsize;',
    }

    if type_name in imports:
        return add_import(file_path, imports[type_name])
    return False

def main():
    print("=== RustyDB Build Fixer (Python) ===\n")

    # Run cargo check
    output = run_cargo_check()

    # Parse errors
    errors = parse_errors(output)
    print(f"Found {len(errors)} errors\n")

    if not errors:
        print("No errors found!")
        return

    # Group errors by type
    by_code = defaultdict(list)
    for err in errors:
        by_code[err['code']].append(err)

    print("Error distribution:")
    for code, errs in sorted(by_code.items(), key=lambda x: -len(x[1])):
        print(f"  {code}: {len(errs)} errors")
    print()

    # Fix E0425: cannot find value
    fixes = 0
    if 'E0425' in by_code:
        print(f"Fixing E0425 (cannot find value) errors...")
        for err in by_code['E0425']:
            # Extract variable name from message
            match = re.search(r'cannot find value `(\w+)`', err['message'])
            if match:
                var_name = match.group(1)
                # Check if there's an underscore version in the file
                if fix_underscore_variable(err['file'], err['line'], var_name):
                    fixes += 1

    # Fix E0412: cannot find type
    if 'E0412' in by_code:
        print(f"Fixing E0412 (cannot find type) errors...")
        for err in by_code['E0412']:
            match = re.search(r'cannot find type `(\w+)`', err['message'])
            if match:
                type_name = match.group(1)
                if fix_missing_type(err['file'], type_name):
                    fixes += 1

    print(f"\nApplied {fixes} fixes")
    print("\nRun the script again to check for remaining errors.")
    print("Run 'cargo check' to verify fixes.")

if __name__ == "__main__":
    main()
