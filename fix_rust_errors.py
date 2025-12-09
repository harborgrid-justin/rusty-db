#!/usr/bin/env python3
"""
Comprehensive Rust build fixer for rusty-db
Fixes:
1. Underscore variable references (both let bindings and function parameters)
2. Missing imports (DbError, collections, time, etc.)
3. let statement semicolons incorrectly changed to commas
"""

import os
import re
import sys
from pathlib import Path

SRC_DIR = "/workspaces/rusty-db/src"

def fix_let_semicolons(content):
    """Fix let statements that incorrectly have commas instead of semicolons"""
    # Pattern: let ... = ..., at end of line (should be ;)
    # But NOT inside struct definitions or function calls
    lines = content.split('\n')
    fixed_lines = []
    in_struct = False
    brace_depth = 0

    for line in lines:
        # Track struct context
        if re.match(r'\s*(pub\s+)?struct\s+\w+', line):
            in_struct = True

        # Track brace depth
        brace_depth += line.count('{') - line.count('}')
        if brace_depth == 0:
            in_struct = False

        # Fix let statements with comma at end (not in struct)
        if not in_struct and re.match(r'\s*let\s+(mut\s+)?[a-z_][a-z0-9_]*\s*:\s*\w+\s*,\s*$', line):
            line = line.rstrip().rstrip(',') + ';'

        # Fix let statements with value assignment ending in comma
        if not in_struct and re.match(r'\s*let\s+(mut\s+)?[a-z_][a-z0-9_]*\s*(:\s*\w+)?\s*=.*,\s*$', line):
            # Check it's not a tuple or array continuation
            if not line.rstrip().endswith('(,') and not line.rstrip().endswith('[,'):
                line = line.rstrip().rstrip(',') + ';'

        fixed_lines.append(line)

    return '\n'.join(fixed_lines)

def fix_underscore_variables(content):
    """Fix underscore variable references"""

    # Find all _variable definitions in let statements
    let_pattern = r'\blet\s+(mut\s+)?(_[a-z][a-z0-9_]*)\s*[=:]'
    let_matches = re.findall(let_pattern, content)

    for mut, var in let_matches:
        plain_var = var[1:]  # Remove leading underscore
        # Check if the plain variable is used somewhere
        if re.search(rf'\b{plain_var}\b', content):
            # Replace the underscore variable with plain
            content = re.sub(rf'\blet\s+{var}\b', f'let {plain_var}', content)
            content = re.sub(rf'\blet\s+mut\s+{var}\b', f'let mut {plain_var}', content)

    # Find all _variable definitions in function parameters
    param_pattern = r'fn\s+\w+[^)]*\(([^)]+)\)'

    def fix_params(match):
        params = match.group(0)
        # Find underscore params that are used in function body
        return params

    # Fix for loop variables
    for_pattern = r'for\s+(_[a-z][a-z0-9_]*)\s+in'
    for_matches = re.findall(for_pattern, content)

    for var in for_matches:
        plain_var = var[1:]
        # Check if plain var is used in nearby context
        if re.search(rf'\b{plain_var}\b', content):
            content = re.sub(rf'for\s+{var}\s+in', f'for {plain_var} in', content)

    # Fix function parameter underscore variables
    # Pattern: find function definitions with _param and usage of param
    fn_pattern = r'((?:pub\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+\w+[^{]*\{)'

    def fix_fn_params(content):
        lines = content.split('\n')
        result = []
        i = 0
        while i < len(lines):
            line = lines[i]
            # Check for function definition with underscore params
            fn_match = re.search(r'fn\s+\w+.*\(([^)]*_[a-z][a-z0-9_]*[^)]*)\)', line)
            if fn_match:
                params_str = fn_match.group(1)
                # Find underscore params
                underscore_params = re.findall(r'(_[a-z][a-z0-9_]*)\s*:', params_str)

                # Look ahead to find function body
                brace_count = line.count('{') - line.count('}')
                fn_body_start = i
                fn_body_end = i

                j = i + 1
                while j < len(lines) and brace_count > 0:
                    brace_count += lines[j].count('{') - lines[j].count('}')
                    fn_body_end = j
                    j += 1

                # Get function body
                fn_body = '\n'.join(lines[i:fn_body_end+1])

                # Check which underscore params are actually used without underscore
                for param in underscore_params:
                    plain_param = param[1:]
                    # Check if plain param is used in body
                    if re.search(rf'\b{plain_param}\b', fn_body):
                        # Replace in the function signature line
                        line = re.sub(rf'\b{param}\s*:', f'{plain_param}:', line)

            result.append(line)
            i += 1

        return '\n'.join(result)

    content = fix_fn_params(content)

    return content

def fix_specific_patterns(content):
    """Fix specific known problematic patterns"""

    # Fix _value references in SIMD/intrinsic code
    content = re.sub(r'\b_value\b(?!\s*[=:])', 'value', content)
    content = re.sub(r'\b_writes_count\b', 'writes_count', content)
    content = re.sub(r'\b_flush_count\b', 'flush_count', content)

    return content

def add_missing_imports(content, filepath):
    """Add missing imports based on usage"""

    imports_to_add = []

    # Check for DbError usage
    if re.search(r'\bDbError\b', content):
        if not re.search(r'use\s+.*DbError', content) and not re.search(r'use\s+crate::error', content):
            if 'error.rs' not in filepath:
                imports_to_add.append('use crate::error::DbError;')

    # Check for HashSet usage
    if re.search(r'\bHashSet\b', content):
        if not re.search(r'use\s+std::collections::HashSet', content) and \
           not re.search(r'use\s+std::collections::\{[^}]*HashSet', content):
            imports_to_add.append('use std::collections::HashSet;')

    # Check for BTreeMap usage
    if re.search(r'\bBTreeMap\b', content):
        if not re.search(r'use\s+std::collections::BTreeMap', content) and \
           not re.search(r'use\s+std::collections::\{[^}]*BTreeMap', content):
            imports_to_add.append('use std::collections::BTreeMap;')

    # Check for VecDeque usage
    if re.search(r'\bVecDeque\b', content):
        if not re.search(r'use\s+std::collections::VecDeque', content) and \
           not re.search(r'use\s+std::collections::\{[^}]*VecDeque', content):
            imports_to_add.append('use std::collections::VecDeque;')

    # Check for Mutex usage
    if re.search(r'\bMutex\b', content):
        if not re.search(r'use\s+std::sync::Mutex', content) and \
           not re.search(r'use\s+std::sync::\{[^}]*Mutex', content) and \
           not re.search(r'use\s+tokio::sync::Mutex', content) and \
           not re.search(r'use\s+parking_lot::Mutex', content):
            imports_to_add.append('use std::sync::Mutex;')

    # Check for oneshot usage
    if re.search(r'\boneshot\b', content):
        if not re.search(r'use\s+tokio::sync::oneshot', content):
            imports_to_add.append('use tokio::sync::oneshot;')

    # Check for sleep usage (tokio)
    if re.search(r'\bsleep\s*\(', content) or re.search(r'\bsleep\s*\.await', content):
        if not re.search(r'use\s+tokio::time::sleep', content) and \
           not re.search(r'use\s+tokio::time::\{[^}]*sleep', content):
            imports_to_add.append('use tokio::time::sleep;')

    # Check for fmt::Display, fmt::Formatter usage
    if re.search(r'\bfmt::(Display|Formatter|Debug|Write)\b', content):
        if not re.search(r'use\s+std::fmt', content):
            imports_to_add.append('use std::fmt;')

    # Check for io::{Read, Write, Seek} usage
    if re.search(r'\bio::(Read|Write|Seek|BufReader|BufWriter)\b', content):
        if not re.search(r'use\s+std::io', content):
            imports_to_add.append('use std::io;')

    if imports_to_add:
        # Find the first use statement or module declaration
        lines = content.split('\n')
        insert_pos = 0

        for i, line in enumerate(lines):
            if line.strip().startswith('use ') or line.strip().startswith('mod '):
                insert_pos = i
                break
            elif line.strip() and not line.strip().startswith('//') and not line.strip().startswith('#'):
                insert_pos = i
                break

        # Insert imports
        for imp in imports_to_add:
            if imp not in content:
                lines.insert(insert_pos, imp)
                insert_pos += 1

        content = '\n'.join(lines)

    return content

def process_file(filepath):
    """Process a single Rust file"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original = content

        # Apply fixes
        content = fix_let_semicolons(content)
        content = fix_underscore_variables(content)
        content = fix_specific_patterns(content)
        content = add_missing_imports(content, filepath)

        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

def main():
    """Main entry point"""
    modified_count = 0

    for root, dirs, files in os.walk(SRC_DIR):
        for file in files:
            if file.endswith('.rs'):
                filepath = os.path.join(root, file)
                if process_file(filepath):
                    print(f"Fixed: {filepath}")
                    modified_count += 1

    print(f"\nModified {modified_count} files")
    print("Run 'cargo check 2>&1 | head -50' to verify")

if __name__ == '__main__':
    main()
