// Spatial Analysis
//
// Advanced spatial analytics including:
// - Nearest neighbor and k-nearest neighbors search
// - Spatial clustering (DBSCAN, K-means)
// - Voronoi diagrams
// - Delaunay triangulation
// - Spatial aggregation and statistics
// - Hot spot analysis

use std::collections::HashSet;
use crate::error::Result;
use crate::spatial::geometry::{BoundingBox, Coordinate, Point, Polygon, LinearRing};
use crate::spatial::indexes::SpatialIndex;
use crate::spatial::operators::{SetOps, ConvexHullOps, TransformOps};
use std::collections::{HashMap};

/// Nearest neighbor search results
#[derive(Debug, Clone)]
pub struct NearestNeighborResult {
    pub id: u64,
    pub distance: f64,
}

/// K-nearest neighbors searcher
pub struct KNearestNeighbors {
    index: Box<dyn SpatialIndex>,
    geometries: HashMap<u64>,
}

impl KNearestNeighbors {
    pub fn new(index: Box<dyn SpatialIndex>, geometries: HashMap<u64>) -> Self {
        Self { index, geometries }
    }

    /// Find k nearest neighbors to a point
    pub fn search(&self, point: &Coordinate, k: usize) -> Vec<NearestNeighborResult> {
        let mut candidates = Vec::new();

        // Start with a reasonable search radius
        let mut search_radius = 100.0;
        let max_radius = 10000.0;

        while candidates.len() < k && search_radius < max_radius {
            let bbox = BoundingBox::new(
                point.x - search_radius,
                point.y - search_radius,
                point.x + search_radius,
                point.y + search_radius,
            );

            let ids = self.index.search(&bbox);

            candidates.clear();
            for id in ids {
                if let Some(geom) = self.geometries.get(&id) {
                    let dist = self.geometry_distance(point, geom);
                    candidates.push(NearestNeighborResult { id, distance: dist });
                }
            }

            if candidates.len() < k {
                search_radius *= 2.0;
            }
        }

        // Sort by distance and take k nearest
        candidates.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        candidates.truncate(k);
        candidates
    }

    /// Find all neighbors within a distance
    pub fn search_radius(&self, point: &Coordinate, radius: f64) -> Vec<NearestNeighborResult> {
        let bbox = BoundingBox::new(
            point.x - radius,
            point.y - radius,
            point.x + radius,
            point.y + radius,
        );

        let ids = self.index.search(&bbox);
        let mut results = Vec::new();

        for id in ids {
            if let Some(geom) = self.geometries.get(&id) {
                let dist = self.geometry_distance(point, geom);
                if dist <= radius {
                    results.push(NearestNeighborResult { id, distance: dist });
                }
            }
        }

        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        results
    }

    fn geometry_distance(&self, point: &Coordinate, geom: &Geometry) -> f64 {
        match geom {
            Geometry::Point(p) => point.distance_2d(&p.coord),
            Geometry::LineString(ls) => {
                ls.coords
                    .iter()
                    .map(|c| point.distance_2d(c))
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(f64::INFINITY)
            }
            Geometry::Polygon(poly) => {
                poly.exterior
                    .coords
                    .iter()
                    .map(|c| point.distance_2d(c))
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(f64::INFINITY)
            }
            _ => f64::INFINITY,
        }
    }
}

/// DBSCAN clustering algorithm for spatial data
pub struct DbscanClusterer {
    epsilon: f64,
    min_points: usize,
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub id: usize,
    pub points: Vec<u64>,
    pub centroid: Coordinate,
}

impl DbscanClusterer {
    pub fn new(epsilon: f64, min_points: usize) -> Self {
        Self { epsilon, min_points }
    }

    /// Perform DBSCAN clustering on points
    pub fn cluster(&self, points: &[(u64, Coordinate)]) -> Vec<Cluster> {
        let mut visited = HashSet::new();
        let mut clusters = Vec::new();
        let mut cluster_id = 0;

        for (id, coord) in points {
            if visited.contains(id) {
                continue;
            }

            visited.insert(*id);
            let neighbors = self.region_query(points, coord);

            if neighbors.len() >= self.min_points {
                let cluster = self.expand_cluster(
                    points,
                    *id,
                    *coord,
                    neighbors,
                    &mut visited,
                    cluster_id,
                );
                clusters.push(cluster);
                cluster_id += 1;
            }
        }

        clusters
    }

    fn expand_cluster(
        &self,
        points: &[(u64, Coordinate)],
        seed_id: u64,
        seed_coord: Coordinate,
        mut neighbors: Vec<usize>,
        visited: &mut HashSet<u64>,
        cluster_id: usize,
    ) -> Cluster {
        let mut cluster_points = vec![seed_id];
        let mut i = 0;

        while i < neighbors.len() {
            let neighbor_idx = neighbors[i];
            let (neighbor_id, neighbor_coord) = points[neighbor_idx];

            if !visited.contains(&neighbor_id) {
                visited.insert(neighbor_id);

                let neighbor_neighbors = self.region_query(points, &neighbor_coord);
                if neighbor_neighbors.len() >= self.min_points {
                    neighbors.extend(neighbor_neighbors);
                }
            }

            if !cluster_points.contains(&neighbor_id) {
                cluster_points.push(neighbor_id);
            }

            i += 1;
        }

        // Calculate centroid
        let centroid = self.calculate_centroid(&cluster_points, points);

        Cluster {
            id: cluster_id,
            points: cluster_points,
            centroid,
        }
    }

    fn region_query(&self, points: &[(u64, Coordinate)], center: &Coordinate) -> Vec<usize> {
        points
            .iter()
            .enumerate()
            .filter(|(_, (_, coord))| center.distance_2d(coord) <= self.epsilon)
            .map(|(idx, _)| idx)
            .collect()
    }

    fn calculate_centroid(&self, point_ids: &[u64], all_points: &[(u64, Coordinate)]) -> Coordinate {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut count = 0;

        for (id, coord) in all_points {
            if point_ids.contains(id) {
                sum_x += coord.x;
                sum_y += coord.y;
                count += 1;
            }
        }

        if count > 0 {
            Coordinate::new(sum_x / count as f64, sum_y / count as f64)
        } else {
            Coordinate::new(0.0, 0.0)
        }
    }
}

/// K-means clustering for spatial data
pub struct KMeansClusterer {
    k: usize,
    max_iterations: usize,
}

impl KMeansClusterer {
    pub fn new(k: usize, max_iterations: usize) -> Self {
        Self { k, max_iterations }
    }

    /// Perform K-means clustering
    pub fn cluster(&self, points: &[(u64, Coordinate)]) -> Vec<Cluster> {
        if points.len() < self.k {
            return Vec::new();
        }

        // Initialize centroids using k-means++
        let mut centroids = self.initialize_centroids(points);
        let mut assignments = vec![0; points.len()];

        for _ in 0..self.max_iterations {
            let mut changed = false;

            // Assign points to nearest centroid
            for (i, (_, coord)) in points.iter().enumerate() {
                let mut min_dist = f64::INFINITY;
                let mut best_cluster = 0;

                for (cluster_id, centroid) in centroids.iter().enumerate() {
                    let dist = coord.distance_2d(centroid);
                    if dist < min_dist {
                        min_dist = dist;
                        best_cluster = cluster_id;
                    }
                }

                if assignments[i] != best_cluster {
                    assignments[i] = best_cluster;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            // Update centroids
            for cluster_id in 0..self.k {
                let cluster_points: Vec<&Coordinate> = points
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| assignments[*i] == cluster_id)
                    .map(|(_, (_, coord))| coord)
                    .collect();

                if !cluster_points.is_empty() {
                    let sum_x: f64 = cluster_points.iter().map(|c| c.x).sum();
                    let sum_y: f64 = cluster_points.iter().map(|c| c.y).sum();
                    let count = cluster_points.len() as f64;

                    centroids[cluster_id] = Coordinate::new(sum_x / count, sum_y / count);
                }
            }
        }

        // Build clusters
        let mut clusters = Vec::new();
        for cluster_id in 0..self.k {
            let cluster_points: Vec<u64> = points
                .iter()
                .enumerate()
                .filter(|(i, _)| assignments[*i] == cluster_id)
                .map(|(_, (id, _))| *id)
                .collect();

            if !cluster_points.is_empty() {
                clusters.push(Cluster {
                    id: cluster_id,
                    points: cluster_points,
                    centroid: centroids[cluster_id],
                });
            }
        }

        clusters
    }

    fn initialize_centroids(&self, points: &[(u64, Coordinate)]) -> Vec<Coordinate> {
        let mut centroids = Vec::new();

        // Choose first centroid randomly (use first point for determinism)
        centroids.push(points[0].1);

        // Choose remaining centroids using k-means++
        for _ in 1..self.k {
            let mut distances: Vec<f64> = points
                .iter()
                .map(|(_, coord)| {
                    centroids
                        .iter()
                        .map(|c| coord.distance_2d(c))
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap()
                })
                .collect();

            // Select point with maximum distance
            let max_idx = distances
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(idx, _)| idx)
                .unwrap();

            centroids.push(points[max_idx].1);
        }

        centroids
    }
}

/// Voronoi diagram computation
pub struct VoronoiDiagram {
    sites: Vec<Coordinate>,
    bounds: BoundingBox,
}

#[derive(Debug, Clone)]
pub struct VoronoiCell {
    pub site_index: usize,
    pub polygon: Polygon,
}

impl VoronoiDiagram {
    pub fn new(sites: Vec<Coordinate>, bounds: BoundingBox) -> Self {
        Self { sites, bounds }
    }

    /// Compute Voronoi diagram (simplified using Fortune's algorithm concept)
    pub fn compute(&self) -> Result<Vec<VoronoiCell>> {
        let mut cells = Vec::new();

        // Simplified implementation - creates cells by finding polygon for each site
        for (site_idx, site) in self.sites.iter().enumerate() {
            let cell_polygon = self.compute_cell(site_idx, site)?;
            cells.push(VoronoiCell {
                site_index: site_idx,
                polygon: cell_polygon,
            });
        }

        Ok(cells)
    }

    fn compute_cell(&self, site_idx: usize, site: &Coordinate) -> Result<Polygon> {
        // Simplified: create a grid of test points and keep those closer to this site
        let resolution = 20;
        let width = self.bounds.max_x - self.bounds.min_x;
        let height = self.bounds.max_y - self.bounds.min_y;

        let mut vertices = Vec::new();

        // Sample points around the boundary
        for _i in 0..resolution {
            let t = i as f64 / resolution as f64;

            // Top edge
            let test = Coordinate::new(
                self.bounds.min_x + t * width,
                self.bounds.max_y,
            );
            if self.closest_site(&test) == site_idx {
                vertices.push(test);
            }

            // Right edge
            let test = Coordinate::new(
                self.bounds.max_x,
                self.bounds.max_y - t * height,
            );
            if self.closest_site(&test) == site_idx {
                vertices.push(test);
            }

            // Bottom edge
            let test = Coordinate::new(
                self.bounds.max_x - t * width,
                self.bounds.min_y,
            );
            if self.closest_site(&test) == site_idx {
                vertices.push(test);
            }

            // Left edge
            let test = Coordinate::new(
                self.bounds.min_x,
                self.bounds.min_y + t * height,
            );
            if self.closest_site(&test) == site_idx {
                vertices.push(test);
            }
        }

        if vertices.is_empty() {
            vertices.push(*site);
        }

        // Close the polygon
        if !vertices.is_empty() {
            vertices.push(vertices[0]);
        }

        let ring = LinearRing::new(vertices)?;
        Ok(Polygon::new(ring, vec![]))
    }

    fn closest_site(&self, point: &Coordinate) -> usize {
        self.sites
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let dist_a = point.distance_2d(a);
                let dist_b = point.distance_2d(b);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }
}

/// Delaunay triangulation
pub struct DelaunayTriangulation {
    points: Vec<Coordinate>,
}

#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Coordinate; 3],
}

impl Triangle {
    pub fn new(p1: Coordinate, p2: Coordinate, p3: Coordinate) -> Self {
        Self {
            vertices: [p1, p2, p3],
        }
    }

    /// Check if a point is inside the circumcircle of this triangle
    pub fn in_circumcircle(&self, point: &Coordinate) -> bool {
        let ax = self.vertices[0].x - point.x;
        let ay = self.vertices[0].y - point.y;
        let bx = self.vertices[1].x - point.x;
        let by = self.vertices[1].y - point.y;
        let cx = self.vertices[2].x - point.x;
        let cy = self.vertices[2].y - point.y;

        let det = (ax * ax + ay * ay) * (bx * cy - cx * by)
            - (bx * bx + by * by) * (ax * cy - cx * ay)
            + (cx * cx + cy * cy) * (ax * by - bx * ay);

        det > 0.0
    }

    /// Get triangle edges
    pub fn edges(&self) -> [(Coordinate, Coordinate); 3] {
        [
            (self.vertices[0], self.vertices[1]),
            (self.vertices[1], self.vertices[2]),
            (self.vertices[2], self.vertices[0]),
        ]
    }
}

impl DelaunayTriangulation {
    pub fn new(points: Vec<Coordinate>) -> Self {
        Self { points }
    }

    /// Compute Delaunay triangulation using Bowyer-Watson algorithm
    pub fn triangulate(&self) -> Result<Vec<Triangle>> {
        if self.points.len() < 3 {
            return Err(DbError::InvalidInput(
                "Need at least 3 points for triangulation".to_string(),
            ));
        }

        // Create super-triangle that contains all points
        let super_triangle = self.create_super_triangle();
        let mut triangles = vec![super_triangle.clone()];

        // Add points one by one
        for point in &self.points {
            let mut bad_triangles = Vec::new();

            // Find triangles whose circumcircle contains the point
            for (i, triangle) in triangles.iter().enumerate() {
                if triangle.in_circumcircle(point) {
                    bad_triangles.push(i);
                }
            }

            // Find the boundary of the polygonal hole
            let mut polygon_edges = Vec::new();
            for &i in &bad_triangles {
                for edge in triangles[i].edges() {
                    polygon_edges.push(edge);
                }
            }

            // Remove bad triangles
            for &i in bad_triangles.iter().rev() {
                triangles.remove(i);
            }

            // Remove duplicate edges (internal edges)
            let mut unique_edges = Vec::new();
            for edge in polygon_edges {
                let reverse = (edge.1, edge.0);
                if !unique_edges.contains(&reverse) {
                    unique_edges.push(edge);
                } else {
                    unique_edges.retain(|&e| e != reverse);
                }
            }

            // Create new triangles from the point to each edge
            for edge in unique_edges {
                triangles.push(Triangle::new(*point, edge.0, edge.1));
            }
        }

        // Remove triangles that contain super-triangle vertices
        let super_vertices = super_triangle.vertices;
        triangles.retain(|t| {
            !t.vertices.iter().any(|v| {
                super_vertices.iter().any(|sv| {
                    (v.x - sv.x).abs() < 1e-10 && (v.y - sv.y).abs() < 1e-10
                })
            })
        });

        Ok(triangles)
    }

    fn create_super_triangle(&self) -> Triangle {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for point in &self.points {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }

        let dx = max_x - min_x;
        let dy = max_y - min_y;
        let delta_max = dx.max(dy) * 10.0;

        let p1 = Coordinate::new(min_x - delta_max, min_y - delta_max);
        let p2 = Coordinate::new(max_x + delta_max, min_y - delta_max);
        let p3 = Coordinate::new(min_x + dx / 2.0, max_y + delta_max);

        Triangle::new(p1, p2, p3)
    }
}

/// Spatial aggregation operations
pub struct SpatialAggregation;

impl SpatialAggregation {
    /// Calculate spatial extent (bounding box) of geometries
    pub fn extent(geometries: &[Geometry]) -> Option<BoundingBox> {
        if geometries.is_empty() {
            return None;
        }

        let mut bbox = geometries[0].bbox()?;

        for geom in geometries.iter().skip(1) {
            if let Some(geom_bbox) = geom.bbox() {
                bbox.expand(&geom_bbox);
            }
        }

        Some(bbox)
    }

    /// Calculate union of all geometries
    pub fn union_all(geometries: &[Geometry]) -> Result<Geometry> {
        if geometries.is_empty() {
            return Err(DbError::InvalidInput("No geometries to union".to_string()));
        }

        let mut result = geometries[0].clone();

        for geom in geometries.iter().skip(1) {
            result = SetOps::union(&result, geom)?;
        }

        Ok(result)
    }

    /// Calculate convex hull of all geometries
    pub fn convex_hull_all(geometries: &[Geometry]) -> Result<Geometry> {
        let mut all_coords = Vec::new();

        for geom in geometries {
            match geom {
                Geometry::Point(p) => all_coords.push(p.coord),
                Geometry::LineString(ls) => all_coords.extend(ls.coords.iter().copied()),
                Geometry::Polygon(poly) => all_coords.extend(poly.exterior.coords.iter().copied()),
                _ => {}
            }
        }

        if all_coords.is_empty() {
            return Err(DbError::InvalidInput("No coordinates found".to_string()));
        }

        ConvexHullOps::convex_hull(&all_coords)
    }

    /// Calculate centroid of multiple geometries
    pub fn centroid_all(geometries: &[Geometry]) -> Result<Point> {
        if geometries.is_empty() {
            return Err(DbError::InvalidInput("No geometries provided".to_string()));
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut count = 0;

        for geom in geometries {
            if let Ok(centroid) = TransformOps::centroid(geom) {
                sum_x += centroid.coord.x;
                sum_y += centroid.coord.y;
                count += 1;
            }
        }

        if count == 0 {
            return Err(DbError::InvalidInput("No valid centroids found".to_string()));
        }

        Ok(Point::new(Coordinate::new(
            sum_x / count as f64,
            sum_y / count as f64,
        )))
    }
}

/// Hot spot analysis (Getis-Ord Gi*)
pub struct HotSpotAnalysis {
    points: Vec<(u64, Coordinate, f64)>, // id, coordinate, value
    distance_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct HotSpot {
    pub id: u64,
    pub gi_star: f64,
    pub z_score: f64,
    pub p_value: f64,
    pub is_hot: bool,
    pub is_cold: bool,
}

impl HotSpotAnalysis {
    pub fn new(points: Vec<(u64, Coordinate, f64)>, distance_threshold: f64) -> Self {
        Self {
            points,
            distance_threshold,
        }
    }

    /// Compute Getis-Ord Gi* statistic for each point
    pub fn analyze(&self) -> Vec<HotSpot> {
        let mut results = Vec::new();

        // Calculate global mean and standard deviation
        let values: Vec<f64> = self.points.iter().map(|(_, _, v)| *v).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        for (i, (id, coord, _)) in self.points.iter().enumerate() {
            let gi_star = self.calculate_gi_star(i, mean, std_dev);
            let z_score = gi_star;

            // Calculate p-value (simplified using normal distribution)
            let p_value = 2.0 * (1.0 - Self::normal_cdf(z_score.abs()));

            results.push(HotSpot {
                id: *id,
                gi_star,
                z_score,
                p_value,
                is_hot: z_score > 1.96,  // 95% confidence
                is_cold: z_score < -1.96,
            });
        }

        results
    }

    fn calculate_gi_star(&self, point_idx: usize, mean: f64, std_dev: f64) -> f64 {
        let (_, point_coord, _) = &self.points[point_idx];

        let mut sum_values = 0.0;
        let mut sum_weights = 0.0;

        for (j, (_, other_coord, value)) in self.points.iter().enumerate() {
            let distance = point_coord.distance_2d(other_coord);

            if distance <= self.distance_threshold {
                let weight = 1.0; // Binary weight (can be distance-based)
                sum_values += weight * value;
                sum_weights += weight;
            }
        }

        let n = self.points.len() as f64;

        if sum_weights == 0.0 || std_dev == 0.0 {
            return 0.0;
        }

        let numerator = sum_values - mean * sum_weights;
        let denominator = std_dev * ((n * sum_weights - sum_weights.powi(2)) / (n - 1.0)).sqrt();

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    fn normal_cdf(x: f64) -> f64 {
        // Approximation of cumulative distribution function for standard normal
        0.5 * (1.0 + Self::erf(x / 2.0_f64.sqrt()))
    }

    fn erf(x: f64) -> f64 {
        // Approximation of error function
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();

        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

        sign * y
    }
}

/// Spatial autocorrelation (Moran's I)
pub struct SpatialAutocorrelation {
    points: Vec<(u64, Coordinate, f64)>,
    distance_threshold: f64,
}

impl SpatialAutocorrelation {
    pub fn new(points: Vec<(u64, Coordinate, f64)>, distance_threshold: f64) -> Self {
        Self {
            points,
            distance_threshold,
        }
    }

    /// Calculate Moran's I statistic
    pub fn morans_i(&self) -> f64 {
        let n = self.points.len() as f64;
        let values: Vec<f64> = self.points.iter().map(|(_, _, v)| *v).collect();
        let mean = values.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;
        let mut sum_weights = 0.0;

        for _i in 0..self.points.len() {
            let (_, coord_i, value_i) = &self.points[i];
            let dev_i = value_i - mean;

            for j in 0..self.points.len() {
                if i == j {
                    continue;
                }

                let (_, coord_j, value_j) = &self.points[j];
                let distance = coord_i.distance_2d(coord_j);

                if distance <= self.distance_threshold {
                    let weight = 1.0; // Binary weight
                    let dev_j = value_j - mean;

                    numerator += weight * dev_i * dev_j;
                    sum_weights += weight;
                }
            }

            denominator += dev_i * dev_i;
        }

        if sum_weights == 0.0 || denominator == 0.0 {
            return 0.0;
        }

        (n / sum_weights) * (numerator / denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbscan_clustering() {
        let points = vec![
            (1, Coordinate::new(0.0, 0.0)),
            (2, Coordinate::new(1.0, 1.0)),
            (3, Coordinate::new(0.5, 0.5)),
            (4, Coordinate::new(10.0, 10.0)),
            (5, Coordinate::new(11.0, 11.0)),
        ];

        let clusterer = DbscanClusterer::new(2.0, 2);
        let clusters = clusterer.cluster(&points);

        assert!(clusters.len() >= 1);
    }

    #[test]
    fn test_kmeans_clustering() {
        let points = vec![
            (1, Coordinate::new(0.0, 0.0)),
            (2, Coordinate::new(1.0, 1.0)),
            (3, Coordinate::new(10.0, 10.0)),
            (4, Coordinate::new(11.0, 11.0)),
        ];

        let clusterer = KMeansClusterer::new(2, 10);
        let clusters = clusterer.cluster(&points);

        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_delaunay_triangulation() {
        let points = vec![
            Coordinate::new(0.0, 0.0),
            Coordinate::new(4.0, 0.0),
            Coordinate::new(2.0, 3.0),
        ];

        let delaunay = DelaunayTriangulation::new(points);
        let triangles = delaunay.triangulate().unwrap();

        assert!(triangles.len() >= 1);
    }

    #[test]
    fn test_voronoi_diagram() {
        let sites = vec![
            Coordinate::new(0.0, 0.0),
            Coordinate::new(10.0, 0.0),
            Coordinate::new(5.0, 10.0),
        ];

        let bounds = BoundingBox::new(-5.0, -5.0, 15.0, 15.0);
        let voronoi = VoronoiDiagram::new(sites, bounds);
        let cells = voronoi.compute().unwrap();

        assert_eq!(cells.len(), 3);
    }
}
