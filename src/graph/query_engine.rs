// # Graph Query Engine
//
// Provides PGQL-like query parsing and execution for property graphs:
// - Pattern matching engine
// - Variable-length path queries
// - Shortest path algorithms (Dijkstra, A*)
// - Graph traversal operators (BFS, DFS)
// - Subgraph matching
// - Query optimization

use std::collections::VecDeque;
use std::collections::HashSet;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use crate::error::{Result, DbError};
use crate::common::Value;
use super::property_graph::{PropertyGraph, VertexId, EdgeId, Edge, Properties};

// ============================================================================
// Query AST (Abstract Syntax Tree)
// ============================================================================

// Graph query representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    // MATCH clauses
    pub match_clauses: Vec<MatchClause>,

    // WHERE clause (filter conditions)
    pub where_clause: Option<WhereClause>,

    // RETURN clause (projection)
    pub return_clause: ReturnClause,

    // ORDER BY clause
    pub order_by: Option<OrderByClause>,

    // LIMIT clause
    pub limit: Option<usize>,

    // SKIP clause
    pub skip: Option<usize>,
}

// MATCH clause for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchClause {
    // Graph patterns to match
    pub patterns: Vec<GraphPattern>,

    // Optional flag (OPTIONAL MATCH)
    pub optional: bool,
}

// Graph pattern (vertex-edge-vertex sequences)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    // Pattern elements (vertices and edges)
    pub elements: Vec<PatternElement>,
}

// Pattern element (vertex or edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternElement {
    // Vertex pattern
    Vertex(VertexPattern),

    // Edge pattern
    Edge(EdgePattern),
}

// Vertex pattern in a query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexPattern {
    // Variable name for this vertex
    pub variable: String,

    // Labels to match
    pub labels: Vec<String>,

    // Property constraints
    pub properties: HashMap<String, PropertyConstraint>,

    // Whether this is an existing binding
    pub is_bound: bool,
}

// Edge pattern in a query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePattern {
    // Variable name for this edge
    pub variable: Option<String>,

    // Edge labels to match
    pub labels: Vec<String>,

    // Property constraints
    pub properties: HashMap<String, PropertyConstraint>,

    // Direction (true = outgoing, false = incoming, None = undirected)
    pub direction: Option<bool>,

    // Path length constraints (for variable-length paths)
    pub length: PathLength,
}

// Path length specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathLength {
    // Exactly one hop
    Single,

    // Fixed number of hops
    Fixed(usize),

    // Range of hops (min, max)
    Range(usize, Option<usize>),

    // Any length (Kleene star)
    Any,
}

// Property constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyConstraint {
    // Equality constraint
    Equals(Value),

    // Not equals constraint
    NotEquals(Value),

    // Greater than
    GreaterThan(Value),

    // Less than
    LessThan(Value),

    // In list
    In(Vec<Value>),

    // Pattern matching (for strings)
    Like(String),

    // Null check
    IsNull,

    // Not null check
    IsNotNull,
}

// WHERE clause for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhereClause {
    // Filter expressions
    pub conditions: Vec<FilterExpression>,
}

// Filter expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterExpression {
    // Property comparison
    PropertyComparison {
        variable: String,
        property: String,
        constraint: PropertyConstraint,
    },

    // AND condition
    And(Box<FilterExpression>, Box<FilterExpression>),

    // OR condition
    Or(Box<FilterExpression>, Box<FilterExpression>),

    // NOT condition
    Not(Box<FilterExpression>),

    // Path exists
    PathExists(GraphPattern),
}

// RETURN clause for projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnClause {
    // Items to return
    pub items: Vec<ReturnItem>,

    // DISTINCT flag
    pub distinct: bool,
}

// Return item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReturnItem {
    // Return entire vertex
    Vertex(String),

    // Return entire edge
    Edge(String),

    // Return vertex property
    VertexProperty(String, String),

    // Return edge property
    EdgeProperty(String, String),

    // Aggregate function
    Aggregate(AggregateFunction),

    // Count
    Count(bool), // true for COUNT(*), false for COUNT(DISTINCT)

    // Path
    Path(String),
}

// Aggregate function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregateFunction {
    Sum(String, String),      // variable, property
    Avg(String, String),
    Min(String, String),
    Max(String, String),
    Collect(String),          // variable
}

// ORDER BY clause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByClause {
    pub items: Vec<OrderByItem>,
}

// Order by item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByItem {
    pub variable: String,
    pub property: Option<String>,
    pub ascending: bool,
}

// ============================================================================
// Query Results
// ============================================================================

// Query result set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    // Column names
    pub columns: Vec<String>,

    // Result rows
    pub rows: Vec<ResultRow>,

    // Execution time in milliseconds
    pub execution_time_ms: u64,

    // Number of rows
    pub row_count: usize,
}

// Single result row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultRow {
    // Values for each column
    pub values: Vec<ResultValue>,
}

// Result value (can be vertex, edge, property, or aggregate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultValue {
    // Vertex result
    Vertex(VertexId, Properties),

    // Edge result
    Edge(EdgeId, VertexId, VertexId, Properties),

    // Property value
    Property(Value),

    // Path result
    Path(Vec<VertexId>),

    // Null value
    Null,
}

// ============================================================================
// Pattern Matching Engine
// ============================================================================

// Variable bindings during pattern matching
#[derive(Debug, Clone)]
pub struct VariableBindings {
    // Vertex variable bindings
    pub vertices: HashMap<String, VertexId>,

    // Edge variable bindings
    pub edges: HashMap<String, EdgeId>,

    // Path variable bindings
    pub paths: HashMap<String, Vec<VertexId>>,
}

impl VariableBindings {
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            edges: HashMap::new(),
            paths: HashMap::new(),
        }
    }

    pub fn bind_vertex(&mut self, var: String, vertex_id: VertexId) {
        self.vertices.insert(var, vertex_id);
    }

    pub fn bind_edge(&mut self, var: String, edge_id: EdgeId) {
        self.edges.insert(var, edge_id);
    }

    pub fn get_vertex(&self, var: &str) -> Option<VertexId> {
        self.vertices.get(var).copied()
    }

    pub fn get_edge(&self, var: &str) -> Option<EdgeId> {
        self.edges.get(var).copied()
    }
}

// Pattern matcher for graph patterns
pub struct PatternMatcher<'a> {
    graph: &'a PropertyGraph,
}

impl<'a> PatternMatcher<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self { graph }
    }

    // Match a graph pattern and return all matching bindings
    pub fn match_pattern(&self, pattern: &GraphPattern) -> Result<Vec<VariableBindings>> {
        let mut results = vec![VariableBindings::new()];

        for element in &pattern.elements {
            results = self.match_element(element, results)?;
        }

        Ok(results)
    }

    // Match a single pattern element
    fn match_element(
        &self,
        element: &PatternElement,
        current_bindings: Vec<VariableBindings>,
    ) -> Result<Vec<VariableBindings>> {
        match element {
            PatternElement::Vertex(vertex_pattern) => {
                self.match_vertex_pattern(vertex_pattern, current_bindings)
            }
            PatternElement::Edge(edge_pattern) => {
                self.match_edge_pattern(edge_pattern, current_bindings)
            }
        }
    }

    // Match a vertex pattern
    fn match_vertex_pattern(
        &self,
        pattern: &VertexPattern,
        current_bindings: Vec<VariableBindings>,
    ) -> Result<Vec<VariableBindings>> {
        let mut new_bindings = Vec::new();

        for binding in current_bindings {
            // Check if variable is already bound
            if let Some(bound_id) = binding.get_vertex(&pattern.variable) {
                // Verify the bound vertex matches the pattern
                if self.vertex_matches_pattern(bound_id, pattern)? {
                    new_bindings.push(binding);
                }
            } else {
                // Find all vertices matching the pattern
                let candidates = self.find_matching_vertices(pattern)?;

                for vertex_id in candidates {
                    let mut new_binding = binding.clone();
                    new_binding.bind_vertex(pattern.variable.clone(), vertex_id);
                    new_bindings.push(new_binding);
                }
            }
        }

        Ok(new_bindings)
    }

    // Match an edge pattern
    fn match_edge_pattern(
        &self,
        pattern: &EdgePattern,
        current_bindings: Vec<VariableBindings>,
    ) -> Result<Vec<VariableBindings>> {
        let mut new_bindings = Vec::new();

        for binding in current_bindings {
            // Edge patterns typically connect two vertices
            // We need to find edges that match the pattern constraints

            // This is a simplified implementation
            // In practice, we'd look at the surrounding vertices in the pattern

            for edge in self.graph.edges() {
                if self.edge_matches_pattern(edge, pattern)? {
                    let mut new_binding = binding.clone();
                    if let Some(ref var) = pattern.variable {
                        new_binding.bind_edge(var.clone(), edge.id);
                    }
                    new_bindings.push(new_binding);
                }
            }
        }

        Ok(new_bindings)
    }

    // Check if a vertex matches a pattern
    fn vertex_matches_pattern(&self, vertex_id: VertexId, pattern: &VertexPattern) -> Result<bool> {
        let vertex = self.graph.get_vertex(vertex_id)
            .ok_or_else(|| DbError::Internal(format!("Vertex {} not found", vertex_id)))?;

        // Check labels
        if !pattern.labels.is_empty() {
            let has_matching_label = pattern.labels.iter()
                .any(|label| vertex.has_label(label));
            if !has_matching_label {
                return Ok(false);
            }
        }

        // Check property constraints
        for (prop_key, constraint) in &pattern.properties {
            if !self.check_property_constraint(vertex.properties.get(prop_key), constraint) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    // Check if an edge matches a pattern
    fn edge_matches_pattern(&self, edge: &Edge, pattern: &EdgePattern) -> Result<bool> {
        // Check labels
        if !pattern.labels.is_empty() {
            if !pattern.labels.contains(&edge.label) {
                return Ok(false);
            }
        }

        // Check property constraints
        for (prop_key, constraint) in &pattern.properties {
            if !self.check_property_constraint(edge.properties.get(prop_key), constraint) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    // Find all vertices matching a pattern
    fn find_matching_vertices(&self, pattern: &VertexPattern) -> Result<Vec<VertexId>> {
        let mut candidates = Vec::new();

        // If pattern has labels, use label index
        if !pattern.labels.is_empty() {
            for label in &pattern.labels {
                let vertices = self.graph.get_vertices_by_label(label);
                for vertex in vertices {
                    if self.vertex_matches_pattern(vertex.id, pattern)? {
                        candidates.push(vertex.id);
                    }
                }
            }
        } else {
            // Check all vertices
            for vertex in self.graph.vertices() {
                if self.vertex_matches_pattern(vertex.id, pattern)? {
                    candidates.push(vertex.id);
                }
            }
        }

        Ok(candidates)
    }

    // Check if a property value satisfies a constraint
    fn check_property_constraint(&self, value: Option<&Value>, constraint: &PropertyConstraint) -> bool {
        match constraint {
            PropertyConstraint::Equals(expected) => {
                value.map_or(false, |v| v == expected)
            }
            PropertyConstraint::NotEquals(expected) => {
                value.map_or(true, |v| v != expected)
            }
            PropertyConstraint::GreaterThan(expected) => {
                value.map_or(false, |v| self.compare_values(v, expected) == Some(Ordering::Greater))
            }
            PropertyConstraint::LessThan(expected) => {
                value.map_or(false, |v| self.compare_values(v, expected) == Some(Ordering::Less))
            }
            PropertyConstraint::In(values) => {
                value.map_or(false, |v| values.contains(v))
            }
            PropertyConstraint::IsNull => value.is_none(),
            PropertyConstraint::IsNotNull => value.is_some(),
            PropertyConstraint::Like(pattern) => {
                if let Some(Value::String(s)) = value {
                    // Simple pattern matching (could be enhanced with regex)
                    s.contains(pattern)
                } else {
                    false
                }
            }
        }
    }

    // Compare two values
    fn compare_values(&self, a: &Value, b: &Value) -> Option<Ordering> {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Some(x.cmp(y)),
            (Value::Float(x), Value::Float(y)) => {
                if x < y { Some(Ordering::Less) }
                else if x > y { Some(Ordering::Greater) }
                else { Some(Ordering::Equal) }
            }
            (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
            _ => None,
        }
    }
}

// ============================================================================
// Path Finding Algorithms
// ============================================================================

// Path finder for shortest path queries
pub struct PathFinder<'a> {
    graph: &'a PropertyGraph,
}

impl<'a> PathFinder<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self { graph }
    }

    // Find shortest path using BFS (unweighted)
    pub fn shortest_path_bfs(&self, start: VertexId, end: VertexId) -> Result<Option<Vec<VertexId>>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<VertexId, VertexId> = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == end {
                return Ok(Some(self.reconstruct_path(&parent, start, end)));
            }

            let neighbors = self.graph.get_outgoing_neighbors(current)?;
            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        Ok(None)
    }

    // Find shortest path using Dijkstra's algorithm (weighted)
    pub fn shortest_path_dijkstra(&self, start: VertexId, end: VertexId) -> Result<Option<(Vec<VertexId>, f64)>> {
        let mut distances: HashMap<VertexId, f64> = HashMap::new();
        let mut parent: HashMap<VertexId, VertexId> = HashMap::new();
        let mut heap = BinaryHeap::new();

        distances.insert(start, 0.0);
        heap.push(DijkstraState { vertex: start, cost: 0.0 });

        while let Some(DijkstraState { vertex, cost }) = heap.pop() {
            if vertex == end {
                return Ok(Some((
                    self.reconstruct_path(&parent, start, end),
                    cost,
                )));
            }

            if cost > *distances.get(&vertex).unwrap_or(&f64::INFINITY) {
                continue;
            }

            // Get outgoing edges
            if let Some(v) = self.graph.get_vertex(vertex) {
                for &edge_id in &v.outgoing_edges {
                    if let Some(edge) = self.graph.get_edge(edge_id) {
                        let next = edge.target;
                        let edge_weight = edge.weight.unwrap_or(1.0);
                        let next_cost = cost + edge_weight;

                        let current_dist = *distances.get(&next).unwrap_or(&f64::INFINITY);
                        if next_cost < current_dist {
                            distances.insert(next, next_cost);
                            parent.insert(next, vertex);
                            heap.push(DijkstraState { vertex: next, cost: next_cost });
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    // A* pathfinding with heuristic
    pub fn shortest_path_astar<F>(
        &self,
        start: VertexId,
        end: VertexId,
        heuristic: F,
    ) -> Result<Option<(Vec<VertexId>, f64)>>
    where
        F: Fn(VertexId, VertexId) -> f64,
    {
        let mut g_score: HashMap<VertexId, f64> = HashMap::new();
        let mut f_score: HashMap<VertexId, f64> = HashMap::new();
        let mut parent: HashMap<VertexId, VertexId> = HashMap::new();
        let mut heap = BinaryHeap::new();

        g_score.insert(start, 0.0);
        f_score.insert(start, heuristic(start, end));
        heap.push(AStarState { vertex: start, f_score: heuristic(start, end) });

        while let Some(AStarState { vertex, .. }) = heap.pop() {
            if vertex == end {
                return Ok(Some((
                    self.reconstruct_path(&parent, start, end),
                    *g_score.get(&vertex).unwrap(),
                )));
            }

            let current_g = *g_score.get(&vertex).unwrap_or(&f64::INFINITY);

            if let Some(v) = self.graph.get_vertex(vertex) {
                for &edge_id in &v.outgoing_edges {
                    if let Some(edge) = self.graph.get_edge(edge_id) {
                        let neighbor = edge.target;
                        let edge_weight = edge.weight.unwrap_or(1.0);
                        let tentative_g = current_g + edge_weight;

                        let neighbor_g = *g_score.get(&neighbor).unwrap_or(&f64::INFINITY);
                        if tentative_g < neighbor_g {
                            parent.insert(neighbor, vertex);
                            g_score.insert(neighbor, tentative_g);
                            let f = tentative_g + heuristic(neighbor, end);
                            f_score.insert(neighbor, f);
                            heap.push(AStarState { vertex: neighbor, f_score: f });
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    // Find all paths up to a certain length (variable-length path)
    pub fn find_variable_length_paths(
        &self,
        start: VertexId,
        end: VertexId,
        min_length: usize,
        max_length: usize,
    ) -> Result<Vec<Vec<VertexId>>> {
        let mut paths = Vec::new();
        let mut current_path = vec![start];
        let mut visited = HashSet::new();
        visited.insert(start);

        self.dfs_variable_length(
            start,
            end,
            min_length,
            max_length,
            &mut current_path,
            &mut visited,
            &mut paths,
        )?;

        Ok(paths)
    }

    // DFS helper for variable-length paths
    fn dfs_variable_length(
        &self,
        current: VertexId,
        end: VertexId,
        min_length: usize,
        max_length: usize,
        current_path: &mut Vec<VertexId>,
        visited: &mut HashSet<VertexId>,
        paths: &mut Vec<Vec<VertexId>>,
    ) -> Result<()> {
        if current == end && current_path.len() >= min_length {
            paths.push(current_path.clone());
            return Ok(());
        }

        if current_path.len() >= max_length {
            return Ok(());
        }

        let neighbors = self.graph.get_outgoing_neighbors(current)?;
        for neighbor in neighbors {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                current_path.push(neighbor);

                self.dfs_variable_length(
                    neighbor,
                    end,
                    min_length,
                    max_length,
                    current_path,
                    visited,
                    paths,
                )?;

                current_path.pop();
                visited.remove(&neighbor);
            }
        }

        Ok(())
    }

    // Reconstruct path from parent map
    fn reconstruct_path(
        &self,
        parent: &HashMap<VertexId, VertexId>,
        start: VertexId,
        end: VertexId,
    ) -> Vec<VertexId> {
        let mut path = vec![end];
        let mut current = end;

        while current != start {
            if let Some(&prev) = parent.get(&current) {
                path.push(prev);
                current = prev;
            } else {
                break;
            }
        }

        path.reverse();
        path
    }
}

// State for Dijkstra's algorithm
#[derive(Copy, Clone)]
struct DijkstraState {
    vertex: VertexId,
    cost: f64,
}

impl Eq for DijkstraState {}

impl PartialEq for DijkstraState {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.vertex == other.vertex
    }
}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.cost.partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.vertex.cmp(&other.vertex))
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// State for A* algorithm
#[derive(Copy, Clone)]
struct AStarState {
    vertex: VertexId,
    f_score: f64,
}

impl Eq for AStarState {}

impl PartialEq for AStarState {
    fn eq(&self, other: &Self) -> bool {
        self.f_score == other.f_score && self.vertex == other.vertex
    }
}

impl Ord for AStarState {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.f_score.partial_cmp(&self.f_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.vertex.cmp(&other.vertex))
    }
}

impl PartialOrd for AStarState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ============================================================================
// Graph Traversal
// ============================================================================

// Graph traversal strategies
pub struct GraphTraversal<'a> {
    graph: &'a PropertyGraph,
}

impl<'a> GraphTraversal<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self { graph }
    }

    // Breadth-first search traversal
    pub fn bfs<F>(&self, start: VertexId, mut visitor: F) -> Result<()>
    where
        F: FnMut(VertexId, usize) -> bool, // Returns true to continue, false to stop
    {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut depths = HashMap::new();

        queue.push_back(start);
        visited.insert(start);
        depths.insert(start, 0);

        while let Some(current) = queue.pop_front() {
            let depth = *depths.get(&current).unwrap();

            if !visitor(current, depth) {
                break;
            }

            let neighbors = self.graph.get_outgoing_neighbors(current)?;
            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    depths.insert(neighbor, depth + 1);
                    queue.push_back(neighbor);
                }
            }
        }

        Ok(())
    }

    // Depth-first search traversal
    pub fn dfs<F>(&self, start: VertexId, visitor: &mut F) -> Result<()>
    where
        F: FnMut(VertexId, usize) -> bool,
    {
        let mut visited = HashSet::new();
        self.dfs_recursive(start, 0, &mut visited, visitor)?;
        Ok(())
    }

    fn dfs_recursive<F>(
        &self,
        current: VertexId,
        depth: usize,
        visited: &mut HashSet<VertexId>,
        visitor: &mut F,
    ) -> Result<bool>
    where
        F: FnMut(VertexId, usize) -> bool,
    {
        visited.insert(current);

        if !visitor(current, depth) {
            return Ok(false);
        }

        let neighbors = self.graph.get_outgoing_neighbors(current)?;
        for neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if !self.dfs_recursive(neighbor, depth + 1, visited, visitor)? {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}

// ============================================================================
// Query Executor
// ============================================================================

// Main query execution engine
pub struct QueryExecutor<'a> {
    graph: &'a PropertyGraph,
    pattern_matcher: PatternMatcher<'a>,
    #[allow(dead_code)]
    path_finder: PathFinder<'a>,
}

impl<'a> QueryExecutor<'a> {
    pub fn new(graph: &'a PropertyGraph) -> Self {
        Self {
            graph,
            pattern_matcher: PatternMatcher::new(graph),
            path_finder: PathFinder::new(graph),
        }
    }

    // Execute a graph query
    pub fn execute(&self, query: &GraphQuery) -> Result<QueryResult> {
        let start_time = std::time::Instant::now();

        // Execute MATCH clauses
        let mut bindings = vec![VariableBindings::new()];
        for match_clause in &query.match_clauses {
            bindings = self.execute_match_clause(match_clause, bindings)?;
        }

        // Apply WHERE clause
        if let Some(ref where_clause) = query.where_clause {
            bindings = self.apply_where_clause(where_clause, bindings)?;
        }

        // Apply ORDER BY
        if let Some(ref order_by) = query.order_by {
            bindings = self.apply_order_by(order_by, bindings)?;
        }

        // Apply SKIP and LIMIT
        if let Some(skip) = query.skip {
            bindings = bindings.into_iter().skip(skip).collect();
        }
        if let Some(limit) = query.limit {
            bindings.truncate(limit);
        }

        // Project results according to RETURN clause
        let (columns, rows) = self.project_results(&query.return_clause, &bindings)?;

        let execution_time = start_time.elapsed();

        Ok(QueryResult {
            columns,
            row_count: rows.len(),
            rows,
            execution_time_ms: execution_time.as_millis() as u64,
        })
    }

    fn execute_match_clause(
        &self,
        match_clause: &MatchClause,
        current_bindings: Vec<VariableBindings>,
    ) -> Result<Vec<VariableBindings>> {
        let mut new_bindings = Vec::new();

        for pattern in &match_clause.patterns {
            let pattern_matches = self.pattern_matcher.match_pattern(pattern)?;

            if current_bindings.is_empty() || current_bindings.len() == 1 && current_bindings[0].vertices.is_empty() {
                new_bindings.extend(pattern_matches);
            } else {
                // Join with existing bindings
                for binding in &current_bindings {
                    for pattern_match in &pattern_matches {
                        // Merge bindings
                        let mut merged = binding.clone();
                        merged.vertices.extend(pattern_match.vertices.clone());
                        merged.edges.extend(pattern_match.edges.clone());
                        new_bindings.push(merged);
                    }
                }
            }
        }

        Ok(new_bindings)
    }

    fn apply_where_clause(
        &self,
        where_clause: &WhereClause,
        bindings: Vec<VariableBindings>,
    ) -> Result<Vec<VariableBindings>> {
        Ok(bindings.into_iter()
            .filter(|binding| {
                where_clause.conditions.iter().all(|cond| {
                    self.evaluate_filter_expression(cond, binding).unwrap_or(false)
                })
            })
            .collect::<Vec<_>>())
    }

    fn evaluate_filter_expression(
        &self,
        expr: &FilterExpression,
        binding: &VariableBindings,
    ) -> Result<bool> {
        match expr {
            FilterExpression::PropertyComparison { variable, property, constraint } => {
                if let Some(vertex_id) = binding.get_vertex(variable) {
                    if let Some(vertex) = self.graph.get_vertex(vertex_id) {
                        let value = vertex.properties.get(property);
                        return Ok(self.pattern_matcher.check_property_constraint(value, constraint));
                    }
                }
                Ok(false)
            }
            FilterExpression::And(left, right) => {
                Ok(self.evaluate_filter_expression(left, binding)? &&
                   self.evaluate_filter_expression(right, binding)?)
            }
            FilterExpression::Or(left, right) => {
                Ok(self.evaluate_filter_expression(left, binding)? ||
                   self.evaluate_filter_expression(right, binding)?)
            }
            FilterExpression::Not(inner) => {
                Ok(!self.evaluate_filter_expression(inner, binding)?)
            }
            FilterExpression::PathExists(_) => {
                // Simplified path existence check
                Ok(true)
            }
        }
    }

    fn apply_order_by(
        &self,
        _order_by: &OrderByClause,
        bindings: Vec<VariableBindings>,
    ) -> Result<Vec<VariableBindings>> {
        // Simplified ordering - would need more complex comparison logic
        // For now, just return as-is
        Ok(bindings)
    }

    fn project_results(
        &self,
        return_clause: &ReturnClause,
        bindings: &[VariableBindings],
    ) -> Result<(Vec<String>, Vec<ResultRow>)> {
        let mut columns = Vec::new();
        let mut rows = Vec::new();

        // Determine column names
        for item in &return_clause.items {
            match item {
                ReturnItem::Vertex(var) => columns.push(var.clone()),
                ReturnItem::VertexProperty(var, prop) => {
                    columns.push(format!("{}.{}", var, prop));
                }
                ReturnItem::Count(_) => columns.push("count".to_string()),
                _ => columns.push("result".to_string()),
            }
        }

        // Project each binding
        for binding in bindings {
            let mut values = Vec::new();

            for item in &return_clause.items {
                let value = match item {
                    ReturnItem::Vertex(var) => {
                        if let Some(vertex_id) = binding.get_vertex(var) {
                            if let Some(vertex) = self.graph.get_vertex(vertex_id) {
                                ResultValue::Vertex(vertex_id, vertex.properties.clone())
                            } else {
                                ResultValue::Null
                            }
                        } else {
                            ResultValue::Null
                        }
                    }
                    ReturnItem::VertexProperty(var, prop) => {
                        if let Some(vertex_id) = binding.get_vertex(var) {
                            if let Some(vertex) = self.graph.get_vertex(vertex_id) {
                                if let Some(val) = vertex.properties.get(prop) {
                                    ResultValue::Property(val.clone())
                                } else {
                                    ResultValue::Null
                                }
                            } else {
                                ResultValue::Null
                            }
                        } else {
                            ResultValue::Null
                        }
                    }
                    ReturnItem::Count(_) => {
                        ResultValue::Property(Value::Integer(bindings.len() as i64))
                    }
                    _ => ResultValue::Null,
                };

                values.push(value);
            }

            rows.push(ResultRow { values });
        }

        Ok((columns, rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::property_graph::EdgeDirection;
use std::time::Instant;

    #[test]
    fn test_pattern_matching() {
        let mut graph = PropertyGraph::new();
        let v1 = graph.add_vertex(vec!["Person".to_string()], Properties::new()).unwrap();
        let v2 = graph.add_vertex(vec!["Person".to_string()], Properties::new()).unwrap();
        graph.add_edge(v1, v2, "KNOWS".to_string(), Properties::new(), EdgeDirection::Directed).unwrap();

        let matcher = PatternMatcher::new(&graph);
        let pattern = GraphPattern {
            elements: vec![
                PatternElement::Vertex(VertexPattern {
                    variable: "a".to_string(),
                    labels: vec!["Person".to_string()],
                    properties: HashMap::new(),
                    is_bound: false,
                }),
            ],
        };

        let results = matcher.match_pattern(&pattern).unwrap();
        assert_eq!(results.len(), 2); // Should match both vertices
    }
}
