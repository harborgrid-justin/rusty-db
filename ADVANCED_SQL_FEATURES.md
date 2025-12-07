# Advanced SQL Features Documentation

This document describes the advanced SQL features implemented in RustyDB, including query optimization, joins, aggregations, triggers, stored procedures, and replication.

## Table of Contents
1. [Query Optimization](#query-optimization)
2. [Join Operations](#join-operations)
3. [Aggregation Functions](#aggregation-functions)
4. [Subqueries](#subqueries)
5. [Foreign Key Constraints](#foreign-key-constraints)
6. [Triggers](#triggers)
7. [Stored Procedures](#stored-procedures)
8. [Replication and High Availability](#replication-and-high-availability)

---

## Query Optimization

RustyDB implements a cost-based query optimizer that applies various optimization techniques to improve query performance.

### Optimization Strategies

#### 1. Predicate Pushdown
Filters are pushed down closer to table scans to reduce the amount of data processed early in the query execution.

```sql
-- Before optimization
SELECT * FROM (SELECT * FROM users) WHERE age > 25

-- After predicate pushdown
SELECT * FROM users WHERE age > 25
```

#### 2. Join Reordering
The optimizer reorders joins based on estimated costs, placing smaller tables on the right side for hash joins.

#### 3. Cost-Based Optimization
Each query plan node has an associated cost estimate:
- **Table Scan**: Base cost of 1000 (estimated rows)
- **Filter**: Reduces cost by 50% (selectivity estimate)
- **Join**: Product of input costs × 0.1
- **Aggregate**: Input cost × 1.2
- **Sort**: Input cost × log(input cost)
- **Limit**: min(input cost, limit value)

### Usage

```rust
use rusty_db::execution::{Planner, Optimizer};

let planner = Planner::new();
let optimizer = Optimizer::new();

// Create a query plan
let plan = planner.plan(&sql_statement)?;

// Optimize the plan
let optimized_plan = optimizer.optimize(plan)?;
```

---

## Join Operations

RustyDB supports all standard SQL join types.

### Join Types

#### INNER JOIN
Returns only rows that have matching values in both tables.

```sql
SELECT orders.id, customers.name
FROM orders
INNER JOIN customers ON orders.customer_id = customers.id;
```

#### LEFT OUTER JOIN
Returns all rows from the left table and matching rows from the right table. NULL values are used for non-matching rows.

```sql
SELECT customers.name, orders.id
FROM customers
LEFT JOIN orders ON customers.id = orders.customer_id;
```

#### RIGHT OUTER JOIN
Returns all rows from the right table and matching rows from the left table.

```sql
SELECT customers.name, orders.id
FROM customers
RIGHT JOIN orders ON customers.id = orders.customer_id;
```

#### FULL OUTER JOIN
Returns all rows from both tables, using NULL values where there are no matches.

```sql
SELECT customers.name, orders.id
FROM customers
FULL OUTER JOIN orders ON customers.id = orders.customer_id;
```

#### CROSS JOIN
Returns the Cartesian product of both tables.

```sql
SELECT products.name, categories.name
FROM products
CROSS JOIN categories;
```

### Implementation

The executor implements joins with the following logic:
- **INNER JOIN**: Nested loop join with condition checking
- **LEFT/RIGHT/FULL JOIN**: Includes NULL padding for non-matching rows
- **CROSS JOIN**: Simple Cartesian product

---

## Aggregation Functions

RustyDB supports standard SQL aggregation functions with GROUP BY and HAVING clauses.

### Supported Aggregate Functions

| Function | Description | Example |
|----------|-------------|---------|
| `COUNT(*)` | Count all rows | `SELECT COUNT(*) FROM users` |
| `COUNT(column)` | Count non-NULL values | `SELECT COUNT(email) FROM users` |
| `SUM(column)` | Sum numeric values | `SELECT SUM(price) FROM orders` |
| `AVG(column)` | Average of values | `SELECT AVG(salary) FROM employees` |
| `MIN(column)` | Minimum value | `SELECT MIN(age) FROM users` |
| `MAX(column)` | Maximum value | `SELECT MAX(salary) FROM employees` |
| `STDDEV(column)` | Standard deviation | `SELECT STDDEV(score) FROM tests` |
| `VARIANCE(column)` | Statistical variance | `SELECT VARIANCE(score) FROM tests` |

### GROUP BY

Group rows by one or more columns and apply aggregate functions.

```sql
SELECT department, COUNT(*), AVG(salary)
FROM employees
GROUP BY department;
```

### HAVING

Filter grouped results based on aggregate conditions.

```sql
SELECT department, COUNT(*) as emp_count
FROM employees
GROUP BY department
HAVING COUNT(*) > 10;
```

### Usage Example

```rust
use rusty_db::execution::planner::{AggregateExpr, AggregateFunction};

// Create aggregate expression
let agg = AggregateExpr {
    function: AggregateFunction::Count,
    column: "*".to_string(),
    alias: Some("total".to_string()),
};
```

---

## Subqueries

RustyDB supports subqueries in various contexts.

### Subquery in WHERE Clause

```sql
SELECT name FROM employees
WHERE department_id IN (SELECT id FROM departments WHERE location = 'NYC');
```

### Subquery in FROM Clause

```sql
SELECT dept.name, emp_count.total
FROM departments dept
JOIN (SELECT department_id, COUNT(*) as total FROM employees GROUP BY department_id) emp_count
ON dept.id = emp_count.department_id;
```

### Implementation

Subqueries are represented as `PlanNode::Subquery` in the query plan and are executed recursively.

```rust
PlanNode::Subquery {
    plan: Box::new(subquery_plan),
    alias: "subquery_alias".to_string(),
}
```

---

## Foreign Key Constraints

Enhanced foreign key support with full referential integrity enforcement.

### Referential Actions

#### CASCADE
Automatically delete or update dependent rows when the referenced row is deleted or updated.

```sql
ALTER TABLE orders
ADD CONSTRAINT fk_customer
FOREIGN KEY (customer_id)
REFERENCES customers(id)
ON DELETE CASCADE
ON UPDATE CASCADE;
```

#### SET NULL
Set the foreign key to NULL when the referenced row is deleted or updated.

```sql
ALTER TABLE orders
ADD CONSTRAINT fk_customer
FOREIGN KEY (customer_id)
REFERENCES customers(id)
ON DELETE SET NULL;
```

#### RESTRICT
Prevent deletion or update of referenced rows if dependent rows exist.

```sql
ALTER TABLE orders
ADD CONSTRAINT fk_customer
FOREIGN KEY (customer_id)
REFERENCES customers(id)
ON DELETE RESTRICT;
```

### Usage

```rust
use rusty_db::constraints::{ForeignKey, ReferentialAction, ConstraintManager};

let cm = ConstraintManager::new();

let fk = ForeignKey {
    name: "fk_order_customer".to_string(),
    table: "orders".to_string(),
    columns: vec!["customer_id".to_string()],
    referenced_table: "customers".to_string(),
    referenced_columns: vec!["id".to_string()],
    on_delete: ReferentialAction::Cascade,
    on_update: ReferentialAction::Cascade,
};

cm.add_foreign_key(fk)?;

// Handle cascade operations
let actions = cm.cascade_operation("customers", "DELETE", &values)?;
```

---

## Triggers

Triggers allow you to execute custom logic automatically before or after INSERT, UPDATE, or DELETE operations.

### Trigger Timing
- **BEFORE**: Execute before the operation
- **AFTER**: Execute after the operation

### Trigger Events
- **INSERT**: Triggered on row insertion
- **UPDATE**: Triggered on row update
- **DELETE**: Triggered on row deletion

### Creating Triggers

```rust
use rusty_db::triggers::{Trigger, TriggerManager, TriggerTiming, TriggerEvent};

let tm = TriggerManager::new();

let trigger = Trigger {
    name: "audit_user_insert".to_string(),
    table: "users".to_string(),
    timing: TriggerTiming::After,
    event: TriggerEvent::Insert,
    condition: None,
    action: "INSERT INTO audit_log (table_name, operation, timestamp) VALUES ('users', 'INSERT', NOW())".to_string(),
    enabled: true,
};

tm.create_trigger(trigger)?;
```

### Executing Triggers

```rust
use rusty_db::triggers::TriggerContext;

let context = TriggerContext {
    old_values: None,
    new_values: Some(new_row_values),
};

tm.execute_triggers("users", TriggerEvent::Insert, TriggerTiming::After, &context)?;
```

### Managing Triggers

```rust
// Disable a trigger
tm.set_trigger_enabled("audit_user_insert", false)?;

// Drop a trigger
tm.drop_trigger("audit_user_insert")?;

// List triggers for a table
let triggers = tm.get_triggers("users");
```

---

## Stored Procedures

Stored procedures allow you to encapsulate reusable SQL logic.

### Parameter Modes
- **IN**: Input parameter
- **OUT**: Output parameter
- **INOUT**: Both input and output

### Creating Stored Procedures

```rust
use rusty_db::procedures::{StoredProcedure, ProcedureParameter, ParameterMode, ProcedureLanguage, ProcedureManager};

let pm = ProcedureManager::new();

let procedure = StoredProcedure {
    name: "calculate_discount".to_string(),
    parameters: vec![
        ProcedureParameter {
            name: "price".to_string(),
            data_type: "FLOAT".to_string(),
            mode: ParameterMode::In,
        },
        ProcedureParameter {
            name: "discount_rate".to_string(),
            data_type: "FLOAT".to_string(),
            mode: ParameterMode::In,
        },
        ProcedureParameter {
            name: "final_price".to_string(),
            data_type: "FLOAT".to_string(),
            mode: ParameterMode::Out,
        },
    ],
    body: "SET final_price = price * (1 - discount_rate);".to_string(),
    language: ProcedureLanguage::Sql,
};

pm.create_procedure(procedure)?;
```

### Executing Stored Procedures

```rust
use rusty_db::procedures::ProcedureContext;

let mut params = HashMap::new();
params.insert("price".to_string(), "100.0".to_string());
params.insert("discount_rate".to_string(), "0.15".to_string());

let context = ProcedureContext { parameters: params };

let result = pm.execute_procedure("calculate_discount", &context)?;
println!("Final price: {}", result.output_parameters.get("final_price").unwrap());
```

### Managing Stored Procedures

```rust
// List all procedures
let procedures = pm.list_procedures();

// Get a specific procedure
let proc = pm.get_procedure("calculate_discount")?;

// Drop a procedure
pm.drop_procedure("calculate_discount")?;
```

---

## Replication and High Availability

RustyDB supports database replication for high availability and disaster recovery.

### Replication Modes

#### Synchronous Replication
Waits for all replicas to acknowledge before committing.

```rust
use rusty_db::replication::{ReplicationManager, ReplicationMode};

let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);
```

#### Asynchronous Replication
Does not wait for replica acknowledgment (fire and forget).

```rust
let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);
```

#### Semi-Synchronous Replication
Waits for at least one replica to acknowledge.

```rust
let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);
```

### Adding Replicas

```rust
use rusty_db::replication::{ReplicaNode, ReplicaStatus};

let replica = ReplicaNode {
    id: "replica-1".to_string(),
    address: "192.168.1.100:5432".to_string(),
    status: ReplicaStatus::Active,
    lag_bytes: 0,
    last_sync: 0,
};

rm.add_replica(replica)?;
```

### Replicating Operations

```rust
use rusty_db::replication::ReplicationOperation;

let data = bincode::serialize(&operation_data)?;

rm.replicate_operation(
    ReplicationOperation::Insert,
    data
).await?;
```

### Monitoring Replicas

```rust
// Get all replicas
let replicas = rm.get_replicas();

for replica in replicas {
    println!("Replica {}: Status = {:?}, Lag = {} bytes", 
             replica.id, replica.status, replica.lag_bytes);
}

// Get lag for a specific replica
let lag = rm.get_replica_lag("replica-1")?;
```

### Failover

```rust
// Promote a replica to primary
rm.failover("replica-1").await?;
```

### Replica Status Management

```rust
// Update replica status
rm.update_replica_status("replica-1", ReplicaStatus::Lagging)?;

// Remove a replica
rm.remove_replica("replica-1")?;
```

---

## Best Practices

### Query Optimization
1. Always use the optimizer for complex queries
2. Create appropriate indexes for frequently queried columns
3. Use EXPLAIN to understand query plans (future feature)

### Joins
1. Prefer INNER JOINs when possible for better performance
2. Ensure join columns are indexed
3. Consider join order for large tables

### Aggregations
1. Use appropriate indexes on GROUP BY columns
2. Filter data before aggregation when possible
3. Use HAVING only when necessary

### Triggers
1. Keep trigger logic simple and fast
2. Avoid recursive triggers
3. Use BEFORE triggers for validation
4. Use AFTER triggers for auditing

### Stored Procedures
1. Use procedures for complex business logic
2. Keep procedures focused on a single task
3. Document parameter requirements
4. Handle errors within procedures

### Replication
1. Use synchronous mode for critical data
2. Monitor replica lag regularly
3. Test failover procedures
4. Plan for network partitions

---

## Performance Considerations

| Feature | Performance Impact | Recommendation |
|---------|-------------------|----------------|
| Query Optimization | +20-100% improvement | Always enable for complex queries |
| INNER JOIN | Moderate overhead | Best for small to medium tables |
| OUTER JOIN | Higher overhead | Use only when necessary |
| CROSS JOIN | Very high overhead | Avoid on large tables |
| Aggregations | Moderate overhead | Use indexes on GROUP BY columns |
| Triggers | Low-moderate overhead | Keep trigger logic minimal |
| Stored Procedures | Low overhead | Good for repeated operations |
| Sync Replication | High latency impact | Use for critical data only |
| Async Replication | Minimal impact | Good for non-critical data |

---

## Future Enhancements

The following features are planned for future releases:

1. **Common Table Expressions (CTEs)**: WITH clause support
2. **Window Functions**: ROW_NUMBER, RANK, PARTITION BY
3. **Advanced Subquery Optimization**: Subquery flattening and decorrelation
4. **Parallel Query Execution**: Multi-threaded query processing
5. **Adaptive Query Optimization**: Runtime plan adjustment
6. **Native Stored Procedures**: Rust-based procedures for performance
7. **Streaming Replication**: Continuous replication stream
8. **Multi-Master Replication**: Bidirectional replication

---

## See Also

- [README.md](README.md) - General project information
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture details
- [ENTERPRISE_FEATURES.md](ENTERPRISE_FEATURES.md) - Enterprise feature documentation
