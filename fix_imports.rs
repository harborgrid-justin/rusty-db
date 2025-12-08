use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn needs_mutex_import(content: &str) -> bool {
    (content.contains("Mutex::new(") || content.contains("Mutex<")) &&
    !(content.contains("use parking_lot::Mutex") ||
      content.contains("use std::sync::Mutex") ||
      (content.contains("use parking_lot::{") && content.contains("Mutex")) ||
      (content.contains("use std::sync::{") && content.contains("Mutex")))
}

fn needs_sleep_import(content: &str) -> bool {
    content.contains("sleep(") &&
    !(content.contains("use tokio::time::sleep") ||
      (content.contains("use tokio::time::{") && content.contains("sleep")))
}

fn needs_interval_import(content: &str) -> bool {
    content.contains("interval(") &&
    !(content.contains("use tokio::time::interval") ||
      (content.contains("use tokio::time::{") && content.contains("interval")))
}

fn has_parking_lot(content: &str) -> bool {
    content.contains("use parking_lot::")
}

fn fix_file(path: &Path) -> Result<bool, std::io::Error> {
    let content = fs::read_to_string(path)?;
    let mut lines: Vec<&str> = content.lines().collect();

    let needs_mutex = needs_mutex_import(&content);
    let needs_sleep = needs_sleep_import(&content);
    let needs_interval = needs_interval_import(&content);

    if !needs_mutex && !needs_sleep && !needs_interval {
        return Ok(false);
    }

    let mut changes = Vec::new();
    let mut modified = false;

    // Find the last `use` statement
    let mut last_use_idx = 0;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") {
            last_use_idx = i;
        }
    }

    // Fix Mutex import
    if needs_mutex {
        let use_parking_lot = has_parking_lot(&content);
        let mut mutex_added = false;

        for i in 0..lines.len() {
            let line = lines[i];

            if use_parking_lot {
                if line.trim() == "use parking_lot::RwLock;" {
                    lines[i] = "use parking_lot::{RwLock, Mutex};";
                    changes.push("Mutex (parking_lot)");
                    mutex_added = true;
                    modified = true;
                    break;
                } else if line.contains("use parking_lot::{") && !line.contains("Mutex") {
                    let new_line = line.replace("}", ", Mutex}");
                    lines[i] = Box::leak(new_line.into_boxed_str());
                    changes.push("Mutex (parking_lot)");
                    mutex_added = true;
                    modified = true;
                    break;
                }
            } else {
                if line.contains("use std::sync::{") && line.contains("Arc") && !line.contains("Mutex") {
                    let new_line = line.replace("}", ", Mutex}");
                    lines[i] = Box::leak(new_line.into_boxed_str());
                    changes.push("Mutex (std::sync)");
                    mutex_added = true;
                    modified = true;
                    break;
                }
            }
        }

        if !mutex_added {
            let import_line = if use_parking_lot {
                "use parking_lot::Mutex;"
            } else {
                "use std::sync::Mutex;"
            };
            lines.insert(last_use_idx + 1, Box::leak(import_line.to_string().into_boxed_str()));
            changes.push(if use_parking_lot { "Mutex (parking_lot)" } else { "Mutex (std::sync)" });
            modified = true;
        }
    }

    // Fix sleep import
    if needs_sleep {
        let mut sleep_added = false;

        for i in 0..lines.len() {
            let line = lines[i];

            if line.contains("use tokio::time::{") && !line.contains("sleep") {
                let new_line = line.replace("}", ", sleep}");
                lines[i] = Box::leak(new_line.into_boxed_str());
                changes.push("sleep");
                sleep_added = true;
                modified = true;
                break;
            }
        }

        if !sleep_added {
            lines.insert(last_use_idx + 1, "use tokio::time::sleep;");
            changes.push("sleep");
            modified = true;
        }
    }

    // Fix interval import
    if needs_interval {
        let mut interval_added = false;

        for i in 0..lines.len() {
            let line = lines[i];

            if line.contains("use tokio::time::{") && !line.contains("interval") {
                let new_line = line.replace("}", ", interval}");
                lines[i] = Box::leak(new_line.into_boxed_str());
                changes.push("interval");
                interval_added = true;
                modified = true;
                break;
            } else if line.trim() == "use tokio::time::sleep;" {
                lines[i] = "use tokio::time::{sleep, interval};";
                changes.push("interval");
                interval_added = true;
                modified = true;
                break;
            }
        }

        if !interval_added {
            lines.insert(last_use_idx + 1, "use tokio::time::interval;");
            changes.push("interval");
            modified = true;
        }
    }

    if modified {
        let new_content = lines.join("\n") + "\n";
        fs::write(path, new_content)?;
        println!("✓ {}: Added {}", path.display(), changes.join(", "));
    }

    Ok(modified)
}

fn main() {
    let mut fixed_count = 0;

    for entry in WalkDir::new("src").follow_links(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error walking directory: {}", e);
                continue;
            }
        };

        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            match fix_file(path) {
                Ok(true) => fixed_count += 1,
                Ok(false) => {},
                Err(e) => eprintln!("Error processing {}: {}", path.display(), e),
            }
        }
    }

    println!("\nFixed {} files", fixed_count);
}
