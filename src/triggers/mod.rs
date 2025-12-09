use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

// Trigger timing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerTiming {
    Before,
    After,
}

// Trigger event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
}

// Trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    pub table: String,
    pub timing: TriggerTiming,
    pub event: TriggerEvent,
    pub condition: Option<String>,
    pub action: String,
    pub enabled: bool,
}

// Trigger context for execution
#[derive(Debug, Clone)]
pub struct TriggerContext {
    pub old_values: Option<HashMap<String, String>>,
    pub new_values: Option<HashMap<String, String>>,
}

// Trigger manager
pub struct TriggerManager {
    triggers: Arc<RwLock<HashMap<String, Vec<Trigger>>>>,
}

impl TriggerManager {
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Create a new trigger
    pub fn create_trigger(&self, trigger: Trigger) -> Result<()> {
        let mut triggers = self.triggers.write();
        triggers.entry(trigger.table.clone())
            .or_insert_with(Vec::new)
            .push(trigger);
        Ok(())
    }

    // Drop a trigger by name
    pub fn drop_trigger(&self, name: &str) -> Result<()> {
        let mut triggers = self.triggers.write();
        for (_, table_triggers) in triggers.iter_mut() {
            table_triggers.retain(|t| t.name != name);
        }
        Ok(())
    }

    // Enable/disable a trigger
    pub fn set_trigger_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let mut triggers = self.triggers.write();
        for (_, table_triggers) in triggers.iter_mut() {
            for trigger in table_triggers {
                if trigger.name == name {
                    trigger.enabled = enabled;
                    return Ok(());
                }
            }
        }
        Err(DbError::NotFound(format!("Trigger '{}' not found", name)))
    }

    // Get all triggers for a table
    pub fn get_triggers(&self, table: &str) -> Vec<Trigger> {
        let triggers = self.triggers.read();
        triggers.get(table)
            .map(|t| t.clone())
            .unwrap_or_default()
    }

    // Execute triggers for a specific event
    pub fn execute_triggers(
        &self,
        table: &str,
        event: TriggerEvent,
        timing: TriggerTiming,
        context: &TriggerContext,
    ) -> Result<()> {
        let triggers = self.triggers.read();

        if let Some(table_triggers) = triggers.get(table) {
            for trigger in table_triggers {
                if trigger.enabled
                    && trigger.event == event
                    && trigger.timing == timing
                {
                    // Check condition if present
                    if let Some(condition) = &trigger.condition {
                        if !self.evaluate_condition(condition, context)? {
                            continue;
                        }
                    }

                    // Execute trigger action
                    self.execute_action(&trigger.action, context)?;
                }
            }
        }

        Ok(())
    }

    // Evaluate a trigger condition
    fn evaluate_condition(&self, condition: &str, context: &TriggerContext) -> Result<bool> {
        // Parse and evaluate the condition expression
        // Supports simple conditions like: NEW.column = value, OLD.column <> value
        let condition = condition.trim();

        // Handle NEW.column references
        if condition.starts_with("NEW.") {
            if let Some(new_values) = &context.new_values {
                return self.evaluate_column_condition(condition, "NEW.", new_values);
            }
            return Ok(false);
        }

        // Handle OLD.column references
        if condition.starts_with("OLD.") {
            if let Some(old_values) = &context.old_values {
                return self.evaluate_column_condition(condition, "OLD.", old_values);
            }
            return Ok(false);
        }

        // Handle comparison between NEW and OLD
        if condition.contains("NEW.") && condition.contains("OLD.") {
            return self.evaluate_new_old_comparison(condition, context);
        }

        // Default: condition evaluates to true if non-empty
        Ok(!condition.is_empty())
    }

    // Helper to evaluate column conditions like "NEW.status = 'active'"
    fn evaluate_column_condition(
        &self,
        condition: &str,
        prefix: &str,
        values: &HashMap<String, String>,
    ) -> Result<bool> {
        // Parse: PREFIX.column operator value
        let rest = condition.strip_prefix(prefix).unwrap_or(condition);

        // Find operator
        let operators = ["<>", "!=", ">=", "<=", "=", ">", "<", " IS NOT NULL", " IS NULL"];
        for op in operators {
            if let Some(pos) = rest.find(op) {
                let column = rest[..pos].trim();
                let value = rest[pos + op.len()..].trim().trim_matches('\'');

                let actual = values.get(column).map(|s| s.as_str()).unwrap_or("");

                return Ok(match op {
                    "=" => actual == value,
                    "<>" | "!=" => actual != value,
                    ">" => actual > value,
                    "<" => actual < value,
                    ">=" => actual >= value,
                    "<=" => actual <= value,
                    " IS NULL" => actual.is_empty(),
                    " IS NOT NULL" => !actual.is_empty(),
                    _ => false,
                });
            }
        }

        Ok(false)
    }

    // Helper to compare NEW and OLD values
    fn evaluate_new_old_comparison(&self, condition: &str, context: &TriggerContext) -> Result<bool> {
        // Handle: NEW.column <> OLD.column (detect changes)
        if let (Some(new_values), Some(old_values)) = (&context.new_values, &context.old_values) {
            // Parse NEW.col <> OLD.col pattern
            if let Some(neq_pos) = condition.find("<>").or_else(|| condition.find("!=")) {
                let left = condition[..neq_pos].trim();
                let right = condition[neq_pos + 2..].trim();

                let left_val = if left.starts_with("NEW.") {
                    new_values.get(left.strip_prefix("NEW.").unwrap_or(""))
                } else if left.starts_with("OLD.") {
                    old_values.get(left.strip_prefix("OLD.").unwrap_or(""))
                } else {
                    None
                };

                let right_val = if right.starts_with("NEW.") {
                    new_values.get(right.strip_prefix("NEW.").unwrap_or(""))
                } else if right.starts_with("OLD.") {
                    old_values.get(right.strip_prefix("OLD.").unwrap_or(""))
                } else {
                    None
                };

                return Ok(left_val != right_val);
            }

            // Handle = operator
            if let Some(eq_pos) = condition.find('=') {
                let left = condition[..eq_pos].trim();
                let right = condition[eq_pos + 1..].trim();

                let left_val = if left.starts_with("NEW.") {
                    new_values.get(left.strip_prefix("NEW.").unwrap_or(""))
                } else if left.starts_with("OLD.") {
                    old_values.get(left.strip_prefix("OLD.").unwrap_or(""))
                } else {
                    None
                };

                let right_val = if right.starts_with("NEW.") {
                    new_values.get(right.strip_prefix("NEW.").unwrap_or(""))
                } else if right.starts_with("OLD.") {
                    old_values.get(right.strip_prefix("OLD.").unwrap_or(""))
                } else {
                    None
                };

                return Ok(left_val == right_val);
            }
        }

        Ok(false)
    }

    // Execute a trigger action
    fn execute_action(&self, action: &str, context: &TriggerContext) -> Result<()> {
        // Parse and execute SQL statements in the trigger action
        // Replace :NEW and :OLD references with actual values
        let mut sql = action.to_string();

        // Replace :NEW.column references
        if let Some(new_values) = &context.new_values {
            for (column, value) in new_values {
                let placeholder = format!(":NEW.{}", column);
                let replacement = format!("'{}'", value.replace('\'', "''"));
                sql = sql.replace(&placeholder, &replacement);

                // Also handle NEW.column without colon
                let placeholder_no_colon = format!("NEW.{}", column);
                sql = sql.replace(&placeholder_no_colon, &replacement);
            }
        }

        // Replace :OLD.column references
        if let Some(old_values) = &context.old_values {
            for (column, value) in old_values {
                let placeholder = format!(":OLD.{}", column);
                let replacement = format!("'{}'", value.replace('\'', "''"));
                sql = sql.replace(&placeholder, &replacement);

                // Also handle OLD.column without colon
                let placeholder_no_colon = format!("OLD.{}", column);
                sql = sql.replace(&placeholder_no_colon, &replacement);
            }
        }

        // Log the action for debugging (in production, execute via query engine)
        if !sql.trim().is_empty() {
            // In a full implementation, this would:
            // 1. Parse the SQL statement
            // 2. Execute it through the query engine
            // 3. Handle any errors and potentially rollback
            log::debug!("Trigger action: {}", sql);
        }

        Ok(())
    }
}

impl Default for TriggerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_trigger() -> Result<()> {
        let tm = TriggerManager::new();

        let trigger = Trigger {
            name: "audit_insert".to_string(),
            table: "users".to_string(),
            timing: TriggerTiming::After,
            event: TriggerEvent::Insert,
            condition: None,
            action: "INSERT INTO audit_log VALUES (...)".to_string(),
            enabled: true,
        };

        tm.create_trigger(trigger)?;

        let triggers = tm.get_triggers("users");
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].name, "audit_insert");

        Ok(())
    }

    #[test]
    fn test_drop_trigger() -> Result<()> {
        let tm = TriggerManager::new();

        let trigger = Trigger {
            name: "test_trigger".to_string(),
            table: "products".to_string(),
            timing: TriggerTiming::Before,
            event: TriggerEvent::Update,
            condition: None,
            action: "".to_string(),
            enabled: true,
        };

        tm.create_trigger(trigger)?;
        assert_eq!(tm.get_triggers("products").len(), 1);

        tm.drop_trigger("test_trigger")?;
        assert_eq!(tm.get_triggers("products").len(), 0);

        Ok(())
    }

    #[test]
    fn test_disable_trigger() -> Result<()> {
        let tm = TriggerManager::new();

        let trigger = Trigger {
            name: "test_trigger".to_string(),
            table: "orders".to_string(),
            timing: TriggerTiming::Before,
            event: TriggerEvent::Delete,
            condition: None,
            action: "".to_string(),
            enabled: true,
        };

        tm.create_trigger(trigger)?;
        tm.set_trigger_enabled("test_trigger", false)?;

        let triggers = tm.get_triggers("orders");
        assert!(!triggers[0].enabled);

        Ok(())
    }
}
