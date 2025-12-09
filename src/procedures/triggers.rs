// Trigger System for RustyDB
//
// This module provides a comprehensive trigger system supporting BEFORE, AFTER,
// and INSTEAD OF triggers at both ROW and STATEMENT levels, with dependency
// tracking and ordering capabilities.

use std::collections::HashSet;
use crate::{Result, DbError};
use crate::procedures::parser::PlSqlBlock;
use crate::procedures::runtime::{RuntimeExecutor, RuntimeValue, ExecutionContext};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use parking_lot::RwLock;

// Trigger timing (when the trigger fires)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TriggerTiming {
    // Fire before the DML operation
    Before,
    // Fire after the DML operation
    After,
    // Fire instead of the DML operation (for views)
    InsteadOf,
}

// DML operation that triggers the event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
}

// Trigger level (row or statement)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerLevel {
    // Fire once for each row affected
    Row,
    // Fire once for the entire statement
    Statement,
}

// Trigger condition (WHEN clause)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub expression: String,
}

// Reference to OLD and NEW row values
#[derive(Debug, Clone)]
pub struct TriggerRowContext {
    pub old_values: Option<HashMap<String, RuntimeValue>>,
    pub new_values: Option<HashMap<String, RuntimeValue>>,
}

impl TriggerRowContext {
    pub fn new() -> Self {
        Self {
            old_values: None,
            new_values: None,
        }
    }

    pub fn with_old(old: HashMap<String, RuntimeValue>) -> Self {
        Self {
            old_values: Some(old),
            new_values: None,
        }
    }

    pub fn with_new(new: HashMap<String, RuntimeValue>) -> Self {
        Self {
            old_values: None,
            new_values: Some(new),
        }
    }

    pub fn with_old_new(old: HashMap<String, RuntimeValue>, new: HashMap<String, RuntimeValue>) -> Self {
        Self {
            old_values: Some(old),
            new_values: Some(new),
        }
    }
}

impl Default for TriggerRowContext {
    fn default() -> Self {
        Self::new()
    }
}

// Trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    pub table_name: String,
    pub timing: TriggerTiming,
    pub events: Vec<TriggerEvent>,
    pub level: TriggerLevel,
    pub condition: Option<TriggerCondition>,
    pub body: PlSqlBlock,
    pub enabled: bool,
    pub order: i32,
    pub follows: Option<String>,
    pub precedes: Option<String>,
}

impl Trigger {
    // Check if trigger should fire for a given event
    pub fn should_fire(&self, event: &TriggerEvent, timing: &TriggerTiming) -> bool {
        self.enabled && self.timing == *timing && self.events.contains(event)
    }

    // Evaluate the WHEN condition
    pub fn evaluate_condition(&self, context: &TriggerRowContext) -> Result<bool> {
        if let Some(_cond) = &self.condition {
            // TODO: Evaluate condition expression
            // For now, always return true
            Ok(true)
        } else {
            Ok(true)
        }
    }
}

// Trigger execution result
#[derive(Debug, Clone)]
pub struct TriggerExecutionResult {
    pub success: bool,
    pub modified_values: Option<HashMap<String, RuntimeValue>>,
    pub error: Option<String>,
}

// Trigger manager
pub struct TriggerManager {
    triggers: Arc<RwLock<HashMap<String, Trigger>>>,
    table_triggers: Arc<RwLock<HashMap<String, Vec<String>>>>,
    runtime: Arc<RuntimeExecutor>,
}

impl TriggerManager {
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(RwLock::new(HashMap::new())),
            table_triggers: Arc::new(RwLock::new(HashMap::new())),
            runtime: Arc::new(RuntimeExecutor::new()),
        }
    }

    // Create a new trigger
    pub fn create_trigger(&self, trigger: Trigger) -> Result<()> {
        let mut triggers = self.triggers.write();
        let mut table_triggers = self.table_triggers.write();

        if triggers.contains_key(&trigger.name) {
            return Err(DbError::AlreadyExists(
                format!("Trigger '{}' already exists", trigger.name)
            ));
        }

        // Validate ordering constraints
        if let Some(ref follows) = trigger.follows {
            if !triggers.contains_key(follows) {
                return Err(DbError::NotFound(
                    format!("Trigger '{}' (specified in FOLLOWS) not found", follows)
                ));
            }
        }

        if let Some(ref precedes) = trigger.precedes {
            if !triggers.contains_key(precedes) {
                return Err(DbError::NotFound(
                    format!("Trigger '{}' (specified in PRECEDES) not found", precedes)
                ));
            }
        }

        // Add to table triggers index
        table_triggers
            .entry(trigger.table_name.clone())
            .or_insert_with(Vec::new)
            .push(trigger.name.clone());

        triggers.insert(trigger.name.clone(), trigger);
        Ok(())
    }

    // Drop a trigger
    pub fn drop_trigger(&self, name: &str) -> Result<()> {
        let mut triggers = self.triggers.write();
        let mut table_triggers = self.table_triggers.write();

        let trigger = triggers.remove(name).ok_or_else(||
            DbError::NotFound(format!("Trigger '{}' not found", name))
        )?;

        // Remove from table triggers index
        if let Some(table_trigs) = table_triggers.get_mut(&trigger.table_name) {
            table_trigs.retain(|t| t != name);
        }

        Ok(())
    }

    // Enable a trigger
    pub fn enable_trigger(&self, name: &str) -> Result<()> {
        let mut triggers = self.triggers.write();

        let trigger = triggers.get_mut(name).ok_or_else(||
            DbError::NotFound(format!("Trigger '{}' not found", name))
        )?;

        trigger.enabled = true;
        Ok(())
    }

    // Disable a trigger
    pub fn disable_trigger(&self, name: &str) -> Result<()> {
        let mut triggers = self.triggers.write();

        let trigger = triggers.get_mut(name).ok_or_else(||
            DbError::NotFound(format!("Trigger '{}' not found", name))
        )?;

        trigger.enabled = false;
        Ok(())
    }

    // Get all triggers for a table
    pub fn get_table_triggers(&self, table_name: &str) -> Vec<Trigger> {
        let triggers = self.triggers.read();
        let table_triggers = self.table_triggers.read();

        if let Some(trigger_names) = table_triggers.get(table_name) {
            trigger_names.iter()
                .filter_map(|name| triggers.get(name).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    // Get triggers for a specific event and timing
    pub fn get_triggers_for_event(
        &self,
        table_name: &str,
        event: &TriggerEvent,
        timing: &TriggerTiming,
    ) -> Vec<Trigger> {
        let table_triggers = self.get_table_triggers(table_name);

        let mut matching_triggers: Vec<Trigger> = table_triggers.into_iter()
            .filter(|t| t.should_fire(event, timing))
            .collect();

        // Sort by order
        self.sort_triggers(&mut matching_triggers);

        matching_triggers
    }

    // Sort triggers by order and dependencies
    fn sort_triggers(&self, triggers: &mut [Trigger]) {
        // Build dependency graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for trigger in triggers.iter() {
            in_degree.insert(trigger.name.clone(), 0);
            graph.insert(trigger.name.clone(), Vec::new());
        }

        for trigger in triggers.iter() {
            if let Some(ref precedes) = trigger.precedes {
                if graph.contains_key(precedes) {
                    graph.get_mut(&trigger.name).unwrap().push(precedes.clone());
                    *in_degree.get_mut(precedes).unwrap() += 1;
                }
            }

            if let Some(ref follows) = trigger.follows {
                if graph.contains_key(follows) {
                    graph.get_mut(follows).unwrap().push(trigger.name.clone());
                    *in_degree.get_mut(&trigger.name).unwrap() += 1;
                }
            }
        }

        // Topological sort
        let mut queue: Vec<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut sorted_names = Vec::new();

        while let Some(name) = queue.pop() {
            sorted_names.push(name.clone());

            if let Some(neighbors) = graph.get(&name) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Reorder triggers based on sorted names and explicit order
        triggers.sort_by(|a, b| {
            let pos_a = sorted_names.iter().position(|n| n == &a.name).unwrap_or(usize::MAX);
            let pos_b = sorted_names.iter().position(|n| n == &b.name).unwrap_or(usize::MAX);

            if pos_a == pos_b {
                a.order.cmp(&b.order)
            } else {
                pos_a.cmp(&pos_b)
            }
        });
    }

    // Fire ROW-level triggers
    pub fn fire_row_triggers(
        &self,
        table_name: &str,
        event: &TriggerEvent,
        timing: &TriggerTiming,
        row_context: &TriggerRowContext,
    ) -> Result<Vec<TriggerExecutionResult>> {
        let triggers = self.get_triggers_for_event(table_name, event, timing);

        let mut results = Vec::new();

        for trigger in triggers {
            if trigger.level != TriggerLevel::Row {
                continue;
            }

            // Evaluate condition
            if !trigger.evaluate_condition(row_context)? {
                continue;
            }

            // Execute trigger body
            match self.runtime.execute(&trigger.body) {
                Ok(_result) => {
                    results.push(TriggerExecutionResult {
                        success: true,
                        modified_values: None,
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(TriggerExecutionResult {
                        success: false,
                        modified_values: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }

    // Fire STATEMENT-level triggers
    pub fn fire_statement_triggers(
        &self,
        table_name: &str,
        event: &TriggerEvent,
        timing: &TriggerTiming,
    ) -> Result<Vec<TriggerExecutionResult>> {
        let triggers = self.get_triggers_for_event(table_name, event, timing);

        let mut results = Vec::new();

        for trigger in triggers {
            if trigger.level != TriggerLevel::Statement {
                continue;
            }

            // Execute trigger body
            match self.runtime.execute(&trigger.body) {
                Ok(_result) => {
                    results.push(TriggerExecutionResult {
                        success: true,
                        modified_values: None,
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(TriggerExecutionResult {
                        success: false,
                        modified_values: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }

    // List all triggers
    pub fn list_triggers(&self) -> Vec<String> {
        let triggers = self.triggers.read();
        triggers.keys().cloned().collect()
    }

    // Get trigger by name
    pub fn get_trigger(&self, name: &str) -> Result<Trigger> {
        let triggers = self.triggers.read();
        triggers.get(name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Trigger '{}' not found", name)
            ))
    }

    // Validate trigger dependencies (check for cycles)
    pub fn validate_dependencies(&self) -> Result<()> {
        let triggers = self.triggers.read();

        for trigger in triggers.values() {
            let mut visited = HashSet::new();
            self.check_cycle(&trigger.name, &mut visited, &triggers)?;
        }

        Ok(())
    }

    fn check_cycle(
        &self,
        current: &str,
        visited: &mut HashSet<String>,
        triggers: &HashMap<String, Trigger>,
    ) -> Result<()> {
        if visited.contains(current) {
            return Err(DbError::InvalidInput(
                format!("Circular trigger dependency detected involving '{}'", current)
            ));
        }

        visited.insert(current.to_string());

        if let Some(trigger) = triggers.get(current) {
            if let Some(ref follows) = trigger.follows {
                self.check_cycle(follows, visited, triggers)?;
            }

            if let Some(ref precedes) = trigger.precedes {
                self.check_cycle(precedes, visited, triggers)?;
            }
        }

        visited.remove(current);
        Ok(())
    }

    // Get trigger statistics
    pub fn get_statistics(&self, table_name: &str) -> TriggerStatistics {
        let triggers = self.get_table_triggers(table_name);

        let mut stats = TriggerStatistics::default();

        for trigger in triggers {
            if !trigger.enabled {
                stats.disabled_count += 1;
                continue;
            }

            stats.total_count += 1;

            match trigger.timing {
                TriggerTiming::Before => stats.before_count += 1,
                TriggerTiming::After => stats.after_count += 1,
                TriggerTiming::InsteadOf => stats.instead_of_count += 1,
            }

            match trigger.level {
                TriggerLevel::Row => stats.row_level_count += 1,
                TriggerLevel::Statement => stats.statement_level_count += 1,
            }

            for event in &trigger.events {
                match event {
                    TriggerEvent::Insert => stats.insert_count += 1,
                    TriggerEvent::Update => stats.update_count += 1,
                    TriggerEvent::Delete => stats.delete_count += 1,
                }
            }
        }

        stats
    }
}

impl Default for TriggerManager {
    fn default() -> Self {
        Self::new()
    }
}

// Trigger statistics
#[derive(Debug, Clone, Default)]
pub struct TriggerStatistics {
    pub total_count: usize,
    pub disabled_count: usize,
    pub before_count: usize,
    pub after_count: usize,
    pub instead_of_count: usize,
    pub row_level_count: usize,
    pub statement_level_count: usize,
    pub insert_count: usize,
    pub update_count: usize,
    pub delete_count: usize,
}

// Compound trigger (Oracle-style)
// Allows multiple timing points in a single trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundTrigger {
    pub name: String,
    pub table_name: String,
    pub events: Vec<TriggerEvent>,
    pub before_statement: Option<PlSqlBlock>,
    pub before_row: Option<PlSqlBlock>,
    pub after_row: Option<PlSqlBlock>,
    pub after_statement: Option<PlSqlBlock>,
    pub enabled: bool,
}

impl CompoundTrigger {
    // Execute the appropriate section of the compound trigger
    pub fn execute_section(
        &self,
        timing: &TriggerTiming,
        level: &TriggerLevel,
        runtime: &RuntimeExecutor,
    ) -> Result<()> {
        let section = match (timing, level) {
            (TriggerTiming::Before, TriggerLevel::Statement) => &self.before_statement,
            (TriggerTiming::Before, TriggerLevel::Row) => &self.before_row,
            (TriggerTiming::After, TriggerLevel::Row) => &self.after_row,
            (TriggerTiming::After, TriggerLevel::Statement) => &self.after_statement,
            _ => return Ok(()),
        };

        if let Some(block) = section {
            runtime.execute(block)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_trigger() -> Result<()> {
        let manager = TriggerManager::new();

        let trigger = Trigger {
            name: "audit_insert".to_string(),
            table_name: "employees".to_string(),
            timing: TriggerTiming::After,
            events: vec![TriggerEvent::Insert],
            level: TriggerLevel::Row,
            condition: None,
            body: PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            },
            enabled: true,
            order: 0,
            follows: None,
            precedes: None,
        };

        manager.create_trigger(trigger)?;

        assert_eq!(manager.list_triggers().len(), 1);

        Ok(())
    }

    #[test]
    fn test_get_table_triggers() -> Result<()> {
        let manager = TriggerManager::new();

        let trigger1 = Trigger {
            name: "audit_insert".to_string(),
            table_name: "employees".to_string(),
            timing: TriggerTiming::After,
            events: vec![TriggerEvent::Insert],
            level: TriggerLevel::Row,
            condition: None,
            body: PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            },
            enabled: true,
            order: 0,
            follows: None,
            precedes: None,
        };

        let trigger2 = Trigger {
            name: "validate_update".to_string(),
            table_name: "employees".to_string(),
            timing: TriggerTiming::Before,
            events: vec![TriggerEvent::Update],
            level: TriggerLevel::Row,
            condition: None,
            body: PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            },
            enabled: true,
            order: 0,
            follows: None,
            precedes: None,
        };

        manager.create_trigger(trigger1)?;
        manager.create_trigger(trigger2)?;

        let triggers = manager.get_table_triggers("employees");
        assert_eq!(triggers.len(), 2);

        Ok(())
    }

    #[test]
    fn test_trigger_ordering() -> Result<()> {
        let manager = TriggerManager::new();

        let trigger1 = Trigger {
            name: "trigger1".to_string(),
            table_name: "test".to_string(),
            timing: TriggerTiming::Before,
            events: vec![TriggerEvent::Insert],
            level: TriggerLevel::Row,
            condition: None,
            body: PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            },
            enabled: true,
            order: 1,
            follows: None,
            precedes: None,
        };

        manager.create_trigger(trigger1)?;

        let trigger2 = Trigger {
            name: "trigger2".to_string(),
            table_name: "test".to_string(),
            timing: TriggerTiming::Before,
            events: vec![TriggerEvent::Insert],
            level: TriggerLevel::Row,
            condition: None,
            body: PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            },
            enabled: true,
            order: 2,
            follows: Some("trigger1".to_string()),
            precedes: None,
        };

        manager.create_trigger(trigger2)?;

        let triggers = manager.get_triggers_for_event("test", &TriggerEvent::Insert, &TriggerTiming::Before);
        assert_eq!(triggers.len(), 2);
        assert_eq!(triggers[0].name, "trigger1");
        assert_eq!(triggers[1].name, "trigger2");

        Ok(())
    }

    #[test]
    fn test_enable_disable_trigger() -> Result<()> {
        let manager = TriggerManager::new();

        let trigger = Trigger {
            name: "test_trigger".to_string(),
            table_name: "test".to_string(),
            timing: TriggerTiming::Before,
            events: vec![TriggerEvent::Insert],
            level: TriggerLevel::Row,
            condition: None,
            body: PlSqlBlock {
                declarations: Vec::new(),
                statements: Vec::new(),
                exception_handlers: Vec::new(),
            },
            enabled: true,
            order: 0,
            follows: None,
            precedes: None,
        };

        manager.create_trigger(trigger)?;

        manager.disable_trigger("test_trigger")?;
        let trigger = manager.get_trigger("test_trigger")?;
        assert!(!trigger.enabled);

        manager.enable_trigger("test_trigger")?;
        let trigger = manager.get_trigger("test_trigger")?;
        assert!(trigger.enabled);

        Ok(())
    }
}
