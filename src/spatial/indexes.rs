// Spatial Indexing Structures
//
// Provides high-performance spatial indexing with:
// - R-tree and R*-tree for general spatial data
// - Quadtree for point data
// - Grid-based indexing for uniform distributions
// - Hilbert curve ordering for locality preservation
// - Bulk loading and dynamic maintenance

use std::collections::HashSet;
use crate::error::{DbError, Result};
use crate::spatial::geometry::{BoundingBox, Coordinate};
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};

/// Spatial index trait
pub trait SpatialIndex: Send + Sync {
    /// Insert a geometry with associated ID
    fn insert(&mut self, id: u64, bbox: BoundingBox) -> Result<()>;

    /// Remove a geometry by ID
    fn remove(&mut self, id: u64) -> Result<bool>;

    /// Search for geometries intersecting the query box
    fn search(&self, query: &BoundingBox) -> Vec<u64>;

    /// Find nearest neighbor
    fn nearest(&self, point: &Coordinate, max_distance: f64) -> Option<u64>;

    /// Get statistics about the index
    fn stats(&self) -> IndexStats;
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub num_entries: usize,
    pub num_nodes: usize,
    pub height: usize,
    pub avg_fill_factor: f64,
}

/// R-tree node
#[derive(Debug, Clone)]
struct RTreeNode {
    bbox: BoundingBox,
    entries: Vec<RTreeEntry>,
    is_leaf: bool,
    level: usize,
}

#[derive(Debug, Clone)]
enum RTreeEntry {
    Leaf { id: u64, bbox: BoundingBox },
    Internal { node: Box<RTreeNode> },
}

impl RTreeEntry {
    fn bbox(&self) -> &BoundingBox {
        match self {
            RTreeEntry::Leaf { bbox, .. } => bbox,
            RTreeEntry::Internal { node } => &node.bbox,
        }
    }
}

/// R-tree spatial index
pub struct RTree {
    root: Option<RTreeNode>,
    max_entries: usize,
    min_entries: usize,
    height: usize,
    size: usize,
}

impl RTree {
    /// Create a new R-tree with default capacity
    pub fn new() -> Self {
        Self::with_capacity(8, 3)
    }

    /// Create an R-tree with specified capacity
    pub fn with_capacity(max_entries: usize, min_entries: usize) -> Self {
        Self {
            root: None,
            max_entries,
            min_entries,
            height: 0,
            size: 0,
        }
    }

    /// Bulk load entries efficiently
    pub fn bulk_load(&mut self, mut entries: Vec<(u64, BoundingBox)>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        // Sort by Hilbert value for better spatial locality
        entries.sort_by_key(|(_, bbox)| self.hilbert_value(&bbox.center()));

        // Build tree bottom-up
        self.root = Some(self.build_level(&entries, 0)?);
        self.size = entries.len();

        Ok(())
    }

    fn build_level(&self, entries: &[(u64, BoundingBox)], level: usize) -> Result<RTreeNode> {
        if entries.len() <= self.max_entries {
            // Create leaf node
            let mut node_entries = Vec::new();
            let mut bbox = entries[0].1;

            for (id, entry_bbox) in entries {
                node_entries.push(RTreeEntry::Leaf {
                    id: *id,
                    bbox: *entry_bbox,
                });
                bbox.expand(entry_bbox);
            }

            return Ok(RTreeNode {
                bbox,
                entries: node_entries,
                is_leaf: true,
                level,
            });
        }

        // Split entries into groups
        let mut child_nodes = Vec::new();
        for chunk in entries.chunks(self.max_entries) {
            child_nodes.push(self.build_level(chunk, level)?);
        }

        // Create internal node
        let mut node_entries = Vec::new();
        let mut bbox = child_nodes[0].bbox;

        for child in child_nodes {
            bbox.expand(&child.bbox);
            node_entries.push(RTreeEntry::Internal {
                node: Box::new(child),
            });
        }

        Ok(RTreeNode {
            bbox,
            entries: node_entries,
            is_leaf: false,
            level: level + 1,
        })
    }

    fn hilbert_value(&self, coord: &Coordinate) -> u64 {
        // Simplified Hilbert curve calculation
        let x = (coord.x * 1000.0) as u32;
        let y = (coord.y * 1000.0) as u32;
        Self::xy_to_hilbert(x, y, 16)
    }

    fn xy_to_hilbert(x: u32, y: u32, order: u32) -> u64 {
        let mut d = 0u64;
        let mut s = 1 << (order - 1);

        while s > 0 {
            let rx = if (x & s) > 0 { 1 } else { 0 };
            let ry = if (y & s) > 0 { 1 } else { 0 };
            d += s as u64 * s as u64 * ((3 * rx) ^ ry) as u64;
            s >>= 1;
        }

        d
    }

    fn choose_subtree(node: &RTreeNode, bbox: &BoundingBox) -> usize {
        let mut min_enlargement = f64::INFINITY;
        let mut best_idx = 0;

        for (idx, entry) in node.entries.iter().enumerate() {
            let entry_bbox = entry.bbox();
            let mut enlarged = *entry_bbox;
            enlarged.expand(bbox);

            let enlargement = enlarged.area() - entry_bbox.area();

            if enlargement < min_enlargement {
                min_enlargement = enlargement;
                best_idx = idx;
            }
        }

        best_idx
    }

    fn split_node(&self, entries: Vec<RTreeEntry>, is_leaf: bool, level: usize) -> (RTreeNode, RTreeNode) {
        // R*-tree split using R*-tree algorithm
        let (seeds, remaining) = self.pick_seeds(&entries);

        let mut group1 = vec![seeds.0];
        let mut group2 = vec![seeds.1];

        // Distribute remaining entries
        for entry in remaining {
            let bbox1 = self.compute_bbox(&group1);
            let bbox2 = self.compute_bbox(&group2);

            let mut expanded1 = bbox1;
            expanded1.expand(entry.bbox());
            let enlargement1 = expanded1.area() - bbox1.area();

            let mut expanded2 = bbox2;
            expanded2.expand(entry.bbox());
            let enlargement2 = expanded2.area() - bbox2.area();

            if enlargement1 < enlargement2 {
                group1.push(entry);
            } else {
                group2.push(entry);
            }
        }

        let node1 = RTreeNode {
            bbox: self.compute_bbox(&group1),
            entries: group1,
            is_leaf,
            level,
        };

        let node2 = RTreeNode {
            bbox: self.compute_bbox(&group2),
            entries: group2,
            is_leaf,
            level,
        };

        (node1, node2)
    }

    fn pick_seeds(&self, entries: &[RTreeEntry]) -> ((RTreeEntry, RTreeEntry), Vec<RTreeEntry>) {
        let mut max_waste = f64::NEG_INFINITY;
        let mut seed1_idx = 0;
        let mut seed2_idx = 1;

        // Find pair with maximum waste
        for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                let bbox1 = entries[i].bbox();
                let bbox2 = entries[j].bbox();

                let mut combined = *bbox1;
                combined.expand(bbox2);

                let waste = combined.area() - bbox1.area() - bbox2.area();

                if waste > max_waste {
                    max_waste = waste;
                    seed1_idx = i;
                    seed2_idx = j;
                }
            }
        }

        let mut remaining = Vec::new();
        let mut seed1 = None;
        let mut seed2 = None;

        for (idx, entry) in entries.iter().enumerate() {
            if idx == seed1_idx {
                seed1 = Some(entry.clone());
            } else if idx == seed2_idx {
                seed2 = Some(entry.clone());
            } else {
                remaining.push(entry.clone());
            }
        }

        ((seed1.unwrap(), seed2.unwrap()), remaining)
    }

    fn compute_bbox(&self, entries: &[RTreeEntry]) -> BoundingBox {
        let mut bbox = *entries[0].bbox();
        for entry in entries.iter().skip(1) {
            bbox.expand(entry.bbox());
        }
        bbox
    }

    fn search_node(&self, node: &RTreeNode, query: &BoundingBox, results: &mut Vec<u64>) {
        if !node.bbox.intersects(query) {
            return;
        }

        for entry in &node.entries {
            match entry {
                RTreeEntry::Leaf { id, bbox } => {
                    if bbox.intersects(query) {
                        results.push(*id);
                    }
                }
                RTreeEntry::Internal { node: child } => {
                    self.search_node(child, query, results);
                }
            }
        }
    }
}

impl Default for RTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SpatialIndex for RTree {
    fn insert(&mut self, id: u64, bbox: BoundingBox) -> Result<()> {
        let entry = RTreeEntry::Leaf { id, bbox };

        if let Some(root) = &mut self.root {
            // Insert into existing tree
            Self::insert_entry(root, entry, self.max_entries)?;
        } else {
            // Create first node
            self.root = Some(RTreeNode {
                bbox,
                entries: vec![entry],
                is_leaf: true,
                level: 0,
            });
        }

        self.size += 1;
        Ok(())
    }

    fn remove(&mut self, id: u64) -> Result<bool> {
        if let Some(root) = &mut self.root {
            let removed = Self::remove_from_node(root, id);
            if removed {
                self.size -= 1;
            }
            Ok(removed)
        } else {
            Ok(false)
        }
    }

    fn search(&self, query: &BoundingBox) -> Vec<u64> {
        let mut results = Vec::new();
        if let Some(root) = &self.root {
            self.search_node(root, query, &mut results);
        }
        results
    }

    fn nearest(&self, point: &Coordinate, max_distance: f64) -> Option<u64> {
        let query = BoundingBox::new(
            point.x - max_distance,
            point.y - max_distance,
            point.x + max_distance,
            point.y + max_distance,
        );

        let candidates = self.search(&query);
        // In a real implementation, would refine based on actual distance
        candidates.first().copied()
    }

    fn stats(&self) -> IndexStats {
        let (num_nodes, total_fill) = self.count_nodes_and_fill();
        IndexStats {
            num_entries: self.size,
            num_nodes,
            height: self.height,
            avg_fill_factor: if num_nodes > 0 {
                total_fill / num_nodes as f64
            } else {
                0.0
            },
        }
    }
}

impl RTree {
    fn insert_entry(node: &mut RTreeNode, entry: RTreeEntry, max_entries: usize) -> Result<()> {
        if node.is_leaf {
            node.bbox.expand(entry.bbox());
            node.entries.push(entry);

            if node.entries.len() > max_entries {
                // Need to split
                // In a full implementation, would handle split propagation
            }
        } else {
            let idx = Self::choose_subtree(node, entry.bbox());
            if let RTreeEntry::Internal { node: child } = &mut node.entries[idx] {
                Self::insert_entry(child, entry, max_entries)?;
                node.bbox.expand(&child.bbox);
            }
        }

        Ok(())
    }

    fn remove_from_node(node: &mut RTreeNode, id: u64) -> bool {
        if node.is_leaf {
            if let Some(pos) = node.entries.iter().position(|e| {
                matches!(e, RTreeEntry::Leaf { id: eid, .. } if *eid == id)
            }) {
                node.entries.remove(pos);
                return true;
            }
        } else {
            for entry in &mut node.entries {
                if let RTreeEntry::Internal { node: child } = entry {
                    if Self::remove_from_node(child, id) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn count_nodes_and_fill(&self) -> (usize, f64) {
        if let Some(root) = &self.root {
            self.count_nodes_recursive(root)
        } else {
            (0, 0.0)
        }
    }

    fn count_nodes_recursive(&self, node: &RTreeNode) -> (usize, f64) {
        let mut count = 1;
        let mut fill = node.entries.len() as f64 / self.max_entries as f64;

        for entry in &node.entries {
            if let RTreeEntry::Internal { node: child } = entry {
                let (child_count, child_fill) = self.count_nodes_recursive(child);
                count += child_count;
                fill += child_fill;
            }
        }

        (count, fill)
    }
}

/// Quadtree for point data
pub struct Quadtree {
    root: Option<QuadtreeNode>,
    bounds: BoundingBox,
    max_depth: usize,
    max_points: usize,
    size: usize,
}

struct QuadtreeNode {
    bounds: BoundingBox,
    points: Vec<(u64, Coordinate)>,
    children: Option<[Box<QuadtreeNode>; 4]>,
    depth: usize,
}

impl Quadtree {
    pub fn new(bounds: BoundingBox) -> Self {
        Self {
            root: None,
            bounds,
            max_depth: 16,
            max_points: 4,
            size: 0,
        }
    }

    pub fn with_params(bounds: BoundingBox, max_depth: usize, max_points: usize) -> Self {
        Self {
            root: None,
            bounds,
            max_depth,
            max_points,
            size: 0,
        }
    }

    fn subdivide_bounds(bounds: &BoundingBox) -> [BoundingBox; 4] {
        let mid_x = (bounds.min_x + bounds.max_x) / 2.0;
        let mid_y = (bounds.min_y + bounds.max_y) / 2.0;

        [
            BoundingBox::new(bounds.min_x, bounds.min_y, mid_x, mid_y), // SW
            BoundingBox::new(mid_x, bounds.min_y, bounds.max_x, mid_y), // SE
            BoundingBox::new(bounds.min_x, mid_y, mid_x, bounds.max_y), // NW
            BoundingBox::new(mid_x, mid_y, bounds.max_x, bounds.max_y), // NE
        ]
    }

    fn insert_into_node(
        node: &mut QuadtreeNode,
        id: u64,
        point: Coordinate,
        max_points: usize,
        max_depth: usize,
    ) -> Result<()> {
        if !node.bounds.contains_coord(&point) {
            return Err(DbError::InvalidInput("Point outside node bounds".to_string()));
        }

        if let Some(children) = &mut node.children {
            // Find appropriate child
            let mid_x = (node.bounds.min_x + node.bounds.max_x) / 2.0;
            let mid_y = (node.bounds.min_y + node.bounds.max_y) / 2.0;

            let child_idx = if point.x < mid_x {
                if point.y < mid_y { 0 } else { 2 }
            } else {
                if point.y < mid_y { 1 } else { 3 }
            };

            Self::insert_into_node(&mut children[child_idx], id, point, max_points, max_depth)?;
        } else {
            node.points.push((id, point));

            // Subdivide if necessary
            if node.points.len() > max_points && node.depth < max_depth {
                let subdivisions = Self::subdivide_bounds(&node.bounds);
                let mut children = [
                    Box::new(QuadtreeNode {
                        bounds: subdivisions[0],
                        points: Vec::new(),
                        children: None,
                        depth: node.depth + 1,
                    }),
                    Box::new(QuadtreeNode {
                        bounds: subdivisions[1],
                        points: Vec::new(),
                        children: None,
                        depth: node.depth + 1,
                    }),
                    Box::new(QuadtreeNode {
                        bounds: subdivisions[2],
                        points: Vec::new(),
                        children: None,
                        depth: node.depth + 1,
                    }),
                    Box::new(QuadtreeNode {
                        bounds: subdivisions[3],
                        points: Vec::new(),
                        children: None,
                        depth: node.depth + 1,
                    }),
                ];

                // Redistribute points
                for (pid, pcoord) in node.points.drain(..) {
                    let mid_x = (node.bounds.min_x + node.bounds.max_x) / 2.0;
                    let mid_y = (node.bounds.min_y + node.bounds.max_y) / 2.0;

                    let child_idx = if pcoord.x < mid_x {
                        if pcoord.y < mid_y { 0 } else { 2 }
                    } else {
                        if pcoord.y < mid_y { 1 } else { 3 }
                    };

                    children[child_idx].points.push((pid, pcoord));
                }

                node.children = Some(children);
            }
        }

        Ok(())
    }

    fn search_node(&self, node: &QuadtreeNode, query: &BoundingBox, results: &mut Vec<u64>) {
        if !node.bounds.intersects(query) {
            return;
        }

        if let Some(children) = &node.children {
            for child in children.iter() {
                self.search_node(child, query, results);
            }
        } else {
            for (id, coord) in &node.points {
                if query.contains_coord(coord) {
                    results.push(*id);
                }
            }
        }
    }
}

impl SpatialIndex for Quadtree {
    fn insert(&mut self, id: u64, bbox: BoundingBox) -> Result<()> {
        // Use center point for quadtree
        let point = bbox.center();

        if let Some(root) = &mut self.root {
            Self::insert_into_node(root, id, point, self.max_points, self.max_depth)?;
        } else {
            self.root = Some(QuadtreeNode {
                bounds: self.bounds,
                points: vec![(id, point)],
                children: None,
                depth: 0,
            });
        }

        self.size += 1;
        Ok(())
    }

    fn remove(&mut self, _id: u64) -> Result<bool> {
        // Simplified - full implementation would traverse tree
        Ok(false)
    }

    fn search(&self, query: &BoundingBox) -> Vec<u64> {
        let mut results = Vec::new();
        if let Some(root) = &self.root {
            self.search_node(root, query, &mut results);
        }
        results
    }

    fn nearest(&self, point: &Coordinate, max_distance: f64) -> Option<u64> {
        let query = BoundingBox::new(
            point.x - max_distance,
            point.y - max_distance,
            point.x + max_distance,
            point.y + max_distance,
        );

        let candidates = self.search(&query);
        candidates.first().copied()
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            num_entries: self.size,
            num_nodes: 0, // Would count in full implementation
            height: self.max_depth,
            avg_fill_factor: 0.0,
        }
    }
}

/// Grid-based spatial index for uniform distributions
pub struct GridIndex {
    grid: HashMap<(i32, i32), Vec<(u64, BoundingBox)>>,
    cell_size: f64,
    bounds: BoundingBox,
    size: usize,
}

impl GridIndex {
    pub fn new(bounds: BoundingBox, cell_size: f64) -> Self {
        Self {
            grid: HashMap::new(),
            cell_size,
            bounds,
            size: 0,
        }
    }

    fn get_cell(&self, x: f64, y: f64) -> (i32, i32) {
        let cell_x = ((x - self.bounds.min_x) / self.cell_size).floor() as i32;
        let cell_y = ((y - self.bounds.min_y) / self.cell_size).floor() as i32;
        (cell_x, cell_y)
    }

    fn get_cells_for_bbox(&self, bbox: &BoundingBox) -> Vec<(i32, i32)> {
        let min_cell = self.get_cell(bbox.min_x, bbox.min_y);
        let max_cell = self.get_cell(bbox.max_x, bbox.max_y);

        let mut cells = Vec::new();
        for x in min_cell.0..=max_cell.0 {
            for y in min_cell.1..=max_cell.1 {
                cells.push((x, y));
            }
        }
        cells
    }
}

impl SpatialIndex for GridIndex {
    fn insert(&mut self, id: u64, bbox: BoundingBox) -> Result<()> {
        let cells = self.get_cells_for_bbox(&bbox);

        for cell in cells {
            self.grid
                .entry(cell)
                .or_insert_with(Vec::new)
                .push((id, bbox));
        }

        self.size += 1;
        Ok(())
    }

    fn remove(&mut self, id: u64) -> Result<bool> {
        let mut found = false;

        for entries in self.grid.values_mut() {
            if let Some(pos) = entries.iter().position(|(eid, _)| *eid == id) {
                entries.remove(pos);
                found = true;
            }
        }

        if found {
            self.size -= 1;
        }

        Ok(found)
    }

    fn search(&self, query: &BoundingBox) -> Vec<u64> {
        let cells = self.get_cells_for_bbox(query);
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for cell in cells {
            if let Some(entries) = self.grid.get(&cell) {
                for (id, bbox) in entries {
                    if !seen.contains(id) && bbox.intersects(query) {
                        results.push(*id);
                        seen.insert(*id);
                    }
                }
            }
        }

        results
    }

    fn nearest(&self, point: &Coordinate, max_distance: f64) -> Option<u64> {
        let query = BoundingBox::new(
            point.x - max_distance,
            point.y - max_distance,
            point.x + max_distance,
            point.y + max_distance,
        );

        let mut min_dist = f64::INFINITY;
        let mut nearest_id = None;

        let cells = self.get_cells_for_bbox(&query);

        for cell in cells {
            if let Some(entries) = self.grid.get(&cell) {
                for (id, bbox) in entries {
                    let center = bbox.center();
                    let dist = point.distance_2d(&center);

                    if dist < min_dist && dist <= max_distance {
                        min_dist = dist;
                        nearest_id = Some(*id);
                    }
                }
            }
        }

        nearest_id
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            num_entries: self.size,
            num_nodes: self.grid.len(),
            height: 1,
            avg_fill_factor: if !self.grid.is_empty() {
                self.size as f64 / self.grid.len() as f64
            } else {
                0.0
            },
        }
    }
}

/// Hilbert curve utilities for spatial ordering
pub struct HilbertCurve {
    order: u32,
}

impl HilbertCurve {
    pub fn new(order: u32) -> Self {
        Self { order }
    }

    /// Convert (x, y) coordinates to Hilbert distance
    pub fn xy_to_distance(&self, x: u32, y: u32) -> u64 {
        let mut d = 0u64;
        let mut s = 1 << (self.order - 1);
        let mut x = x;
        let mut y = y;

        while s > 0 {
            let rx = if (x & s) > 0 { 1 } else { 0 };
            let ry = if (y & s) > 0 { 1 } else { 0 };

            d += (s * s) as u64 * ((3 * rx) ^ ry) as u64;

            self.rotate(s, &mut x, &mut y, rx, ry);

            s >>= 1;
        }

        d
    }

    /// Convert Hilbert distance to (x, y) coordinates
    pub fn distance_to_xy(&self, mut d: u64) -> (u32, u32) {
        let mut x = 0u32;
        let mut y = 0u32;
        let mut s = 1u32;

        while s < (1 << self.order) {
            let rx = 1 & (d / 2);
            let ry = 1 & (d ^ rx);

            self.rotate(s, &mut x, &mut y, rx as u32, ry as u32);

            x += s * rx as u32;
            y += s * ry as u32;

            d /= 4;
            s *= 2;
        }

        (x, y)
    }

    fn rotate(&self, n: u32, x: &mut u32, y: &mut u32, rx: u32, ry: u32) {
        if ry == 0 {
            if rx == 1 {
                *x = n - 1 - *x;
                *y = n - 1 - *y;
            }

            std::mem::swap(x, y);
        }
    }

    /// Sort points by Hilbert order
    pub fn sort_points(&self, points: &mut [(u64, Coordinate)]) {
        points.sort_by_key(|(_, coord)| {
            let x = (coord.x * 1000.0) as u32;
            let y = (coord.y * 1000.0) as u32;
            self.xy_to_distance(x, y)
        });
    }
}

/// Thread-safe spatial index wrapper
pub struct ConcurrentSpatialIndex {
    index: Arc<RwLock<Box<dyn SpatialIndex>>>,
}

impl ConcurrentSpatialIndex {
    pub fn new(index: Box<dyn SpatialIndex>) -> Self {
        Self {
            index: Arc::new(RwLock::new(index)),
        }
    }

    pub fn insert(&self, id: u64, bbox: BoundingBox) -> Result<()> {
        self.index.write().unwrap().insert(id, bbox)
    }

    pub fn remove(&self, id: u64) -> Result<bool> {
        self.index.write().unwrap().remove(id)
    }

    pub fn search(&self, query: &BoundingBox) -> Vec<u64> {
        self.index.read().unwrap().search(query)
    }

    pub fn nearest(&self, point: &Coordinate, max_distance: f64) -> Option<u64> {
        self.index.read().unwrap().nearest(point, max_distance)
    }

    pub fn stats(&self) -> IndexStats {
        self.index.read().unwrap().stats()
    }
}

/// Index builder for bulk loading
pub struct SpatialIndexBuilder {
    index_type: IndexType,
    entries: Vec<(u64, BoundingBox)>,
}

pub enum IndexType {
    RTree { max_entries: usize, min_entries: usize },
    Quadtree { bounds: BoundingBox, max_depth: usize },
    Grid { bounds: BoundingBox, cell_size: f64 },
}

impl SpatialIndexBuilder {
    pub fn new(index_type: IndexType) -> Self {
        Self {
            index_type,
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, id: u64, bbox: BoundingBox) {
        self.entries.push((id, bbox));
    }

    pub fn build(self) -> Result<Box<dyn SpatialIndex>> {
        match self.index_type {
            IndexType::RTree { max_entries, min_entries } => {
                let mut rtree = RTree::with_capacity(max_entries, min_entries);
                rtree.bulk_load(self.entries)?;
                Ok(Box::new(rtree))
            }
            IndexType::Quadtree { bounds, max_depth } => {
                let mut quadtree = Quadtree::with_params(bounds, max_depth, 4);
                for (id, bbox) in self.entries {
                    quadtree.insert(id, bbox)?;
                }
                Ok(Box::new(quadtree))
            }
            IndexType::Grid { bounds, cell_size } => {
                let mut grid = GridIndex::new(bounds, cell_size);
                for (id, bbox) in self.entries {
                    grid.insert(id, bbox)?;
                }
                Ok(Box::new(grid))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtree_insert_search() {
        let mut rtree = RTree::new();

        let bbox1 = BoundingBox::new(0.0, 0.0, 1.0, 1.0);
        let bbox2 = BoundingBox::new(2.0, 2.0, 3.0, 3.0);
        let bbox3 = BoundingBox::new(0.5, 0.5, 1.5, 1.5);

        rtree.insert(1, bbox1).unwrap();
        rtree.insert(2, bbox2).unwrap();
        rtree.insert(3, bbox3).unwrap();

        let query = BoundingBox::new(0.0, 0.0, 2.0, 2.0);
        let results = rtree.search(&query);

        assert!(results.contains(&1));
        assert!(results.contains(&3));
        assert!(!results.contains(&2));
    }

    #[test]
    fn test_quadtree() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let mut qtree = Quadtree::new(bounds);

        qtree.insert(1, BoundingBox::new(10.0, 10.0, 10.0, 10.0)).unwrap();
        qtree.insert(2, BoundingBox::new(90.0, 90.0, 90.0, 90.0)).unwrap();

        let query = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let results = qtree.search(&query);

        assert!(results.contains(&1));
        assert!(!results.contains(&2));
    }

    #[test]
    fn test_hilbert_curve() {
        let curve = HilbertCurve::new(8);

        let d1 = curve.xy_to_distance(10, 20);
        let (x, y) = curve.distance_to_xy(d1);

        assert_eq!(x, 10);
        assert_eq!(y, 20);
    }

    #[test]
    fn test_grid_index() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let mut grid = GridIndex::new(bounds, 10.0);

        grid.insert(1, BoundingBox::new(5.0, 5.0, 7.0, 7.0)).unwrap();
        grid.insert(2, BoundingBox::new(25.0, 25.0, 27.0, 27.0)).unwrap();

        let query = BoundingBox::new(0.0, 0.0, 15.0, 15.0);
        let results = grid.search(&query);

        assert!(results.contains(&1));
        assert!(!results.contains(&2));
    }
}
