# Complete Feature Guide for RustyDB

## Table of Contents
1. [Common Table Expressions (CTEs)](#common-table-expressions-ctes)
2. [Advanced Subqueries](#advanced-subqueries)
3. [Table Partitioning](#table-partitioning)
4. [Full-Text Search](#full-text-search)
5. [JSON Support](#json-support)
6. [Advanced Query Optimization](#advanced-query-optimization)
7. [Parallel Query Execution](#parallel-query-execution)
8. [Resource Management](#resource-management)

---

## Common Table Expressions (CTEs)

### Overview
CTEs (Common Table Expressions) provide a way to write auxiliary statements for use in a larger query. They are particularly useful for recursive queries and improving query readability.

### Non-Recursive CTEs

#### Basic Syntax
```sql
WITH cte_name AS (
    SELECT column1, column2
    FROM table_name
    WHERE condition
)
SELECT * FROM cte_name;
```

#### Example: Sales Analysis
```sql
WITH regional_sales AS (
    SELECT 
        region,
        SUM(amount) AS total_sales
    FROM orders
    GROUP BY region
),
top_regions AS (
    SELECT region
    FROM regional_sales
    WHERE total_sales > 1000000
)
SELECT 
    o.order_id,
    o.amount,
    o.region
FROM orders o
INNER JOIN top_regions tr ON o.region = tr.region;
```

### Recursive CTEs

#### Syntax
```sql
WITH RECURSIVE cte_name AS (
    -- Base case (anchor member)
    SELECT ...
    
    UNION ALL
    
    -- Recursive case (recursive member)
    SELECT ...
    FROM cte_name
    WHERE termination_condition
)
SELECT * FROM cte_name;
```

#### Example: Organization Hierarchy
```sql
WITH RECURSIVE employee_hierarchy AS (
    -- Base case: top-level employees (no manager)
    SELECT 
        id,
        name,
        manager_id,
        1 AS level,
        name AS path
    FROM employees
    WHERE manager_id IS NULL
    
    UNION ALL
    
    -- Recursive case: employees with managers
    SELECT 
        e.id,
        e.name,
        e.manager_id,
        eh.level + 1,
        eh.path || ' -> ' || e.name
    FROM employees e
    INNER JOIN employee_hierarchy eh ON e.manager_id = eh.id
    WHERE eh.level < 10  -- Prevent infinite recursion
)
SELECT 
    id,
    name,
    level,
    path
FROM employee_hierarchy
ORDER BY level, name;
```

### Multiple CTEs
```sql
WITH 
    sales_2023 AS (
        SELECT * FROM sales WHERE YEAR(date) = 2023
    ),
    top_products AS (
        SELECT 
            product_id,
            SUM(amount) AS total
        FROM sales_2023
        GROUP BY product_id
        ORDER BY total DESC
        LIMIT 10
    ),
    product_details AS (
        SELECT 
            p.id,
            p.name,
            p.category,
            tp.total
        FROM products p
        INNER JOIN top_products tp ON p.id = tp.product_id
    )
SELECT * FROM product_details
ORDER BY total DESC;
```

### CTE Optimization

#### Materialization
RustyDB automatically materializes CTEs that are:
- Recursive (always materialized)
- Referenced multiple times in the query
- Computationally expensive

```rust
// Rust API usage
use rusty_db::execution::{CteContext, CteDefinition, RecursiveCteEvaluator};

let mut cte_context = CteContext::new();

// Register a CTE
let cte = CteDefinition {
    name: "regional_sales".to_string(),
    columns: vec!["region".to_string(), "total".to_string()],
    query: Box::new(plan_node),
    recursive: false,
};
cte_context.register_cte(cte)?;

// Execute with CTE context
let result = executor.execute_with_ctes(&plan, &cte_context)?;
```

---

## Advanced Subqueries

### EXISTS and NOT EXISTS

#### Syntax
```sql
SELECT column_list
FROM table1
WHERE EXISTS (
    SELECT 1
    FROM table2
    WHERE table2.foreign_key = table1.primary_key
);
```

#### Example: Find Customers with Orders
```sql
SELECT 
    c.customer_id,
    c.name
FROM customers c
WHERE EXISTS (
    SELECT 1
    FROM orders o
    WHERE o.customer_id = c.customer_id
      AND o.order_date >= '2023-01-01'
);
```

#### Example: Find Products Never Ordered
```sql
SELECT 
    p.product_id,
    p.name
FROM products p
WHERE NOT EXISTS (
    SELECT 1
    FROM order_items oi
    WHERE oi.product_id = p.product_id
);
```

### IN and NOT IN

#### Example: Filter by Set of Values
```sql
SELECT 
    employee_id,
    name,
    department
FROM employees
WHERE department IN (
    SELECT department_name
    FROM departments
    WHERE budget > 100000
);
```

#### Example: Exclude Specific Values
```sql
SELECT 
    product_id,
    name
FROM products
WHERE category_id NOT IN (
    SELECT id
    FROM categories
    WHERE discontinued = true
);
```

### Scalar Subqueries

#### Example: Compare Against Aggregate
```sql
SELECT 
    employee_id,
    name,
    salary
FROM employees
WHERE salary > (
    SELECT AVG(salary)
    FROM employees
);
```

### Correlated Subqueries

#### Example: For Each Row
```sql
SELECT 
    o.order_id,
    o.customer_id,
    o.order_date,
    (SELECT COUNT(*)
     FROM order_items oi
     WHERE oi.order_id = o.order_id
    ) AS item_count
FROM orders o;
```

### ANY and ALL Operators

#### Example: ANY (at least one match)
```sql
SELECT 
    product_id,
    name,
    price
FROM products
WHERE price < ANY (
    SELECT price
    FROM competitor_products
    WHERE category = 'Electronics'
);
```

#### Example: ALL (all matches)
```sql
SELECT 
    product_id,
    name,
    price
FROM products
WHERE price >= ALL (
    SELECT price
    FROM products
    WHERE category = 'Budget'
);
```

### Subquery Optimization

RustyDB automatically optimizes subqueries through:

1. **Decorrelation**: Converts correlated subqueries to joins
```sql
-- Before optimization (correlated)
SELECT *
FROM orders o
WHERE o.amount > (
    SELECT AVG(amount)
    FROM orders o2
    WHERE o2.customer_id = o.customer_id
);

-- After optimization (decorrelated)
SELECT o.*
FROM orders o
INNER JOIN (
    SELECT customer_id, AVG(amount) as avg_amount
    FROM orders
    GROUP BY customer_id
) avg_orders ON o.customer_id = avg_orders.customer_id
WHERE o.amount > avg_orders.avg_amount;
```

2. **Semi-Join Conversion**: Converts IN/EXISTS to semi-joins

```rust
// Rust API usage
use rusty_db::execution::subquery::{SubqueryExpr, SubqueryType, SubqueryDecorrelator};

// Create EXISTS subquery
let subquery = SubqueryExpr::new(SubqueryType::Exists, plan)
    .with_outer_refs(vec!["customer_id".to_string()]);

// Try to decorrelate
if let Some(decorrelated) = SubqueryDecorrelator::decorrelate(&subquery) {
    // Use decorrelated plan
}
```

---

## Table Partitioning

### Overview
Table partitioning divides large tables into smaller, more manageable pieces while maintaining a single logical table. This improves query performance and manageability.

### Range Partitioning

#### By Date
```sql
CREATE TABLE orders (
    order_id INT,
    order_date DATE,
    amount DECIMAL(10,2)
)
PARTITION BY RANGE (order_date) (
    PARTITION p_2023_q1 VALUES LESS THAN ('2023-04-01'),
    PARTITION p_2023_q2 VALUES LESS THAN ('2023-07-01'),
    PARTITION p_2023_q3 VALUES LESS THAN ('2023-10-01'),
    PARTITION p_2023_q4 VALUES LESS THAN ('2024-01-01')
);
```

#### By Numeric Range
```sql
CREATE TABLE customers (
    customer_id INT,
    name VARCHAR(100),
    revenue DECIMAL(12,2)
)
PARTITION BY RANGE (revenue) (
    PARTITION p_small VALUES LESS THAN (10000),
    PARTITION p_medium VALUES LESS THAN (100000),
    PARTITION p_large VALUES LESS THAN (1000000),
    PARTITION p_enterprise VALUES LESS THAN (MAXVALUE)
);
```

### Hash Partitioning

Evenly distributes rows across partitions:

```sql
CREATE TABLE users (
    user_id INT,
    username VARCHAR(50),
    email VARCHAR(100)
)
PARTITION BY HASH (user_id)
PARTITIONS 8;
```

### List Partitioning

Based on discrete values:

```sql
CREATE TABLE stores (
    store_id INT,
    store_name VARCHAR(100),
    region VARCHAR(20)
)
PARTITION BY LIST (region) (
    PARTITION p_west VALUES IN ('CA', 'OR', 'WA', 'NV'),
    PARTITION p_east VALUES IN ('NY', 'MA', 'FL', 'VA'),
    PARTITION p_central VALUES IN ('TX', 'IL', 'MO', 'CO'),
    PARTITION p_other VALUES IN (DEFAULT)
);
```

### Partition Management

#### Add Partition
```sql
ALTER TABLE orders
ADD PARTITION p_2024_q1 VALUES LESS THAN ('2024-04-01');
```

#### Drop Partition
```sql
ALTER TABLE orders
DROP PARTITION p_2023_q1;
```

#### List Partitions
```sql
SHOW PARTITIONS FROM orders;
```

### Partition Pruning

Automatically eliminates partitions that don't match query criteria:

```sql
-- Only scans p_2023_q3 partition
SELECT *
FROM orders
WHERE order_date BETWEEN '2023-07-01' AND '2023-09-30';
```

```rust
// Rust API usage
use rusty_db::storage::partitioning::{
    PartitionManager, PartitionStrategy, PartitionPruner,
    RangePartition, QueryPredicate, PredicateOperator
};

let mut manager = PartitionManager::new();

// Create range partitioned table
let strategy = PartitionStrategy::Range {
    column: "order_date".to_string(),
    ranges: vec![
        RangePartition {
            name: "p_2023_q1".to_string(),
            lower_bound: Some("2023-01-01".to_string()),
            upper_bound: Some("2023-04-01".to_string()),
        },
        // ... more partitions
    ],
};

manager.create_partitioned_table("orders".to_string(), strategy)?;

// Partition pruning
let predicate = QueryPredicate {
    column: "order_date".to_string(),
    operator: PredicateOperator::Equal,
    value: "2023-02-15".to_string(),
};

let metadata = manager.get_metadata("orders")?;
let active_partitions = PartitionPruner::prune_partitions(&metadata, &predicate);
// Returns only ["p_2023_q1"]
```

---

## Full-Text Search

### Overview
Full-text search enables fast text searching with relevance ranking, supporting natural language queries.

### Creating Full-Text Index

```sql
CREATE FULLTEXT INDEX idx_content
ON articles (title, content);
```

### Basic Search

```sql
SELECT 
    article_id,
    title,
    SCORE() as relevance
FROM articles
WHERE MATCH(title, content) AGAINST ('database performance optimization')
ORDER BY relevance DESC
LIMIT 10;
```

### Boolean Search

```sql
-- AND operator (implicit)
MATCH(content) AGAINST ('database optimization')

-- OR operator
MATCH(content) AGAINST ('database OR postgresql')

-- NOT operator
MATCH(content) AGAINST ('database -mysql')

-- Phrase search
MATCH(content) AGAINST ('"query optimization"')
```

### Wildcard Search

```sql
-- Prefix wildcard
MATCH(content) AGAINST ('data*')

-- Returns: data, database, dataset, etc.
```

### Relevance Scoring

RustyDB uses TF-IDF (Term Frequency-Inverse Document Frequency) for scoring:

- **TF**: How often a term appears in a document
- **IDF**: How rare the term is across all documents
- **Score** = TF × IDF

### Full-Text Features

1. **Stop Word Filtering**: Common words (the, and, or) are automatically excluded
2. **Stemming**: Reduces words to root form (running → run)
3. **Case Insensitive**: Searches are case-insensitive by default

```rust
// Rust API usage
use rusty_db::index::fulltext::{FullTextIndex, QueryParser, Highlighter};

let mut index = FullTextIndex::new("articles".to_string(), "content".to_string());

// Index documents
index.index_document(1, "Introduction to database systems".to_string())?;
index.index_document(2, "Advanced query optimization".to_string())?;

// Simple search
let results = index.search("database optimization")?;
for result in results {
    println!("Doc {}: score = {}", result.doc_id, result.score);
    println!("Snippet: {}", result.snippet);
}

// Phrase search
let phrase_results = index.search_phrase("query optimization")?;

// Wildcard search
let wildcard_results = index.search_wildcard("data*")?;

// Boolean search
let query = QueryParser::parse(r#"database "full text" -spam"#);
let bool_results = BooleanSearchEvaluator::evaluate(&index, &query)?;

// Highlight matches
let text = "This is a document about databases";
let highlighted = Highlighter::highlight(
    text,
    &vec!["database".to_string()],
    "<b>",
    "</b>"
);
// Returns: "This is a document about <b>database</b>s"
```

---

## JSON Support

### JSON Data Type

```sql
CREATE TABLE products (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    attributes JSON
);

INSERT INTO products VALUES
(1, 'Laptop', '{"brand": "Dell", "specs": {"ram": "16GB", "storage": "512GB"}}');
```

### JSON Path Expressions

```sql
-- Extract value
SELECT JSON_EXTRACT(attributes, '$.brand') AS brand
FROM products;
-- Returns: "Dell"

-- Extract nested value
SELECT JSON_EXTRACT(attributes, '$.specs.ram') AS ram
FROM products;
-- Returns: "16GB"

-- Array access
SELECT JSON_EXTRACT(data, '$.items[0].name')
FROM orders;
```

### JSON Operators

#### JSON_SET
```sql
UPDATE products
SET attributes = JSON_SET(attributes, '$.price', 999.99)
WHERE id = 1;
```

#### JSON_DELETE
```sql
UPDATE products
SET attributes = JSON_DELETE(attributes, '$.old_field')
WHERE id = 1;
```

#### JSON_CONTAINS
```sql
SELECT *
FROM products
WHERE JSON_CONTAINS(attributes, '"Dell"', '$.brand');
```

#### JSON_ARRAY_LENGTH
```sql
SELECT 
    id,
    JSON_ARRAY_LENGTH(JSON_EXTRACT(data, '$.items')) AS item_count
FROM orders;
```

#### JSON_KEYS
```sql
SELECT JSON_KEYS(attributes)
FROM products
WHERE id = 1;
-- Returns: ["brand", "specs", "price"]
```

### JSON Aggregation

```sql
-- Aggregate into JSON array
SELECT JSON_AGG(name) AS all_names
FROM products;
-- Returns: ["Laptop", "Mouse", "Keyboard"]

-- Aggregate into JSON object
SELECT JSON_OBJECT_AGG(id, name) AS product_map
FROM products;
-- Returns: {"1": "Laptop", "2": "Mouse"}
```

### JSON Indexing

```sql
CREATE INDEX idx_brand
ON products (JSON_EXTRACT(attributes, '$.brand'));

-- Fast query using index
SELECT *
FROM products
WHERE JSON_EXTRACT(attributes, '$.brand') = 'Dell';
```

```rust
// Rust API usage
use rusty_db::storage::json::{JsonData, JsonPath, JsonOperators};

// Create JSON
let json = JsonData::from_str(r#"{"name": "John", "age": 30, "address": {"city": "NYC"}}"#)?;

// Extract value
let name = JsonPath::extract(&json, "$.name")?;
println!("{}", name.to_string()); // "John"

// Extract nested
let city = JsonPath::extract(&json, "$.address.city")?;
println!("{}", city.to_string()); // "NYC"

// Set value
let updated = JsonOperators::json_set(
    &json,
    "$.age",
    JsonData::from_str("31")?
)?;

// Delete field
let without_age = JsonOperators::json_delete(&json, "$.age")?;

// Check contains
let has_name = JsonOperators::json_contains(&json, &JsonData::from_str(r#""John""#)?);

// Get array length
let json_array = JsonData::from_str("[1, 2, 3, 4, 5]")?;
let length = JsonOperators::json_array_length(&json_array)?;
println!("Length: {}", length); // 5

// Get object keys
let keys = JsonOperators::json_keys(&json)?;
println!("Keys: {:?}", keys); // ["name", "age", "address"]
```

This comprehensive guide continues with more sections...
