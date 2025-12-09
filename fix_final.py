#!/usr/bin/env python3
"""
Final comprehensive fix script for rusty-db
"""

import os
import re
from pathlib import Path

SRC_DIR = "/workspaces/rusty-db/src"

def fix_broken_format_macro(content):
    """Fix format!() calls that have semicolons instead of closing paren"""
    # Pattern: format!("...", something;
    pattern = re.compile(r'(format!\([^;]+);(\s*$)', re.MULTILINE)
    content = pattern.sub(r'\1);\2', content)
    return content

def fix_snake_case_variables(content):
    """Fix snake_case variable mismatches (page_id vs pageid)"""
    replacements = [
        (r'\bpageid\b', 'page_id'),
        (r'\btablename\b', 'table_name'),
        (r'\bnumnodes\b', 'num_nodes'),
        (r'\bnumpartitions\b', 'num_partitions'),
        (r'\bblocksize\b', 'block_size'),
        (r'\bbuffersize\b', 'buffer_size'),
        (r'\bpagesize\b', 'page_size'),
        (r'\bfilename\b(?!\s*[=:])', 'file_name'),  # Don't change if it's a definition
    ]

    for pattern, replacement in replacements:
        content = re.sub(pattern, replacement, content)

    return content

def remove_duplicate_dbderror_import(content):
    """Remove duplicate DbError imports"""
    lines = content.split('\n')
    has_dberror_import = False
    result = []

    for line in lines:
        # Check if this line imports DbError
        if 'use crate::error::DbError' in line or 'crate::error::DbError' in line:
            if has_dberror_import:
                # Skip this duplicate
                if 'error::DbError,' in line:
                    # It's part of a larger import, remove just DbError
                    line = re.sub(r'\s*error::DbError,?\s*', '', line)
                    if line.strip() == 'use crate::{' or line.strip() == '};':
                        continue
                else:
                    continue
            else:
                has_dberror_import = True

        result.append(line)

    return '\n'.join(result)

def add_missing_imports(content, filepath):
    """Add missing imports"""
    imports_to_add = []

    # Check for HashMap usage without import
    if re.search(r'\bHashMap\b', content):
        if not re.search(r'use\s+std::collections::HashMap', content) and \
           not re.search(r'use\s+std::collections::\{[^}]*HashMap', content):
            imports_to_add.append('use std::collections::HashMap;')

    # Check for SystemTime usage without import
    if re.search(r'\bSystemTime\b', content):
        if not re.search(r'use\s+std::time::SystemTime', content) and \
           not re.search(r'use\s+std::time::\{[^}]*SystemTime', content):
            imports_to_add.append('use std::time::SystemTime;')

    # Check for UNIX_EPOCH usage without import
    if re.search(r'\bUNIX_EPOCH\b', content):
        if not re.search(r'use\s+std::time::UNIX_EPOCH', content) and \
           not re.search(r'use\s+std::time::\{[^}]*UNIX_EPOCH', content):
            imports_to_add.append('use std::time::UNIX_EPOCH;')

    # Check for Duration usage without import
    if re.search(r'\bDuration\b', content):
        if not re.search(r'use\s+std::time::Duration', content) and \
           not re.search(r'use\s+std::time::\{[^}]*Duration', content) and \
           not re.search(r'use\s+tokio::time::Duration', content):
            imports_to_add.append('use std::time::Duration;')

    # Check for Instant usage without import
    if re.search(r'\bInstant\b', content):
        if not re.search(r'use\s+std::time::Instant', content) and \
           not re.search(r'use\s+std::time::\{[^}]*Instant', content) and \
           not re.search(r'use\s+tokio::time::Instant', content):
            imports_to_add.append('use std::time::Instant;')

    # Check for HashSet usage without import
    if re.search(r'\bHashSet\b', content):
        if not re.search(r'use\s+std::collections::HashSet', content) and \
           not re.search(r'use\s+std::collections::\{[^}]*HashSet', content):
            imports_to_add.append('use std::collections::HashSet;')

    # Check for BTreeMap usage without import
    if re.search(r'\bBTreeMap\b', content):
        if not re.search(r'use\s+std::collections::BTreeMap', content) and \
           not re.search(r'use\s+std::collections::\{[^}]*BTreeMap', content):
            imports_to_add.append('use std::collections::BTreeMap;')

    # Check for VecDeque usage without import
    if re.search(r'\bVecDeque\b', content):
        if not re.search(r'use\s+std::collections::VecDeque', content) and \
           not re.search(r'use\s+std::collections::\{[^}]*VecDeque', content):
            imports_to_add.append('use std::collections::VecDeque;')

    if imports_to_add:
        # Find where to insert (after existing use statements)
        lines = content.split('\n')
        insert_pos = 0

        for i, line in enumerate(lines):
            if line.strip().startswith('use '):
                insert_pos = i + 1

        # If no use statements found, insert at beginning
        if insert_pos == 0:
            for i, line in enumerate(lines):
                if line.strip() and not line.strip().startswith('//') and not line.strip().startswith('#'):
                    insert_pos = i
                    break

        # Remove duplicates from imports_to_add
        imports_to_add = list(set(imports_to_add))

        # Filter out imports that already exist
        imports_to_add = [imp for imp in imports_to_add if imp not in content]

        for imp in imports_to_add:
            lines.insert(insert_pos, imp)
            insert_pos += 1

        content = '\n'.join(lines)

    return content

def fix_let_underscore_in_file(content):
    """Fix let _var where var is used later"""
    # Find all let _var = patterns and check if var is used
    let_pattern = re.compile(r'\blet\s+(mut\s+)?(_[a-z][a-z0-9_]*)\s*=')

    matches = list(let_pattern.finditer(content))

    # Process in reverse to not mess up offsets
    for match in reversed(matches):
        underscore_var = match.group(2)
        plain_var = underscore_var[1:]

        # Check if plain_var is used after this point
        after_match = content[match.end():]
        if re.search(rf'\b{plain_var}\b', after_match):
            # Replace the underscore version
            start = match.start(2)
            end = match.end(2)
            content = content[:start] + plain_var + content[end:]

    return content

def fix_fn_param_underscore(content):
    """Fix function parameters with underscore where the non-underscore version is used"""
    # This is tricky, let's do it more carefully
    fn_pattern = re.compile(r'(fn\s+\w+\s*(?:<[^>]*>)?\s*\([^)]*_[a-z][a-z0-9_]*\s*:[^)]*\))')

    for match in fn_pattern.finditer(content):
        fn_sig = match.group(1)
        # Find underscore params
        params = re.findall(r'(_[a-z][a-z0-9_]*)\s*:', fn_sig)

        for underscore_param in params:
            plain_param = underscore_param[1:]

            # Find the function body
            fn_start = match.start()
            brace_start = content.find('{', match.end())
            if brace_start == -1:
                continue

            # Find matching brace
            brace_count = 1
            pos = brace_start + 1
            while pos < len(content) and brace_count > 0:
                if content[pos] == '{':
                    brace_count += 1
                elif content[pos] == '}':
                    brace_count -= 1
                pos += 1

            fn_body = content[brace_start:pos]

            # Check if plain_param is used in body
            if re.search(rf'\b{plain_param}\b', fn_body):
                # Replace in signature
                new_sig = fn_sig.replace(f'{underscore_param}:', f'{plain_param}:')
                content = content[:match.start()] + new_sig + content[match.end():]

    return content

def process_file(filepath):
    """Process a single file"""
    try:
        with open(filepath, 'r') as f:
            content = f.read()

        original = content

        content = fix_broken_format_macro(content)
        content = fix_snake_case_variables(content)
        content = remove_duplicate_dbderror_import(content)
        content = add_missing_imports(content, filepath)
        content = fix_let_underscore_in_file(content)
        content = fix_fn_param_underscore(content)

        if content != original:
            with open(filepath, 'w') as f:
                f.write(content)
            return True
        return False
    except Exception as e:
        print(f"Error: {filepath}: {e}")
        return False

def main():
    count = 0
    for root, dirs, files in os.walk(SRC_DIR):
        for f in files:
            if f.endswith('.rs'):
                path = os.path.join(root, f)
                if process_file(path):
                    print(f"Fixed: {path}")
                    count += 1
    print(f"\nFixed {count} files")

if __name__ == '__main__':
    main()
