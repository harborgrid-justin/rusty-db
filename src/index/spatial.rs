/// Spatial Index Implementation (R-Tree)
///
/// This module provides spatial indexing for:
/// - 2D and 3D bounding box queries
/// - Nearest neighbor search
/// - Spatial joins
/// - Point, line, and polygon indexing
///
/// Uses R-tree data structure for efficient spatial queries

use crate::Result;
use crate::error::DbError;
use parking_lot::RwLock;
use std::sync::Arc;
use std::cmp::Ordering;

/// R-Tree Spatial Index
pub struct RTree<T: Clone> {
    root: Arc<RwLock<Option<NodeRef<T>>>>,
    max_entries: usize,
    min_entries: usize,
}

impl<T: Clone> Clone for RTree<T> {
    fn clone(&self) -> Self {
        Self {
            root: Arc::clone(&self.root),
            max_entries: self.max_entries,
            min_entries: self.min_entries,
        }
    }
}

impl<T: Clone> RTree<T> {
    /// Create a new R-tree with default parameters
    pub fn new() -> Self {
        Self::with_capacity(8)
    }

    /// Create a new R-tree with specified node capacity
    pub fn with_capacity(max_entries: usize) -> Self {
        let min_entries = max_entries / 2;
        Self {
            root: Arc::new(RwLock::new(None)),
            max_entries,
            min_entries,
        }
    }

    /// Insert a bounding box with associated data
    pub fn insert(&self, bbox: BoundingBox, data: T) -> Result<()> {
        let mut root_lock = self.root.write();

        if root_lock.is_none() {
            // Create initial leaf node
            let leaf = Node::new_leaf(self.max_entries);
            leaf.entries.write().push(Entry {
                bbox,
                data: EntryData::Data(data),
            });
            *root_lock = Some(Arc::new(RwLock::new(leaf)));
            return Ok(());
        }

        let root = root_lock.as_ref().unwrap().clone();
        drop(root_lock);

        // Find leaf to insert into
        let leaf = self.choose_leaf(root.clone(), &bbox)?;

        // Insert entry
        let mut leaf_lock = leaf.write();
        leaf_lock.entries.write().push(Entry {
            bbox,
            data: EntryData::Data(data),
        });

        // Check if split is needed
        if leaf_lock.entries.read().len() > self.max_entries {
            drop(leaf_lock);
            self.split_node(leaf)?;
        }

        Ok(())
    }

    /// Search for all entries intersecting a bounding box
    pub fn search(&self, query_bbox: &BoundingBox) -> Result<Vec<(BoundingBox, T)>> {
        let root_lock = self.root.read();

        match root_lock.as_ref() {
            None => Ok(Vec::new()),
            Some(root) => {
                let root_clone = root.clone();
                drop(root_lock);
                self.search_recursive(root_clone, query_bbox)
            }
        }
    }

    /// Recursive search
    fn search_recursive(
        &self,
        node_ref: NodeRef<T>,
        query_bbox: &BoundingBox,
    ) -> Result<Vec<(BoundingBox, T)>> {
        let node = node_ref.read();
        let entries = node.entries.read();
        let mut results = Vec::new();

        for entry in entries.iter() {
            if entry.bbox.intersects(query_bbox) {
                match &entry.data {
                    EntryData::Data(data) => {
                        // Leaf entry
                        results.push((entry.bbox.clone(), data.clone()));
                    }
                    EntryData::Child(child) => {
                        // Internal node - recurse
                        let child_clone = child.clone();
                        drop(entries);
                        drop(node);
                        results.extend(self.search_recursive(child_clone, query_bbox)?);
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Find k nearest neighbors to a point
    pub fn nearest_neighbors(&self, point: &Point, k: usize) -> Result<Vec<(BoundingBox, T)>> {
        let root_lock = self.root.read();

        match root_lock.as_ref() {
            None => Ok(Vec::new()),
            Some(root) => {
                let root_clone = root.clone();
                drop(root_lock);

                // Use a priority queue for best-first search
                let mut queue = std::collections::BinaryHeap::new();
                let mut results = Vec::new();

                queue.push(SearchEntry {
                    distance: 0.0,
                    node: Some(root_clone),
                    entry: None,
                });

                while let Some(search_entry) = queue.pop() {
                    if results.len() >= k {
                        break;
                    }

                    if let Some(node) = search_entry.node {
                        let node_lock = node.read();
                        let entries = node_lock.entries.read();

                        for entry in entries.iter() {
                            let dist = entry.bbox.distance_to_point(point);

                            match &entry.data {
                                EntryData::Data(data) => {
                                    queue.push(SearchEntry {
                                        distance: dist,
                                        node: None,
                                        entry: Some((entry.bbox.clone(), data.clone())),
                                    });
                                }
                                EntryData::Child(child) => {
                                    queue.push(SearchEntry {
                                        distance: dist,
                                        node: Some(child.clone()),
                                        entry: None,
                                    });
                                }
                            }
                        }
                    } else if let Some(entry) = search_entry.entry {
                        results.push(entry);
                    }
                }

                Ok(results)
            }
        }
    }

    /// Choose the best leaf node to insert a new bounding box
    fn choose_leaf(&self, node_ref: NodeRef<T>, bbox: &BoundingBox) -> Result<NodeRef<T>> {
        let node = node_ref.read();

        if node.is_leaf {
            drop(node);
            return Ok(node_ref.clone());
        }

        // Find entry requiring least enlargement
        let entries = node.entries.read();
        let mut best_entry: Option<(usize, f64)> = None;

        for (i, entry) in entries.iter().enumerate() {
            let enlargement = entry.bbox.enlargement_needed(bbox);

            match best_entry {
                None => best_entry = Some((i, enlargement)),
                Some((_, best_enlargement)) => {
                    if enlargement < best_enlargement {
                        best_entry = Some((i, enlargement));
                    }
                }
            }
        }

        let (best_idx, _) = best_entry.unwrap();
        let child = match &entries[best_idx].data {
            EntryData::Child(child) => child.clone(),
            _ => return Err(DbError::Internal("Expected child node".into())),
        };

        drop(entries);
        drop(node);

        self.choose_leaf(child, bbox)
    }

    /// Split a node when it overflows
    fn split_node(&self, node_ref: NodeRef<T>) -> Result<()> {
        let mut node = node_ref.write();
        let mut entries = node.entries.write().clone();

        // Use quadratic split algorithm
        let (group1, group2) = self.quadratic_split(&mut entries)?;

        // Update current node with first group
        *node.entries.write() = group1;

        // Create new node with second group
        let new_node = Node {
            is_leaf: node.is_leaf,
            entries: Arc::new(RwLock::new(group2)),
        };

        // If this is the root, create a new root
        let mut root_lock = self.root.write();
        if Arc::ptr_eq(&node_ref, root_lock.as_ref().unwrap()) {
            let old_root_bbox = Self::compute_mbr(&node.entries.read());
            let new_node_bbox = Self::compute_mbr(&new_node.entries.read());

            drop(node);

            let mut new_root = Node::new_internal(self.max_entries);
            new_root.entries.write().push(Entry {
                bbox: old_root_bbox,
                data: EntryData::Child(node_ref.clone()),
            });
            new_root.entries.write().push(Entry {
                bbox: new_node_bbox,
                data: EntryData::Child(Arc::new(RwLock::new(new_node))),
            });

            *root_lock = Some(Arc::new(RwLock::new(new_root)));
        }

        Ok(())
    }

    /// Quadratic split algorithm
    fn quadratic_split(&self, entries: &mut Vec<Entry<T>>) -> Result<(Vec<Entry<T>>, Vec<Entry<T>>)> {
        if entries.len() < 2 {
            return Err(DbError::Internal("Not enough entries to split".into()));
        }

        // Find the pair with maximum wasted space
        let (seed1, seed2) = self.pick_seeds(entries);

        let mut group1 = vec![entries[seed1].clone()];
        let mut group2 = vec![entries[seed2].clone()];

        // Remove seeds (in reverse order to maintain indices)
        let idx1 = seed1.max(seed2);
        let idx2 = seed1.min(seed2);
        entries.remove(idx1);
        entries.remove(idx2);

        // Distribute remaining entries
        while !entries.is_empty() {
            let entry = entries.pop().unwrap();

            // Calculate which group would expand less
            let mbr1 = Self::compute_mbr(&group1);
            let mbr2 = Self::compute_mbr(&group2);

            let expand1 = mbr1.enlargement_needed(&entry.bbox);
            let expand2 = mbr2.enlargement_needed(&entry.bbox);

            if expand1 < expand2 {
                group1.push(entry);
            } else {
                group2.push(entry);
            }
        }

        Ok((group1, group2))
    }

    /// Pick the two entries that would waste most space if grouped together
    fn pick_seeds(&self, entries: &[Entry<T>]) -> (usize, usize) {
        let mut max_waste = 0.0;
        let mut seed1 = 0;
        let mut seed2 = 1;

        for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                let combined = entries[i].bbox.union(&entries[j].bbox);
                let waste = combined.area()
                    - entries[i].bbox.area()
                    - entries[j].bbox.area();

                if waste > max_waste {
                    max_waste = waste;
                    seed1 = i;
                    seed2 = j;
                }
            }
        }

        (seed1, seed2)
    }

    /// Compute minimum bounding rectangle for a set of entries
    fn compute_mbr(entries: &[Entry<T>]) -> BoundingBox {
        if entries.is_empty() {
            return BoundingBox::empty();
        }

        let mut mbr = entries[0].bbox.clone();
        for entry in entries.iter().skip(1) {
            mbr = mbr.union(&entry.bbox);
        }
        mbr
    }
}

type NodeRef<T> = Arc<RwLock<Node<T>>>;

/// R-tree node
struct Node<T: Clone> {
    is_leaf: bool,
    entries: Arc<RwLock<Vec<Entry<T>>>>,
}

impl<T: Clone> Node<T> {
    fn new_leaf(capacity: usize) -> Self {
        Self {
            is_leaf: true,
            entries: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
        }
    }

    fn new_internal(capacity: usize) -> Self {
        Self {
            is_leaf: false,
            entries: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
        }
    }
}

/// Entry in an R-tree node
#[derive(Clone)]
struct Entry<T: Clone> {
    bbox: BoundingBox,
    data: EntryData<T>,
}

/// Entry data - either actual data (leaf) or child node (internal)
#[derive(Clone)]
enum EntryData<T: Clone> {
    Data(T),
    Child(NodeRef<T>),
}

/// 2D Bounding Box
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    /// Create a new bounding box
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// Create an empty bounding box
    pub fn empty() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    /// Create a bounding box from a point
    pub fn from_point(point: &Point) -> Self {
        Self {
            min_x: point.x,
            min_y: point.y,
            max_x: point.x,
            max_y: point.y,
        }
    }

    /// Check if this box intersects another
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        !(self.max_x < other.min_x
            || self.min_x > other.max_x
            || self.max_y < other.min_y
            || self.min_y > other.max_y)
    }

    /// Check if this box contains a point
    pub fn contains_point(&self, point: &Point) -> bool {
        point.x >= self.min_x
            && point.x <= self.max_x
            && point.y >= self.min_y
            && point.y <= self.max_y
    }

    /// Calculate union of two bounding boxes
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Calculate area of bounding box
    pub fn area(&self) -> f64 {
        (self.max_x - self.min_x) * (self.max_y - self.min_y)
    }

    /// Calculate enlargement needed to include another box
    pub fn enlargement_needed(&self, other: &BoundingBox) -> f64 {
        let union = self.union(other);
        union.area() - self.area()
    }

    /// Calculate distance from bounding box to a point
    pub fn distance_to_point(&self, point: &Point) -> f64 {
        let dx = if point.x < self.min_x {
            self.min_x - point.x
        } else if point.x > self.max_x {
            point.x - self.max_x
        } else {
            0.0
        };

        let dy = if point.y < self.min_y {
            self.min_y - point.y
        } else if point.y > self.max_y {
            point.y - self.max_y
        } else {
            0.0
        };

        (dx * dx + dy * dy).sqrt()
    }
}

/// 2D Point
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculate Euclidean distance to another point
    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Search entry for nearest neighbor search
struct SearchEntry<T: Clone> {
    distance: f64,
    node: Option<NodeRef<T>>,
    entry: Option<(BoundingBox, T)>,
}

impl<T: Clone> PartialEq for SearchEntry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl<T: Clone> Eq for SearchEntry<T> {}

impl<T: Clone> PartialOrd for SearchEntry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering for min-heap behavior
        other.distance.partial_cmp(&self.distance)
    }
}

impl<T: Clone> Ord for SearchEntry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Polygon for spatial indexing
#[derive(Debug, Clone)]
pub struct Polygon {
    pub vertices: Vec<Point>,
}

impl Polygon {
    /// Create a new polygon
    pub fn new(vertices: Vec<Point>) -> Self {
        Self { vertices }
    }

    /// Get bounding box of polygon
    pub fn bounding_box(&self) -> BoundingBox {
        if self.vertices.is_empty() {
            return BoundingBox::empty();
        }

        let mut min_x = self.vertices[0].x;
        let mut min_y = self.vertices[0].y;
        let mut max_x = self.vertices[0].x;
        let mut max_y = self.vertices[0].y;

        for vertex in &self.vertices {
            min_x = min_x.min(vertex.x);
            min_y = min_y.min(vertex.y);
            max_x = max_x.max(vertex.x);
            max_y = max_y.max(vertex.y);
        }

        BoundingBox::new(min_x, min_y, max_x, max_y)
    }

    /// Check if polygon contains a point
    pub fn contains_point(&self, point: &Point) -> bool {
        // Ray casting algorithm
        let mut inside = false;
        let n = self.vertices.len();

        for i in 0..n {
            let j = (i + 1) % n;
            let vi = &self.vertices[i];
            let vj = &self.vertices[j];

            if ((vi.y > point.y) != (vj.y > point.y))
                && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }
        }

        inside
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtree_insert_search() {
        let rtree: RTree<String> = RTree::new();

        let bbox1 = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let bbox2 = BoundingBox::new(5.0, 5.0, 15.0, 15.0);
        let bbox3 = BoundingBox::new(20.0, 20.0, 30.0, 30.0);

        rtree.insert(bbox1.clone(), "box1".to_string()).unwrap();
        rtree.insert(bbox2.clone(), "box2".to_string()).unwrap();
        rtree.insert(bbox3.clone(), "box3".to_string()).unwrap();

        // Search for overlapping boxes
        let query = BoundingBox::new(0.0, 0.0, 12.0, 12.0);
        let results = rtree.search(&query).unwrap();

        assert!(results.len() >= 2); // Should find box1 and box2
    }

    #[test]
    fn test_bounding_box_intersects() {
        let bbox1 = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let bbox2 = BoundingBox::new(5.0, 5.0, 15.0, 15.0);
        let bbox3 = BoundingBox::new(20.0, 20.0, 30.0, 30.0);

        assert!(bbox1.intersects(&bbox2));
        assert!(bbox2.intersects(&bbox1));
        assert!(!bbox1.intersects(&bbox3));
    }

    #[test]
    fn test_bounding_box_contains_point() {
        let bbox = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let point1 = Point::new(5.0, 5.0);
        let point2 = Point::new(15.0, 15.0);

        assert!(bbox.contains_point(&point1));
        assert!(!bbox.contains_point(&point2));
    }

    #[test]
    fn test_nearest_neighbors() {
        let rtree: RTree<String> = RTree::new();

        rtree.insert(BoundingBox::from_point(&Point::new(0.0, 0.0)), "p1".to_string()).unwrap();
        rtree.insert(BoundingBox::from_point(&Point::new(10.0, 10.0)), "p2".to_string()).unwrap();
        rtree.insert(BoundingBox::from_point(&Point::new(5.0, 5.0)), "p3".to_string()).unwrap();

        let query_point = Point::new(4.0, 4.0);
        let neighbors = rtree.nearest_neighbors(&query_point, 2).unwrap();

        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_polygon_contains_point() {
        let vertices = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let polygon = Polygon::new(vertices);

        assert!(polygon.contains_point(&Point::new(5.0, 5.0)));
        assert!(!polygon.contains_point(&Point::new(15.0, 15.0)));
    }

    #[test]
    fn test_polygon_bounding_box() {
        let vertices = vec![
            Point::new(1.0, 2.0),
            Point::new(5.0, 1.0),
            Point::new(6.0, 4.0),
            Point::new(2.0, 5.0),
        ];
        let polygon = Polygon::new(vertices);
        let bbox = polygon.bounding_box();

        assert_eq!(bbox.min_x, 1.0);
        assert_eq!(bbox.min_y, 1.0);
        assert_eq!(bbox.max_x, 6.0);
        assert_eq!(bbox.max_y, 5.0);
    }
}


