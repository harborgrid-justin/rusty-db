#!/usr/bin/env python3
"""
Script to fix missing imports in Rust files.
Adds:
- Mutex (from parking_lot or std::sync)
- sleep (from tokio::time)
- interval (from tokio::time)
"""

import re
import sys
from pathlib import Path

def has_parking_lot_import(content):
    """Check if file already uses parking_lot"""
    return 'use parking_lot::' in content or 'use parking_lot::{' in content

def needs_mutex_import(content):
    """Check if file uses Mutex but doesn't import it"""
    # Check for Mutex:: usage
    if 'Mutex::new(' not in content and 'Mutex<' not in content:
        return False

    # Check if already imported
    if 'use parking_lot::Mutex' in content or 'use std::sync::Mutex' in content:
        return False
    if 'use parking_lot::{' in content and 'Mutex' in content.split('use parking_lot::{')[1].split('}')[0]:
        return False
    if 'use std::sync::{' in content and 'Mutex' in content.split('use std::sync::{')[1].split('}')[0]:
        return False

    return True

def needs_sleep_import(content):
    """Check if file uses sleep but doesn't import it"""
    if 'sleep(' not in content:
        return False

    # Check if already imported
    if 'use tokio::time::sleep' in content:
        return False
    if 'use tokio::time::{' in content and 'sleep' in content.split('use tokio::time::{')[1].split('}')[0]:
        return False

    return True

def needs_interval_import(content):
    """Check if file uses interval but doesn't import it"""
    if 'interval(' not in content:
        return False

    # Check if already imported
    if 'use tokio::time::interval' in content:
        return False
    if 'use tokio::time::{' in content and 'interval' in content.split('use tokio::time::{')[1].split('}')[0]:
        return False

    return True

def find_import_section_end(lines):
    """Find the last line of the import section"""
    last_use_line = -1
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith('use '):
            last_use_line = i
        elif last_use_line > -1 and stripped and not stripped.startswith('//') and not stripped.startswith('use '):
            # Found first non-use, non-comment line after imports
            return last_use_line

    return last_use_line if last_use_line > -1 else 0

def add_mutex_import(lines, use_parking_lot):
    """Add Mutex import to the appropriate line"""
    if use_parking_lot:
        # Try to add to existing parking_lot import
        for i, line in enumerate(lines):
            if line.strip().startswith('use parking_lot::RwLock;'):
                lines[i] = line.replace('use parking_lot::RwLock;', 'use parking_lot::{RwLock, Mutex};')
                return True
            elif line.strip().startswith('use parking_lot::'):
                # Add Mutex to existing single import
                import_item = line.strip()[17:].rstrip(';')
                lines[i] = f'use parking_lot::{{{import_item}, Mutex}};\n'
                return True
            elif 'use parking_lot::{' in line:
                # Add to existing multi-import
                parts = line.split('}')
                items = parts[0].split('{')[1]
                if 'Mutex' not in items:
                    lines[i] = parts[0] + ', Mutex}' + '}'. join(parts[1:])
                return True

        # No existing parking_lot import, add new one
        insert_pos = find_import_section_end(lines) + 1
        lines.insert(insert_pos, 'use parking_lot::Mutex;\n')
    else:
        # Use std::sync::Mutex
        for i, line in enumerate(lines):
            if 'use std::sync::{' in line and 'Arc' in line:
                parts = line.split('}')
                items = parts[0].split('{')[1]
                if 'Mutex' not in items:
                    lines[i] = parts[0] + ', Mutex}' + '}'.join(parts[1:])
                return True

        # Add new import
        insert_pos = find_import_section_end(lines) + 1
        lines.insert(insert_pos, 'use std::sync::Mutex;\n')

    return True

def add_sleep_import(lines):
    """Add sleep import from tokio::time"""
    for i, line in enumerate(lines):
        if 'use tokio::time::{' in line:
            parts = line.split('}')
            items = parts[0].split('{')[1]
            if 'sleep' not in items:
                lines[i] = parts[0] + ', sleep}' + '}'.join(parts[1:])
            return True

    # Add new import
    insert_pos = find_import_section_end(lines) + 1
    lines.insert(insert_pos, 'use tokio::time::sleep;\n')
    return True

def add_interval_import(lines):
    """Add interval import from tokio::time"""
    for i, line in enumerate(lines):
        if 'use tokio::time::{' in line:
            parts = line.split('}')
            items = parts[0].split('{')[1]
            if 'interval' not in items:
                lines[i] = parts[0] + ', interval}' + '}'.join(parts[1:])
            return True
        elif line.strip() == 'use tokio::time::sleep;':
            lines[i] = 'use tokio::time::{sleep, interval};\n'
            return True

    # Add new import
    insert_pos = find_import_section_end(lines) + 1
    lines.insert(insert_pos, 'use tokio::time::interval;\n')
    return True

def fix_file(file_path):
    """Fix imports in a single file"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        lines = content.split('\n')

        # Determine what needs to be added
        needs_mutex = needs_mutex_import(content)
        needs_sleep = needs_sleep_import(content)
        needs_interval = needs_interval_import(content)

        if not (needs_mutex or needs_sleep or needs_interval):
            return False, "No changes needed"

        changes = []

        # Add imports
        if needs_mutex:
            use_parking_lot = has_parking_lot_import(content)
            add_mutex_import(lines, use_parking_lot)
            changes.append(f"Mutex ({'parking_lot' if use_parking_lot else 'std::sync'})")

        if needs_sleep:
            add_sleep_import(lines)
            changes.append("sleep")

        if needs_interval:
            add_interval_import(lines)
            changes.append("interval")

        # Write back
        new_content = '\n'.join(lines)
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(new_content)

        return True, f"Added: {', '.join(changes)}"

    except Exception as e:
        return False, f"Error: {str(e)}"

def main():
    """Main function to process all Rust files"""
    src_dir = Path('src')

    if not src_dir.exists():
        print("Error: src directory not found")
        sys.exit(1)

    rust_files = list(src_dir.rglob('*.rs'))

    fixed_count = 0
    for file_path in rust_files:
        changed, message = fix_file(file_path)
        if changed:
            print(f"✓ {file_path}: {message}")
            fixed_count += 1

    print(f"\nFixed {fixed_count} files")

if __name__ == '__main__':
    main()
