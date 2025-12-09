use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

/// Trigger timing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerTiming {
    Before,
    After,
}

/// Trigger event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
}

/// Trigger definition
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

/// Trigger context for execution
#[derive(Debug, Clone)]
pub struct TriggerContext {
    pub old_values: Option<HashMap<String, String>>,
    pub new_values: Option<HashMap<String, String>>,
}

/// Trigger manager
pub struct TriggerManager {
    triggers: Arc<RwLock<HashMap<String, Vec<Trigger>>>>,
}

impl TriggerManager {
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create a new trigger
    pub fn create_trigger(&self, trigger: Trigger) -> Result<()> {
        let mut triggers = self.triggers.write();
        triggers.entry(trigger.table.clone())
            .or_insert_with(Vec::new)
            .push(trigger);
        Ok(())
    }
    
    /// Drop a trigger by name
    pub fn drop_trigger(&self, name: &str) -> Result<()> {
        let mut triggers = self.triggers.write();
        for (_, table_triggers) in triggers.iter_mut() {
            table_triggers.retain(|t| t.name != name);
        }
        Ok(())
    }
    
    /// Enable/disable a trigger
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
    
    /// Get all triggers for a table
    pub fn get_triggers(&self, table: &str) -> Vec<Trigger> {
        let triggers = self.triggers.read()));
        triggers.get(table)
            .map(|t| t.clone())
            .unwrap_or_default()
    }
    
    /// Execute triggers for a specific event
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
    
    /// Evaluate a trigger condition
    fn evaluate_condition(&self, condition: &str, _context: &TriggerContext) -> Result<bool> {
        // TODO: Implement condition evaluation
        // For now, always return true
        Ok(true)
    }
    
    /// Execute a trigger action
    fn execute_action(&self, action: &str, _context: &TriggerContext) -> Result<()> {
        // TODO: Implement action execution
        // This would typically parse and execute SQL statements
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


