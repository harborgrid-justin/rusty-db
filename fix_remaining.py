#!/usr/bin/env python3
"""
Enhanced Rust build fixer - handles all remaining issues
"""

import os
import re
from pathlib import Path

SRC_DIR = "/workspaces/rusty-db/src"

def remove_duplicate_use_items(content):
    """Remove duplicate items from use statements"""

    # Track what's been imported
    imported = set()
    lines = content.split('\n')
    result = []

    for line in lines:
        stripped = line.strip()

        # Check for simple use statement
        if stripped.startswith('use ') and stripped.endswith(';'):
            # Extract the imported item
            match = re.match(r'use\s+(.+);', stripped)
            if match:
                import_path = match.group(1)
                # Get the final item name
                if '::' in import_path:
                    parts = import_path.split('::')
                    # Handle {A, B} syntax
                    if '{' in parts[-1]:
                        # Complex import, keep it but track items
                        items_match = re.search(r'\{([^}]+)\}', import_path)
                        if items_match:
                            items = [i.strip().split()[0] for i in items_match.group(1).split(',')]
                            new_items = []
                            for item in items:
                                if item and item not in imported:
                                    imported.add(item)
                                    new_items.append(item)
                            if not new_items:
                                continue  # Skip entire line
                            if len(new_items) < len(items):
                                # Rebuild the import with fewer items
                                base = '::'.join(parts[:-1])
                                new_line = f"use {base}::{{{', '.join(new_items)}}};"
                                result.append(new_line)
                                continue
                    else:
                        item_name = parts[-1].split()[0]  # Handle 'as Alias'
                        if item_name in imported:
                            continue  # Skip duplicate
                        imported.add(item_name)

        result.append(line)

    return '\n'.join(result)

def fix_underscore_variables(content):
    """Fix remaining underscore variable issues"""

    # Pattern: let _foo = ... then later foo is used
    let_pattern = re.compile(r'\blet\s+(mut\s+)?(_[a-z][a-z0-9_]*)\s*=')

    for match in let_pattern.finditer(content):
        underscore_var = match.group(2)
        plain_var = underscore_var[1:]

        # Check if plain version is used (not as definition)
        if re.search(rf'(?<![a-z_]){plain_var}(?![a-z0-9_])', content[match.end():]):
            # Replace all occurrences of the underscore version with plain
            content = content[:match.start(2)] + plain_var + content[match.end(2):]

    # Handle the specific patterns we know about
    replacements = [
        (r'\blet _value\b', 'let value'),
        (r'\blet mut _value\b', 'let mut value'),
        (r'\blet _writes_count\b', 'let writes_count'),
        (r'\blet mut _writes_count\b', 'let mut writes_count'),
        (r'\blet _flush_count\b', 'let flush_count'),
        (r'\blet mut _flush_count\b', 'let mut flush_count'),
        (r'\blet _result\b', 'let result'),
        (r'\blet mut _result\b', 'let mut result'),
    ]

    for pattern, replacement in replacements:
        content = re.sub(pattern, replacement, content)

    return content

def fix_broken_format_strings(content):
    """Fix format strings that got corrupted"""
    # Pattern: format!("...", var; should be format!("...", var)
    content = re.sub(r'(format!\([^)]+),\s*(\w+);', r'\1, \2)', content)
    return content

def fix_function_param_underscores(content):
    """Fix function parameters with underscore that are used"""

    # Find function definitions
    fn_pattern = re.compile(r'fn\s+\w+[^{]*\(([^)]*)\)[^{]*\{', re.DOTALL)

    for match in fn_pattern.finditer(content):
        params_str = match.group(1)
        fn_start = match.start()

        # Find underscore parameters
        param_matches = re.findall(r'(_[a-z][a-z0-9_]*)\s*:', params_str)

        for underscore_param in param_matches:
            plain_param = underscore_param[1:]

            # Find the end of this function (matching braces)
            brace_count = 1
            pos = match.end()
            while pos < len(content) and brace_count > 0:
                if content[pos] == '{':
                    brace_count += 1
                elif content[pos] == '}':
                    brace_count -= 1
                pos += 1

            fn_body = content[match.end():pos]

            # Check if plain param is used in body
            if re.search(rf'\b{plain_param}\b', fn_body):
                # Replace underscore param with plain in signature
                new_params = params_str.replace(f'{underscore_param}:', f'{plain_param}:')
                content = content[:fn_start] + content[fn_start:].replace(params_str, new_params, 1)

    return content

def process_file(filepath):
    """Process a single file"""
    try:
        with open(filepath, 'r') as f:
            content = f.read()

        original = content

        content = remove_duplicate_use_items(content)
        content = fix_underscore_variables(content)
        content = fix_broken_format_strings(content)
        content = fix_function_param_underscores(content)

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
