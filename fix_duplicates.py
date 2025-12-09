#!/usr/bin/env python3
"""
Fix duplicate imports and struct field semicolons in rusty-db
"""

import os
import re
from pathlib import Path

SRC_DIR = "/workspaces/rusty-db/src"

def fix_struct_semicolons(content):
    """Fix struct field semicolons -> commas"""
    lines = content.split('\n')
    result = []
    in_struct = False
    brace_depth = 0
    struct_brace_depth = 0

    for line in lines:
        # Detect struct definition start
        if re.match(r'\s*(pub\s+)?struct\s+\w+', line) and '{' in line:
            in_struct = True
            struct_brace_depth = brace_depth + 1
        elif re.match(r'\s*(pub\s+)?struct\s+\w+', line):
            in_struct = True
            struct_brace_depth = brace_depth + 1

        # Track braces
        brace_depth += line.count('{') - line.count('}')

        # Check if we exited the struct
        if in_struct and brace_depth < struct_brace_depth:
            in_struct = False

        # Fix semicolons in struct fields
        if in_struct:
            # Match field definition with semicolon
            if re.match(r'\s*(pub\s+)?[a-z_][a-z0-9_]*\s*:\s*[^,;]+;\s*$', line):
                line = line.rstrip().rstrip(';') + ','

        result.append(line)

    return '\n'.join(result)

def fix_duplicate_mutex_imports(content):
    """Remove std::sync::Mutex if parking_lot::Mutex exists"""
    if 'use parking_lot::{Mutex' in content or 'use parking_lot::Mutex' in content:
        # Remove std::sync::Mutex
        content = re.sub(r'use std::sync::Mutex;\n?', '', content)
        # Also handle it in combined imports
        content = re.sub(r',\s*Mutex(?=\s*[,}])', '', content)
    return content

def fix_duplicate_imports(content):
    """Fix various duplicate import issues"""
    # Remove duplicate use std::sync::Mutex when parking_lot::Mutex exists
    content = fix_duplicate_mutex_imports(content)

    # Remove duplicate HashSet imports
    lines = content.split('\n')
    seen_imports = set()
    result = []

    for line in lines:
        # Check for import lines
        if line.strip().startswith('use '):
            if line.strip() in seen_imports:
                continue  # Skip duplicate
            seen_imports.add(line.strip())
        result.append(line)

    return '\n'.join(result)

def process_file(filepath):
    """Process a single file"""
    try:
        with open(filepath, 'r') as f:
            content = f.read()

        original = content

        content = fix_struct_semicolons(content)
        content = fix_duplicate_imports(content)

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
