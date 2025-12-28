# RustyDB v0.6 Graph Database (PGQL-Compatible)

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: Graph Database Developers, Data Scientists, Network Analysts

---

## Overview

RustyDB Graph Database provides a property graph model with PGQL-compatible query language and advanced graph algorithms for analyzing connected data. Store vertices (nodes), edges (relationships), and properties while leveraging powerful graph traversal and analytics capabilities.

### Key Features

- **Property Graph Model**: Vertices, edges, and properties
- **PGQL Query Language**: SQL-like graph queries
- **Graph Algorithms**: PageRank, shortest path, community detection, centrality
- **Multiple Storage Formats**: Adjacency list, CSR (Compressed Sparse Row), edge-centric
- **Temporal Graphs**: Time-aware graph analysis
- **Graph Embeddings**: Vector representations for ML
- **Recommendation Engine**: Built-in collaborative filtering

---

## Property Graph Model

### Concepts

**Vertices** (Nodes):
- Unique ID
- Labels (types)
- Properties (key-value pairs)

**Edges** (Relationships):
- Unique ID
- Source and destination vertices
- Direction
- Label (type)
- Properties

**Graph Structure**:
```
   Vertex         Edge          Vertex
(Person:Alice) ─[KNOWS]─────→ (Person:Bob)
  {age: 30}    {since: 2020}   {age: 28}
```

### Create Graph

```sql
-- Create property graph
CREATE PROPERTY GRAPH social_network;

-- Add vertices
INSERT INTO social_network.vertices (id, labels, properties)
VALUES
  ('alice', ARRAY['Person'], '{"name": "Alice", "age": 30}'::jsonb),
  ('bob', ARRAY['Person'], '{"name": "Bob", "age": 28}'::jsonb),
  ('acme', ARRAY['Company'], '{"name": "Acme Corp", "industry": "Tech"}'::jsonb);

-- Add edges
INSERT INTO social_network.edges (id, source_id, dest_id, label, properties)
VALUES
  ('e1', 'alice', 'bob', 'KNOWS', '{"since": 2020}'::jsonb),
  ('e2', 'alice', 'acme', 'WORKS_FOR', '{"role": "Engineer", "start_date": "2022-01-01"}'::jsonb),
  ('e3', 'bob', 'acme', 'WORKS_FOR', '{"role": "Manager", "start_date": "2021-06-15"}'::jsonb);
```

---

## PGQL Queries

### Basic Pattern Matching

```sql
-- Find all persons
SELECT v.id, v.properties->>'name' as name
FROM social_network.vertices v
WHERE 'Person' = ANY(v.labels);

-- Find relationships
SELECT e.source_id, e.label, e.dest_id
FROM social_network.edges e
WHERE e.label = 'KNOWS';

-- Pattern: Person -> KNOWS -> Person
SELECT v1.properties->>'name' as person1,
       v2.properties->>'name' as person2,
       e.properties->>'since' as friends_since
FROM social_network.vertices v1
JOIN social_network.edges e ON v1.id = e.source_id
JOIN social_network.vertices v2 ON e.dest_id = v2.id
WHERE e.label = 'KNOWS';
```

### Advanced Patterns

```sql
-- Friends of friends (2-hop)
SELECT DISTINCT v3.properties->>'name' as friend_of_friend
FROM social_network.vertices v1
JOIN social_network.edges e1 ON v1.id = e1.source_id
JOIN social_network.vertices v2 ON e1.dest_id = v2.id
JOIN social_network.edges e2 ON v2.id = e2.source_id
JOIN social_network.vertices v3 ON e2.dest_id = v3.id
WHERE v1.properties->>'name' = 'Alice'
  AND e1.label = 'KNOWS'
  AND e2.label = 'KNOWS'
  AND v3.id != v1.id;

-- Coworkers (work at same company)
SELECT v1.properties->>'name' as person1,
       v2.properties->>'name' as person2,
       company.properties->>'name' as company
FROM social_network.vertices v1
JOIN social_network.edges e1 ON v1.id = e1.source_id
JOIN social_network.vertices company ON e1.dest_id = company.id
JOIN social_network.edges e2 ON company.id = e2.dest_id
JOIN social_network.vertices v2 ON e2.source_id = v2.id
WHERE e1.label = 'WORKS_FOR'
  AND e2.label = 'WORKS_FOR'
  AND v1.id < v2.id;  -- Avoid duplicates
```

### Variable-Length Paths

```sql
-- Find paths of any length between Alice and Bob
SELECT path_vertices, path_edges
FROM graph_shortest_path(
  graph_name => 'social_network',
  start_vertex => 'alice',
  end_vertex => 'bob',
  max_depth => 10
);

-- All vertices reachable within 3 hops
SELECT v.id, v.properties->>'name', distance
FROM graph_traverse(
  graph_name => 'social_network',
  start_vertex => 'alice',
  max_depth => 3,
  mode => 'BFS'  -- Breadth-First Search
) as v;
```

---

## Graph Algorithms

### PageRank

**Use Case**: Identify influential nodes

```sql
-- Calculate PageRank
SELECT vertex_id,
       pagerank_score,
       rank() OVER (ORDER BY pagerank_score DESC) as rank
FROM calculate_pagerank(
  graph_name => 'social_network',
  damping_factor => 0.85,
  max_iterations => 100,
  tolerance => 0.0001
);

-- Output:
-- vertex_id | pagerank_score | rank
-- alice     | 0.342         | 1
-- bob       | 0.298         | 2
-- charlie   | 0.256         | 3
```

### Shortest Path (Dijkstra)

```sql
-- Find shortest path between two vertices
SELECT *
FROM shortest_path(
  graph_name => 'social_network',
  start_vertex => 'alice',
  end_vertex => 'dave',
  weight_property => 'distance'  -- Optional: edge weight
);

-- Output:
-- path_length: 3
-- path_vertices: ['alice', 'bob', 'charlie', 'dave']
-- path_edges: ['e1', 'e4', 'e7']
-- total_weight: 15.5
```

### Community Detection

**Use Case**: Find clusters/groups

```sql
-- Louvain algorithm (modularity optimization)
SELECT vertex_id,
       community_id,
       count(*) OVER (PARTITION BY community_id) as community_size
FROM detect_communities(
  graph_name => 'social_network',
  algorithm => 'louvain',
  min_community_size => 3
);

-- Label propagation (faster, less accurate)
SELECT vertex_id,
       community_id
FROM detect_communities(
  graph_name => 'social_network',
  algorithm => 'label_propagation',
  max_iterations => 100
);
```

### Centrality Measures

**Betweenness Centrality** (bridge nodes):
```sql
SELECT vertex_id,
       betweenness_score
FROM calculate_betweenness_centrality('social_network')
ORDER BY betweenness_score DESC
LIMIT 10;
```

**Closeness Centrality** (central nodes):
```sql
SELECT vertex_id,
       closeness_score
FROM calculate_closeness_centrality('social_network')
ORDER BY closeness_score DESC
LIMIT 10;
```

**Degree Centrality** (most connected):
```sql
SELECT v.id as vertex_id,
       count(*) as degree,
       count(*) FILTER (WHERE e.source_id = v.id) as out_degree,
       count(*) FILTER (WHERE e.dest_id = v.id) as in_degree
FROM social_network.vertices v
LEFT JOIN social_network.edges e ON (v.id = e.source_id OR v.id = e.dest_id)
GROUP BY v.id
ORDER BY degree DESC;
```

### Triangle Counting

**Use Case**: Measure clustering coefficient

```sql
SELECT vertex_id,
       triangle_count,
       clustering_coefficient
FROM count_triangles('social_network');
```

### Connected Components

```sql
-- Find connected components
SELECT component_id,
       array_agg(vertex_id) as vertices,
       count(*) as component_size
FROM find_connected_components('social_network')
GROUP BY component_id
ORDER BY component_size DESC;
```

---

## Graph Analytics

### Degree Distribution

```sql
SELECT degree,
       count(*) as vertex_count
FROM (
  SELECT count(*) as degree
  FROM social_network.edges e
  GROUP BY COALESCE(e.source_id, e.dest_id)
) deg
GROUP BY degree
ORDER BY degree;
```

### Graph Density

```sql
SELECT vertex_count,
       edge_count,
       density
FROM calculate_graph_density('social_network');

-- density = (2 * edge_count) / (vertex_count * (vertex_count - 1))
```

### Temporal Analysis

```sql
-- Graph snapshot at specific time
SELECT *
FROM graph_snapshot(
  graph_name => 'social_network',
  timestamp => '2024-01-01'::timestamp
);

-- Growth over time
SELECT date_trunc('month', created_at) as month,
       count(*) as new_vertices
FROM social_network.vertices
GROUP BY month
ORDER BY month;
```

---

## Graph Embeddings

### Node2Vec

**Use Case**: Convert graph structure to vectors for ML

```sql
-- Generate embeddings
SELECT vertex_id,
       embedding_vector
FROM generate_node_embeddings(
  graph_name => 'social_network',
  algorithm => 'node2vec',
  dimensions => 128,
  walk_length => 80,
  num_walks => 10,
  p => 1.0,  -- Return parameter
  q => 1.0   -- In-out parameter
);

-- Find similar nodes
SELECT v2.id,
       cosine_similarity(e1.embedding_vector, e2.embedding_vector) as similarity
FROM node_embeddings e1
JOIN node_embeddings e2 ON e1.graph_name = e2.graph_name
JOIN social_network.vertices v2 ON e2.vertex_id = v2.id
WHERE e1.vertex_id = 'alice'
  AND e2.vertex_id != 'alice'
ORDER BY similarity DESC
LIMIT 10;
```

### Graph Kernels

```sql
-- Weisfeiler-Lehman kernel (graph similarity)
SELECT graph1_name,
       graph2_name,
       kernel_similarity
FROM calculate_wl_kernel(
  graphs => ARRAY['graph1', 'graph2', 'graph3'],
  iterations => 5
);
```

---

## Recommendation Engine

### Collaborative Filtering

**Item-Based**:
```sql
-- Recommend products based on user purchases
SELECT recommended_product_id,
       recommendation_score
FROM recommend_items(
  user_id => 'alice',
  graph_name => 'purchase_graph',
  method => 'item_based',
  top_k => 10
);
```

**User-Based**:
```sql
-- Find similar users
SELECT similar_user_id,
       similarity_score
FROM find_similar_users(
  user_id => 'alice',
  graph_name => 'social_network',
  method => 'collaborative_filtering',
  top_k => 20
);
```

### Personalized PageRank

```sql
-- Personalized recommendations
SELECT vertex_id,
       personalized_score
FROM calculate_personalized_pagerank(
  graph_name => 'social_network',
  source_vertex => 'alice',
  damping_factor => 0.85
)
ORDER BY personalized_score DESC
LIMIT 10;
```

---

## Storage Formats

### Adjacency List

**Best for**: General-purpose, frequent updates

```
Vertex: alice
  Outgoing: [bob, charlie, dave]
  Incoming: [eve]

Vertex: bob
  Outgoing: [charlie]
  Incoming: [alice, dave]
```

### CSR (Compressed Sparse Row)

**Best for**: Read-heavy, algorithmic processing, memory efficiency

```
Vertices: [alice, bob, charlie, dave, eve]
Offsets:  [0, 3, 4, 6, 8, 9]
Edges:    [bob, charlie, dave, charlie, alice, dave, alice, bob, alice]
```

**Convert to CSR**:
```sql
SELECT convert_graph_format(
  graph_name => 'social_network',
  format => 'CSR'
);
```

### Edge-Centric

**Best for**: Edge-heavy queries, streaming updates

---

## Performance Optimization

### Caching Degree

```sql
-- Pre-compute degrees (cached)
ALTER GRAPH social_network ENABLE DEGREE_CACHE;

-- Query uses cached values
SELECT vertex_id, degree FROM get_vertex_degree('alice');
```

### Graph Partitioning

```sql
-- Partition large graphs
ALTER GRAPH large_network
PARTITION BY vertex_property('region')
INTO 4 PARTITIONS;
```

### Indexes

```sql
-- Index on vertex labels
CREATE INDEX ON social_network.vertices USING GIN (labels);

-- Index on edge labels
CREATE INDEX ON social_network.edges (label);

-- Index on properties
CREATE INDEX ON social_network.vertices USING GIN (properties);
```

---

## Use Cases

### Social Network Analysis

```sql
-- Influencer identification (PageRank)
-- Friend recommendations (collaborative filtering)
-- Community detection (Louvain)
-- Viral content prediction (temporal analysis)
```

### Fraud Detection

```sql
-- Detect fraud rings
SELECT *
FROM detect_communities(
  graph_name => 'transaction_network',
  algorithm => 'louvain'
)
WHERE community_size > 10
  AND avg_transaction_amount > 10000;

-- Suspicious patterns (rapid money flow)
SELECT *
FROM graph_pattern_match(
  graph_name => 'transaction_network',
  pattern => '(a:Account)-[:TRANSFER]->(b:Account)-[:TRANSFER]->(c:Account)',
  where => 'path_length < 3 AND total_amount > 100000'
);
```

### Knowledge Graphs

```sql
-- Semantic search
SELECT entity_id,
       entity_type,
       relationship_path
FROM graph_traverse(
  graph_name => 'knowledge_graph',
  start_entity => 'RustyDB',
  relationship_types => ARRAY['SIMILAR_TO', 'PART_OF', 'RELATED_TO'],
  max_depth => 3
);
```

### Network Topology

```sql
-- Identify critical infrastructure nodes
SELECT node_id,
       betweenness_centrality,
       removal_impact
FROM calculate_network_robustness('infrastructure_graph');
```

---

## Best Practices

1. **Choose Right Storage**: CSR for algorithms, adjacency list for updates
2. **Index Strategically**: Index labels and frequently queried properties
3. **Cache Degrees**: Enable degree cache for large graphs
4. **Partition Large Graphs**: Split graphs > 10M vertices
5. **Use Appropriate Algorithms**: PageRank for influence, Louvain for communities
6. **Temporal Graphs**: Track timestamps for evolving networks
7. **Batch Updates**: Bulk insert edges for better performance
8. **Limit Traversal Depth**: Set max_depth to prevent infinite loops
9. **Embeddings for ML**: Use Node2Vec for graph-based ML tasks
10. **Monitor Memory**: Large graphs can consume significant RAM

---

**See Also**:
- [Specialized Engines Flow](/diagrams/08_specialized_engines_flow.md)
- [Graph Algorithms Reference](../reference/GRAPH_ALGORITHMS.md)
- [Performance Tuning](../operations/PERFORMANCE_TUNING.md)

**Document Version**: 1.0
**Last Updated**: December 2025
