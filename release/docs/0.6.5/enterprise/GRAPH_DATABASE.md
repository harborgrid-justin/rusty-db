# RustyDB v0.6.5 Graph Database Engine

**Version**: 0.6.5
**Last Updated**: December 2025
**Target Audience**: Graph Database Developers, Data Scientists
**Status**: ✅ **Production Ready**

---

## Overview

RustyDB Graph Database Engine provides native property graph storage and querying capabilities, supporting complex relationship analysis, path finding, and graph algorithms at scale.

### Key Features ✅

- **Property Graph Model**: Nodes and relationships with properties
- **PGQL-like Query Language**: Familiar graph query syntax
- **Native Storage**: Index-free adjacency for fast traversal
- **Graph Algorithms**: Shortest path, centrality, community detection
- **Transactional**: ACID guarantees for graph operations
- **Scalable**: Billions of nodes and relationships

---

## Property Graph Model

### Data Model

```
┌────────────────────────────────────────────────────────┐
│                   Property Graph                        │
├────────────────────────────────────────────────────────┤
│                                                         │
│  Nodes (Vertices):                                     │
│    ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│    │ Person  │  │ Product │  │ Company │            │
│    │ id: 1   │  │ id: 2   │  │ id: 3   │            │
│    │ name: A │  │ name: X │  │ name: Y │            │
│    └────┬────┘  └────┬────┘  └────┬────┘            │
│         │            │            │                   │
│  Relationships (Edges):                               │
│         ├─[BOUGHT]──▶│                                │
│         │ date: 2024-01-15                            │
│         │ amount: $99.99                              │
│         │                                              │
│         └─[WORKS_FOR]─▶│                              │
│           since: 2020-06-01                           │
│           role: "Engineer"                            │
└────────────────────────────────────────────────────────┘
```

### Node Definition

```rust
pub struct Node {
    pub id: NodeId,
    pub labels: Vec<String>,  // e.g., ["Person", "Employee"]
    pub properties: HashMap<String, Value>,
}
```

**Example**:
```json
{
  "id": 1,
  "labels": ["Person", "Employee"],
  "properties": {
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com"
  }
}
```

### Relationship Definition

```rust
pub struct Relationship {
    pub id: RelationshipId,
    pub relationship_type: String,  // e.g., "KNOWS", "LIKES"
    pub source_node: NodeId,
    pub target_node: NodeId,
    pub properties: HashMap<String, Value>,
}
```

**Example**:
```json
{
  "id": 100,
  "type": "KNOWS",
  "source": 1,
  "target": 2,
  "properties": {
    "since": "2015-03-20",
    "strength": 0.8
  }
}
```

---

## Graph Query Language

### PGQL-like Syntax

**Create Nodes**:
```sql
-- Create person node
CREATE (:Person {
    name: 'Alice',
    age: 30,
    city: 'San Francisco'
});

-- Create product node
CREATE (:Product {
    name: 'Laptop',
    price: 999.99,
    category: 'Electronics'
});
```

**Create Relationships**:
```sql
-- Create KNOWS relationship
MATCH (a:Person {name: 'Alice'}), (b:Person {name: 'Bob'})
CREATE (a)-[:KNOWS {since: '2015-03-20'}]->(b);

-- Create BOUGHT relationship
MATCH (p:Person {name: 'Alice'}), (prod:Product {name: 'Laptop'})
CREATE (p)-[:BOUGHT {date: '2024-01-15', amount: 999.99}]->(prod);
```

**Query Patterns**:
```sql
-- Find all friends of Alice
MATCH (a:Person {name: 'Alice'})-[:KNOWS]->(friend)
RETURN friend.name, friend.age;

-- Find products bought by Alice's friends
MATCH (a:Person {name: 'Alice'})-[:KNOWS]->(friend)-[:BOUGHT]->(product)
RETURN DISTINCT product.name, product.price;

-- Find 2-hop friends (friends of friends)
MATCH (a:Person {name: 'Alice'})-[:KNOWS*2]->(fof)
RETURN fof.name;

-- Variable-length path (1 to 3 hops)
MATCH (a:Person {name: 'Alice'})-[:KNOWS*1..3]->(connection)
RETURN connection.name, LENGTH(path) AS hops;
```

---

## Graph Algorithms

### 1. Shortest Path ✅

**Dijkstra's Algorithm**:
```sql
-- Find shortest path between Alice and Charlie
MATCH path = shortestPath(
    (a:Person {name: 'Alice'})-[:KNOWS*]-(c:Person {name: 'Charlie'})
)
RETURN path, LENGTH(path) AS distance;
```

**Implementation**:
```rust
pub fn dijkstra(
    graph: &Graph,
    source: NodeId,
    target: NodeId,
) -> Result<Vec<NodeId>> {
    // Priority queue-based Dijkstra
    let mut distances: HashMap<NodeId, f64> = HashMap::new();
    let mut previous: HashMap<NodeId, NodeId> = HashMap::new();
    let mut pq = BinaryHeap::new();

    distances.insert(source, 0.0);
    pq.push(State { node: source, cost: 0.0 });

    while let Some(State { node, cost }) = pq.pop() {
        if node == target {
            return Ok(reconstruct_path(&previous, source, target));
        }

        for neighbor in graph.neighbors(node)? {
            let edge_weight = graph.edge_weight(node, neighbor)?;
            let new_cost = cost + edge_weight;

            if new_cost < *distances.get(&neighbor).unwrap_or(&f64::INFINITY) {
                distances.insert(neighbor, new_cost);
                previous.insert(neighbor, node);
                pq.push(State { node: neighbor, cost: new_cost });
            }
        }
    }

    Err(GraphError::PathNotFound)
}
```

**A* Algorithm** (with heuristic):
```rust
pub fn astar(
    graph: &Graph,
    source: NodeId,
    target: NodeId,
    heuristic: impl Fn(NodeId, NodeId) -> f64,
) -> Result<Vec<NodeId>>
```

---

### 2. Centrality Measures ✅

**Betweenness Centrality**:
```sql
-- Find most influential nodes (bridges)
CALL graph.betweenness_centrality()
YIELD nodeId, score
RETURN nodeId, score
ORDER BY score DESC
LIMIT 10;
```

**Closeness Centrality**:
```sql
-- Find nodes closest to all others
CALL graph.closeness_centrality()
YIELD nodeId, score
RETURN nodeId, score
ORDER BY score DESC
LIMIT 10;
```

**PageRank**:
```sql
-- Rank nodes by importance
CALL graph.pagerank(iterations: 20, damping: 0.85)
YIELD nodeId, rank
RETURN nodeId, rank
ORDER BY rank DESC
LIMIT 10;
```

**Implementation**:
```rust
pub fn pagerank(
    graph: &Graph,
    iterations: usize,
    damping_factor: f64,
) -> HashMap<NodeId, f64> {
    let n = graph.node_count();
    let mut ranks: HashMap<NodeId, f64> = HashMap::new();

    // Initialize all ranks to 1/N
    for node in graph.nodes() {
        ranks.insert(node, 1.0 / n as f64);
    }

    // Iterate
    for _ in 0..iterations {
        let mut new_ranks: HashMap<NodeId, f64> = HashMap::new();

        for node in graph.nodes() {
            let mut rank = (1.0 - damping_factor) / n as f64;

            for incoming in graph.incoming_neighbors(node) {
                let out_degree = graph.out_degree(incoming) as f64;
                rank += damping_factor * ranks[&incoming] / out_degree;
            }

            new_ranks.insert(node, rank);
        }

        ranks = new_ranks;
    }

    ranks
}
```

---

### 3. Community Detection ✅

**Louvain Method**:
```sql
-- Detect communities
CALL graph.louvain_communities()
YIELD nodeId, communityId
RETURN communityId, COUNT(*) AS size
ORDER BY size DESC;
```

**Label Propagation**:
```sql
-- Fast community detection
CALL graph.label_propagation(iterations: 10)
YIELD nodeId, label
RETURN label, COLLECT(nodeId) AS members;
```

---

### 4. Graph Traversal ✅

**Breadth-First Search (BFS)**:
```rust
pub fn bfs(
    graph: &Graph,
    start: NodeId,
    max_depth: Option<usize>,
) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut result = Vec::new();

    queue.push_back((start, 0));
    visited.insert(start);

    while let Some((node, depth)) = queue.pop_front() {
        if let Some(max) = max_depth {
            if depth >= max {
                continue;
            }
        }

        result.push(node);

        for neighbor in graph.neighbors(node) {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    result
}
```

**Depth-First Search (DFS)**:
```rust
pub fn dfs(
    graph: &Graph,
    start: NodeId,
    visitor: &mut impl FnMut(NodeId),
) {
    let mut visited = HashSet::new();
    dfs_recursive(graph, start, &mut visited, visitor);
}

fn dfs_recursive(
    graph: &Graph,
    node: NodeId,
    visited: &mut HashSet<NodeId>,
    visitor: &mut impl FnMut(NodeId),
) {
    visited.insert(node);
    visitor(node);

    for neighbor in graph.neighbors(node) {
        if !visited.contains(&neighbor) {
            dfs_recursive(graph, neighbor, visited, visitor);
        }
    }
}
```

---

## Storage Architecture

### Index-Free Adjacency

**Direct Relationship Pointers**:
```
Node 1: {
    id: 1,
    properties: {...},
    outgoing: [rel_100, rel_101, rel_102],  // Direct pointers
    incoming: [rel_50, rel_51]
}

Relationship 100: {
    id: 100,
    type: "KNOWS",
    source: 1,
    target: 2,
    properties: {...}
}
```

**Performance**:
- Traversal: O(1) to find neighbors
- No index lookups required
- Cache-friendly (locality)

### Native Graph Storage

```
┌─────────────────────────────────────────┐
│        Graph Storage Layout              │
├─────────────────────────────────────────┤
│                                          │
│  Node Store:                             │
│    ┌──────────────────────────────┐     │
│    │ Node ID → Node Data          │     │
│    │ Hash Map (fast lookup)       │     │
│    └──────────────────────────────┘     │
│                                          │
│  Relationship Store:                     │
│    ┌──────────────────────────────┐     │
│    │ Rel ID → Relationship Data   │     │
│    │ Hash Map (fast lookup)       │     │
│    └──────────────────────────────┘     │
│                                          │
│  Adjacency Lists:                        │
│    ┌──────────────────────────────┐     │
│    │ Node ID → [Outgoing Rels]    │     │
│    │ Node ID → [Incoming Rels]    │     │
│    └──────────────────────────────┘     │
│                                          │
│  Indexes:                                │
│    ┌──────────────────────────────┐     │
│    │ Label → [Node IDs]           │     │
│    │ Property → [Node IDs]        │     │
│    │ Rel Type → [Rel IDs]         │     │
│    └──────────────────────────────┘     │
└─────────────────────────────────────────┘
```

---

## Use Cases

### 1. Social Networks

**Friend Recommendations**:
```sql
-- Find friends of friends who aren't already friends
MATCH (me:Person {name: 'Alice'})-[:KNOWS]->(friend)-[:KNOWS]->(fof)
WHERE NOT (me)-[:KNOWS]->(fof) AND me <> fof
RETURN fof.name, COUNT(*) AS mutual_friends
ORDER BY mutual_friends DESC
LIMIT 10;
```

### 2. Fraud Detection

**Suspicious Patterns**:
```sql
-- Find accounts with shared contact info (potential fraud rings)
MATCH (a1:Account)-[:HAS_EMAIL]->(email)<-[:HAS_EMAIL]-(a2:Account)
WHERE a1 <> a2
RETURN a1.account_id, a2.account_id, email.address;
```

### 3. Recommendation Engines

**Collaborative Filtering**:
```sql
-- Recommend products based on similar users
MATCH (me:User {id: 123})-[:RATED]->(p:Product)<-[:RATED]-(similar:User)
MATCH (similar)-[:RATED]->(recommendation:Product)
WHERE NOT (me)-[:RATED]->(recommendation)
RETURN recommendation.name, COUNT(*) AS score
ORDER BY score DESC
LIMIT 10;
```

### 4. Knowledge Graphs

**Entity Relationships**:
```sql
-- Find all connections between two entities
MATCH path = (entity1:Entity {name: 'Apple'})-[*..5]-(entity2:Entity {name: 'iPhone'})
RETURN path
LIMIT 100;
```

---

## Performance Optimization

### Indexing

**Create Indexes**:
```sql
-- Index on node label
CREATE INDEX ON :Person(name);

-- Composite index
CREATE INDEX ON :Person(name, age);

-- Relationship type index
CREATE INDEX ON [:KNOWS];
```

### Query Optimization

**Best Practices**:
1. **Anchor patterns**: Start with specific nodes
   ```sql
   -- Good: Anchored on specific person
   MATCH (a:Person {name: 'Alice'})-[:KNOWS]->(friend)

   -- Bad: Unanchored (scans all relationships)
   MATCH ()-[:KNOWS]->(friend)
   ```

2. **Limit relationship traversal**:
   ```sql
   -- Good: Limited depth
   MATCH (a)-[:KNOWS*1..3]->(b)

   -- Bad: Unbounded (potential infinite loop)
   MATCH (a)-[:KNOWS*]->(b)
   ```

3. **Use DISTINCT**:
   ```sql
   -- Avoid duplicate results
   MATCH (a)-[:KNOWS]->(friend)-[:LIKES]->(thing)
   RETURN DISTINCT thing.name;
   ```

---

## Scalability

### Horizontal Scaling

**Graph Partitioning**:
- Hash-based partitioning by node ID
- Edge-cut minimization
- Balanced partitions

**Distributed Query Processing**:
- Query fragmentation
- Remote traversal
- Result aggregation

### Vertical Scaling

**In-Memory Graph**:
- Cache hot nodes and relationships
- Memory-mapped files
- Compressed storage for cold data

---

## Conclusion

RustyDB v0.6.5 Graph Database Engine provides **production-ready graph analytics** with:
- ✅ Native property graph storage
- ✅ PGQL-like query language
- ✅ Comprehensive graph algorithms
- ✅ Index-free adjacency
- ✅ Transactional ACID guarantees

**Status**: Production-ready for graph workloads

---

**Document Version**: 0.6.5
**Last Updated**: December 2025
**Validation**: ✅ Production Ready

---
