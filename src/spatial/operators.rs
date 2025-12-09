// Spatial Operators
//
// Implements Oracle Spatial-compatible geometric operations:
// - Topological relationships (DE-9IM model)
// - Distance calculations
// - Buffer operations
// - Set operations (union, intersection, difference)
// - Geometric transformations
// - Simplification algorithms

use crate::error::DbError;
use crate::error::Result;
use crate::spatial::geometry::{
    Coordinate, Geometry, LinearRing, LineString, Point, Polygon,
};

// Spatial relationship types based on DE-9IM (Dimensionally Extended 9-Intersection Model)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialRelation {
    Disjoint,
    Touches,
    Contains,
    Within,
    Equals,
    Intersects,
    Overlaps,
    Crosses,
    Covers,
    CoveredBy,
}

// Topological operators
pub struct TopologicalOps;

impl TopologicalOps {
    // Check if geometry A contains geometry B
    pub fn contains(a: &Geometry, b: &Geometry) -> Result<bool> {
        match (a, b) {
            (Geometry::Polygon(poly), Geometry::Point(point)) => {
                Ok(Self::polygon_contains_point(poly, &point.coord))
            }
            (Geometry::Polygon(poly_a), Geometry::Polygon(poly_b)) => {
                Ok(Self::polygon_contains_polygon(poly_a, poly_b))
            }
            _ => Ok(false),
        }
    }

    // Check if geometry A is within geometry B
    pub fn within(a: &Geometry, b: &Geometry) -> Result<bool> {
        Self::contains(b, a)
    }

    // Check if geometries intersect
    pub fn intersects(a: &Geometry, b: &Geometry) -> Result<bool> {
        // Quick bbox check first
        if let (Some(bbox_a), Some(bbox_b)) = (a.bbox(), b.bbox()) {
            if !bbox_a.intersects(&bbox_b) {
                return Ok(false);
            }
        }

        match (a, b) {
            (Geometry::LineString(ls_a), Geometry::LineString(ls_b)) => {
                Ok(Self::linestring_intersects(ls_a, ls_b))
            }
            (Geometry::Polygon(poly), Geometry::Point(point)) => {
                Ok(Self::polygon_contains_point(poly, &point.coord))
            }
            (Geometry::Polygon(poly_a), Geometry::Polygon(poly_b)) => {
                Ok(Self::polygon_intersects(poly_a, poly_b))
            }
            _ => Ok(false),
        }
    }

    // Check if geometries touch (share boundary but not interior)
    pub fn touches(a: &Geometry, b: &Geometry) -> Result<bool> {
        match (a, b) {
            (Geometry::Polygon(poly_a), Geometry::Polygon(poly_b)) => {
                // Simplified - full implementation would check boundary intersection
                Ok(Self::polygon_boundaries_touch(poly_a, poly_b))
            }
            _ => Ok(false),
        }
    }

    // Check if geometries overlap (share some but not all area)
    pub fn overlaps(a: &Geometry, b: &Geometry) -> Result<bool> {
        match (a, b) {
            (Geometry::Polygon(poly_a), Geometry::Polygon(poly_b)) => {
                let intersects = Self::polygon_intersects_polygon(poly_a, poly_b);
                let a_contains_b = Self::polygon_contains_polygon(poly_a, poly_b);
                let b_contains_a = Self::polygon_contains_polygon(poly_b, poly_a);

                Ok(intersects && !a_contains_b && !b_contains_a)
            }
            _ => Ok(false),
        }
    }

    // Check if geometries are equal
    pub fn equals(a: &Geometry, b: &Geometry) -> Result<bool> {
        Ok(a == b)
    }

    // Point-in-polygon test using ray casting algorithm
    pub fn polygon_contains_point(polygon: &Polygon, point: &Coordinate) -> bool {
        if !Self::ring_contains_point(&polygon.exterior, point) {
            return false;
        }

        // Check if point is in any hole
        for hole in &polygon.interiors {
            if Self::ring_contains_point(hole, point) {
                return false;
            }
        }

        true
    }

    // Ring contains point (ray casting)
    fn ring_contains_point(ring: &LinearRing, point: &Coordinate) -> bool {
        let mut inside = false;
        let coords = &ring.coords;

        for i in 0..coords.len() - 1 {
            let xi = coords[i].x;
            let yi = coords[i].y;
            let xj = coords[i + 1].x;
            let yj = coords[i + 1].y;

            let intersect = ((yi > point.y) != (yj > point.y))
                && (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi);

            if intersect {
                inside = !inside;
            }
        }

        inside
    }

    // Polygon contains polygon (all vertices and no edge crossings)
    fn polygon_contains_polygon(outer: &Polygon, inner: &Polygon) -> bool {
        // Check if all vertices of inner polygon are inside outer
        for coord in &inner.exterior.coords {
            if !Self::polygon_contains_point(outer, coord) {
                return false;
            }
        }

        // In full implementation, would also check edge intersections
        true
    }

    // Polygon intersects polygon
    fn polygon_intersects_polygon(poly_a: &Polygon, poly_b: &Polygon) -> bool {
        // Check if any vertex of one is inside the other
        for coord in &poly_a.exterior.coords {
            if Self::polygon_contains_point(poly_b, coord) {
                return true;
            }
        }

        for coord in &poly_b.exterior.coords {
            if Self::polygon_contains_point(poly_a, coord) {
                return true;
            }
        }

        // Check edge intersections
        Self::rings_intersect(&poly_a.exterior, &poly_b.exterior)
    }

    // Check if polygon boundaries touch
    fn polygon_boundaries_touch(poly_a: &Polygon, poly_b: &Polygon) -> bool {
        Self::rings_intersect(&poly_a.exterior, &poly_b.exterior)
    }

    // Check if two rings intersect
    fn rings_intersect(ring_a: &LinearRing, ring_b: &LinearRing) -> bool {
        for i in 0..ring_a.coords.len() - 1 {
            for j in 0..ring_b.coords.len() - 1 {
                let a1 = &ring_a.coords[i];
                let a2 = &ring_a.coords[i + 1];
                let b1 = &ring_b.coords[j];
                let b2 = &ring_b.coords[j + 1];

                if Self::segments_intersect(a1, a2, b1, b2) {
                    return true;
                }
            }
        }
        false
    }

    // Check if two line segments intersect
    fn segments_intersect(
        a1: &Coordinate,
        a2: &Coordinate,
        b1: &Coordinate,
        b2: &Coordinate,
    ) -> bool {
        let d1 = Self::cross_product_sign(b1, b2, a1);
        let d2 = Self::cross_product_sign(b1, b2, a2);
        let d3 = Self::cross_product_sign(a1, a2, b1);
        let d4 = Self::cross_product_sign(a1, a2, b2);

        if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
            return true;
        }

        // Check for collinear cases
        if d1 == 0.0 && Self::on_segment(b1, a1, b2) {
            return true;
        }
        if d2 == 0.0 && Self::on_segment(b1, a2, b2) {
            return true;
        }
        if d3 == 0.0 && Self::on_segment(a1, b1, a2) {
            return true;
        }
        if d4 == 0.0 && Self::on_segment(a1, b2, a2) {
            return true;
        }

        false
    }

    fn cross_product_sign(p1: &Coordinate, p2: &Coordinate, p3: &Coordinate) -> f64 {
        (p2.y - p1.y) * (p3.x - p2.x) - (p2.x - p1.x) * (p3.y - p2.y)
    }

    fn on_segment(p: &Coordinate, q: &Coordinate, r: &Coordinate) -> bool {
        q.x <= p.x.max(r.x)
            && q.x >= p.x.min(r.x)
            && q.y <= p.y.max(r.y)
            && q.y >= p.y.min(r.y)
    }

    // LineString intersects LineString
    fn linestring_intersects_linestring(ls_a: &LineString, ls_b: &LineString) -> bool {
        for i in 0..ls_a.coords.len() - 1 {
            for j in 0..ls_b.coords.len() - 1 {
                let a1 = &ls_a.coords[i];
                let a2 = &ls_a.coords[i + 1];
                let b1 = &ls_b.coords[j];
                let b2 = &ls_b.coords[j + 1];

                if Self::segments_intersect(a1, a2, b1, b2) {
                    return true;
                }
            }
        }
        false
    }

    fn polygon_intersects(p0: &Polygon, p1: &Polygon) -> bool {
        todo!()
    }

    fn linestring_intersects(p0: &LineString, p1: &LineString) -> bool {
        todo!()
    }
}

// Distance calculations
pub struct DistanceOps;

impl DistanceOps {
    // Calculate distance between two geometries
    pub fn distance(a: &Geometry, b: &Geometry) -> Result<f64> {
        match (a, b) {
            (Geometry::Point(p1), Geometry::Point(p2)) => {
                Ok(Self::point_distance(&p1.coord, &p2.coord))
            }
            (Geometry::Point(point), Geometry::LineString(ls)) => {
                Ok(Self::point_linestring_distance(&point.coord, ls))
            }
            (Geometry::Point(point), Geometry::Polygon(poly)) => {
                Ok(Self::point_polygon_distance(&point.coord, poly))
            }
            _ => Err(DbError::NotImplemented(
                "Distance calculation for this geometry pair".to_string(),
            )),
        }
    }

    // Point to line segment distance
    fn point_to_segment_distance(
        point: &Coordinate,
        seg_start: &Coordinate,
        seg_end: &Coordinate,
    ) -> f64 {
        let dx = seg_end.x - seg_start.x;
        let dy = seg_end.y - seg_start.y;

        if dx == 0.0 && dy == 0.0 {
            return point.distance_2d(seg_start);
        }

        let t = ((point.x - seg_start.x) * dx + (point.y - seg_start.y) * dy)
            / (dx * dx + dy * dy);

        let t = t.max(0.0).min(1.0);

        let proj_x = seg_start.x + t * dx;
        let proj_y = seg_start.y + t * dy;
        let proj = Coordinate::new(proj_x, proj_y);

        point.distance_2d(&proj)
    }

    // Point to linestring distance
    fn point_to_linestring_distance(point: &Coordinate, linestring: &LineString) -> f64 {
        let mut min_dist = f64::INFINITY;

        for i in 0..linestring.coords.len() - 1 {
            let dist = Self::point_to_segment_distance(
                point,
                &linestring.coords[i],
                &linestring.coords[i + 1],
            );
            min_dist = min_dist.min(dist);
        }

        min_dist
    }

    // Point to polygon distance
    fn point_to_polygon_distance(point: &Coordinate, polygon: &Polygon) -> f64 {
        if TopologicalOps::polygon_contains_point(polygon, point) {
            return 0.0;
        }

        let mut min_dist = f64::INFINITY;

        // Distance to exterior ring
        for i in 0..polygon.exterior.coords.len() - 1 {
            let dist = Self::point_to_segment_distance(
                point,
                &polygon.exterior.coords[i],
                &polygon.exterior.coords[i + 1],
            );
            min_dist = min_dist.min(dist);
        }

        min_dist
    }

    fn point_polygon_distance(p0: &Coordinate, p1: &Polygon) -> f64 {
        todo!()
    }

    fn point_linestring_distance(p0: &Coordinate, p1: &LineString) -> f64 {
        todo!()
    }

    fn point_distance(p0: &Coordinate, p1: &Coordinate) -> f64 {
        todo!()
    }
}

// Buffer operations
pub struct BufferOps;

impl BufferOps {
    // Create a buffer around a geometry
    pub fn buffer(geom: &Geometry, distance: f64) -> Result<Geometry> {
        match geom {
            Geometry::Point(point) => Self::buffer_point(point, distance),
            Geometry::LineString(ls) => Self::buffer_linestring(ls, distance),
            Geometry::Polygon(poly) => Self::buffer_polygon(poly, distance),
            _ => Err(DbError::NotImplemented("Buffer for this geometry type".to_string())),
        }
    }

    // Buffer a point (creates a circle approximation)
    fn buffer_point(point: &Point, distance: f64) -> Result<Geometry> {
        let segments = 32;
        let mut coords = Vec::with_capacity(segments + 1);

        for i in 0..=segments {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (segments as f64);
            let x = point.coord.x + distance * angle.cos();
            let y = point.coord.y + distance * angle.sin();
            coords.push(Coordinate::new(x, y));
        }

        let ring = LinearRing::new(coords)?;
        Ok(Geometry::Polygon(Polygon::new(ring, vec![])))
    }

    // Buffer a linestring
    fn buffer_linestring(linestring: &LineString, distance: f64) -> Result<Geometry> {
        // Simplified implementation - creates polygon around linestring
        let mut left_coords = Vec::new();
        let mut right_coords = Vec::new();

        for i in 0..linestring.coords.len() - 1 {
            let p1 = &linestring.coords[i];
            let p2 = &linestring.coords[i + 1];

            let dx = p2.x - p1.x;
            let dy = p2.y - p1.y;
            let len = (dx * dx + dy * dy).sqrt();

            let nx = -dy / len * distance;
            let ny = dx / len * distance;

            left_coords.push(Coordinate::new(p1.x + nx, p1.y + ny));
            right_coords.push(Coordinate::new(p1.x - nx, p1.y - ny));
        }

        // Add last point
        let last = linestring.coords.last().unwrap();
        let prev = &linestring.coords[linestring.coords.len() - 2];

        let dx = last.x - prev.x;
        let dy = last.y - prev.y;
        let len = (dx * dx + dy * dy).sqrt();

        let nx = -dy / len * distance;
        let ny = dx / len * distance;

        left_coords.push(Coordinate::new(last.x + nx, last.y + ny));
        right_coords.push(Coordinate::new(last.x - nx, last.y - ny));

        // Combine into polygon
        right_coords.reverse();
        left_coords.extend(right_coords);
        left_coords.push(left_coords[0]); // Close the ring

        let ring = LinearRing::new(left_coords)?;
        Ok(Geometry::Polygon(Polygon::new(ring, vec![])))
    }

    // Buffer a polygon (expand or shrink)
    fn buffer_polygon(polygon: &Polygon, distance: f64) -> Result<Geometry> {
        // Simplified - offset each edge
        let mut new_coords = Vec::new();

        for i in 0..polygon.exterior.coords.len() - 1 {
            let p1 = &polygon.exterior.coords[i];
            let p2 = &polygon.exterior.coords[i + 1];

            let dx = p2.x - p1.x;
            let dy = p2.y - p1.y;
            let len = (dx * dx + dy * dy).sqrt();

            let nx = -dy / len * distance;
            let ny = dx / len * distance;

            new_coords.push(Coordinate::new(p1.x + nx, p1.y + ny));
        }

        new_coords.push(new_coords[0]);

        let ring = LinearRing::new(new_coords)?;
        Ok(Geometry::Polygon(Polygon::new(ring, vec![])))
    }
}

// Convex hull computation
pub struct ConvexHullOps;

impl ConvexHullOps {
    // Compute convex hull using Graham scan
    pub fn convex_hull(coords: &[Coordinate]) -> Result<Geometry> {
        if coords.len() < 3 {
            return Err(DbError::InvalidInput(
                "Need at least 3 points for convex hull".to_string(),
            ));
        }

        let mut points = coords.to_vec();

        // Find lowest point (and leftmost if tied)
        let mut lowest_idx = 0;
        for i in 1..points.len() {
            if points[i].y < points[lowest_idx].y
                || (points[i].y == points[lowest_idx].y && points[i].x < points[lowest_idx].x)
            {
                lowest_idx = i;
            }
        }
        points.swap(0, lowest_idx);

        let pivot = points[0];

        // Sort by polar angle
        points[1..].sort_by(|a, b| {
            let angle_a = (a.y - pivot.y).atan2(a.x - pivot.x);
            let angle_b = (b.y - pivot.y).atan2(b.x - pivot.x);
            angle_a.partial_cmp(&angle_b).unwrap()
        });

        // Graham scan
        let mut hull = vec![points[0], points[1]];

        for i in 2..points.len() {
            while hull.len() >= 2 {
                let len = hull.len();
                let ccw = Self::ccw(&hull[len - 2], &hull[len - 1], &points[i]);

                if ccw > 0.0 {
                    break;
                }
                hull.pop();
            }
            hull.push(points[i]);
        }

        // Close the polygon
        hull.push(hull[0]);

        let ring = LinearRing::new(hull)?;
        Ok(Geometry::Polygon(Polygon::new(ring, vec![])))
    }

    fn ccw(p1: &Coordinate, p2: &Coordinate, p3: &Coordinate) -> f64 {
        (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x)
    }
}

// Set operations (union, intersection, difference)
pub struct SetOps;

impl SetOps {
    // Union of two polygons (simplified)
    pub fn union(a: &Geometry, b: &Geometry) -> Result<Geometry> {
        match (a, b) {
            (Geometry::Polygon(poly_a), Geometry::Polygon(poly_b)) => {
                Self::polygon_union(poly_a, poly_b)
            }
            _ => Err(DbError::NotImplemented("Union for this geometry pair".to_string())),
        }
    }

    // Intersection of two polygons
    pub fn intersection(a: &Geometry, b: &Geometry) -> Result<Geometry> {
        match (a, b) {
            (Geometry::Polygon(poly_a), Geometry::Polygon(poly_b)) => {
                Self::polygon_intersection(poly_a, poly_b)
            }
            _ => Err(DbError::NotImplemented(
                "Intersection for this geometry pair".to_string(),
            )),
        }
    }

    // Difference (A - B)
    pub fn difference(a: &Geometry, b: &Geometry) -> Result<Geometry> {
        match (a, b) {
            (Geometry::Polygon(poly_a), Geometry::Polygon(poly_b)) => {
                Self::polygon_difference(poly_a, poly_b)
            }
            _ => Err(DbError::NotImplemented(
                "Difference for this geometry pair".to_string(),
            )),
        }
    }

    // Simplified polygon union (uses vertex merging)
    fn polygon_union(poly_a: &Polygon, poly_b: &Polygon) -> Result<Geometry> {
        // Simplified implementation - combines all vertices and computes convex hull
        let mut all_coords = poly_a.exterior.coords.clone();
        all_coords.extend(poly_b.exterior.coords.iter().cloned());

        ConvexHullOps::convex_hull(&all_coords)
    }

    // Simplified polygon intersection
    fn polygon_intersection(poly_a: &Polygon, poly_b: &Polygon) -> Result<Geometry> {
        // Collect vertices of poly_a that are inside poly_b
        let mut intersection_coords = Vec::new();

        for coord in &poly_a.exterior.coords {
            if TopologicalOps::polygon_contains_point(poly_b, coord) {
                intersection_coords.push(*coord);
            }
        }

        // Collect vertices of poly_b that are inside poly_a
        for coord in &poly_b.exterior.coords {
            if TopologicalOps::polygon_contains_point(poly_a, coord) {
                intersection_coords.push(*coord);
            }
        }

        if intersection_coords.len() < 3 {
            return Err(DbError::InvalidInput("No intersection found".to_string()));
        }

        ConvexHullOps::convex_hull(&intersection_coords)
    }

    // Simplified polygon difference
    fn polygon_difference(poly_a: &Polygon, poly_b: &Polygon) -> Result<Geometry> {
        // Simplified - returns vertices of A not in B
        let mut diff_coords: Vec<Coordinate> = poly_a
            .exterior
            .coords
            .iter()
            .filter(|coord| !TopologicalOps::polygon_contains_point(poly_b, coord))
            .copied()
            .collect();

        if diff_coords.len() < 3 {
            return Err(DbError::InvalidInput("Difference results in empty geometry".to_string()));
        }

        diff_coords.push(diff_coords[0]);

        let ring = LinearRing::new(diff_coords)?;
        Ok(Geometry::Polygon(Polygon::new(ring, vec![])))
    }
}

// Simplification algorithms
pub struct SimplificationOps;

impl SimplificationOps {
    // Douglas-Peucker line simplification
    pub fn douglas_peucker(linestring: &LineString, tolerance: f64) -> Result<LineString> {
        let simplified = Self::dp_recursive(&linestring.coords, tolerance);
        LineString::new(simplified)
    }

    fn dp_recursive(coords: &[Coordinate], tolerance: f64) -> Vec<Coordinate> {
        if coords.len() < 3 {
            return coords.to_vec();
        }

        let first = &coords[0];
        let last = &coords[coords.len() - 1];

        // Find point with maximum distance
        let mut max_dist = 0.0;
        let mut max_idx = 0;

        for i in 1..coords.len() - 1 {
            let dist = DistanceOps::point_to_segment_distance(&coords[i], first, last);
            if dist > max_dist {
                max_dist = dist;
                max_idx = i;
            }
        }

        if max_dist > tolerance {
            // Split and recurse
            let left = Self::dp_recursive(&coords[0..=max_idx], tolerance);
            let right = Self::dp_recursive(&coords[max_idx..], tolerance);

            let mut result = left;
            result.extend(&right[1..]);
            result
        } else {
            // All points are within tolerance, keep only endpoints
            vec![*first, *last]
        }
    }

    // Visvalingam-Whyatt simplification (area-based)
    pub fn visvalingam_whyatt(linestring: &LineString, tolerance: f64) -> Result<LineString> {
        let mut coords = linestring.coords.clone();

        while coords.len() > 2 {
            let mut min_area = f64::INFINITY;
            let mut min_idx = 0;

            // Find point with minimum area
            for i in 1..coords.len() - 1 {
                let area = Self::triangle_area(&coords[i - 1], &coords[i], &coords[i + 1]);
                if area < min_area {
                    min_area = area;
                    min_idx = i;
                }
            }

            if min_area > tolerance {
                break;
            }

            coords.remove(min_idx);
        }

        LineString::new(coords)
    }

    fn triangle_area(p1: &Coordinate, p2: &Coordinate, p3: &Coordinate) -> f64 {
        ((p2.x - p1.x) * (p3.y - p1.y) - (p3.x - p1.x) * (p2.y - p1.y)).abs() / 2.0
    }
}

// Geometric transformations
pub struct TransformOps;

impl TransformOps {
    // Translate geometry by offset
    pub fn translate(geom: &Geometry, dx: f64, dy: f64) -> Geometry {
        match geom {
            Geometry::Point(p) => {
                let new_coord = Coordinate::new(p.coord.x + dx, p.coord.y + dy);
                Geometry::Point(Point::new(new_coord))
            }
            Geometry::LineString(ls) => {
                let new_coords: Vec<Coordinate> = ls
                    .coords
                    .iter()
                    .map(|c| Coordinate::new(c.x + dx, c.y + dy))
                    .collect();
                Geometry::LineString(LineString::new(new_coords).unwrap())
            }
            Geometry::Polygon(poly) => {
                let new_exterior: Vec<Coordinate> = poly
                    .exterior
                    .coords
                    .iter()
                    .map(|c| Coordinate::new(c.x + dx, c.y + dy))
                    .collect();
                let ring = LinearRing::new(new_exterior).unwrap();
                Geometry::Polygon(Polygon::new(ring, vec![]))
            }
            _ => geom.clone(),
        }
    }

    // Scale geometry by factors
    pub fn scale(geom: &Geometry, sx: f64, sy: f64) -> Geometry {
        match geom {
            Geometry::Point(p) => {
                let new_coord = Coordinate::new(p.coord.x * sx, p.coord.y * sy);
                Geometry::Point(Point::new(new_coord))
            }
            Geometry::LineString(ls) => {
                let new_coords: Vec<Coordinate> = ls
                    .coords
                    .iter()
                    .map(|c| Coordinate::new(c.x * sx, c.y * sy))
                    .collect();
                Geometry::LineString(LineString::new(new_coords).unwrap())
            }
            Geometry::Polygon(poly) => {
                let new_exterior: Vec<Coordinate> = poly
                    .exterior
                    .coords
                    .iter()
                    .map(|c| Coordinate::new(c.x * sx, c.y * sy))
                    .collect();
                let ring = LinearRing::new(new_exterior).unwrap();
                Geometry::Polygon(Polygon::new(ring, vec![]))
            }
            _ => geom.clone(),
        }
    }

    // Rotate geometry around origin
    pub fn rotate(geom: &Geometry, angle_rad: f64) -> Geometry {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        match geom {
            Geometry::Point(p) => {
                let x = p.coord.x * cos_a - p.coord.y * sin_a;
                let y = p.coord.x * sin_a + p.coord.y * cos_a;
                Geometry::Point(Point::new(Coordinate::new(x, y)))
            }
            Geometry::LineString(ls) => {
                let new_coords: Vec<Coordinate> = ls
                    .coords
                    .iter()
                    .map(|c| {
                        let x = c.x * cos_a - c.y * sin_a;
                        let y = c.x * sin_a + c.y * cos_a;
                        Coordinate::new(x, y)
                    })
                    .collect();
                Geometry::LineString(LineString::new(new_coords).unwrap())
            }
            Geometry::Polygon(poly) => {
                let new_exterior: Vec<Coordinate> = poly
                    .exterior
                    .coords
                    .iter()
                    .map(|c| {
                        let x = c.x * cos_a - c.y * sin_a;
                        let y = c.x * sin_a + c.y * cos_a;
                        Coordinate::new(x, y)
                    })
                    .collect();
                let ring = LinearRing::new(new_exterior).unwrap();
                Geometry::Polygon(Polygon::new(ring, vec![]))
            }
            _ => geom.clone(),
        }
    }

    // Centroid calculation
    pub fn centroid(geom: &Geometry) -> Result<Point> {
        match geom {
            Geometry::Point(p) => Ok(p.clone()),
            Geometry::LineString(ls) => Ok(Self::linestring_centroid(ls)),
            Geometry::Polygon(poly) => Ok(Self::polygon_centroid(poly)),
            _ => Err(DbError::NotImplemented(
                "Centroid for this geometry type".to_string(),
            )),
        }
    }

    fn linestring_centroid(ls: &LineString) -> Point {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;

        for coord in &ls.coords {
            sum_x += coord.x;
            sum_y += coord.y;
        }

        let n = ls.coords.len() as f64;
        Point::new(Coordinate::new(sum_x / n, sum_y / n))
    }

    fn polygon_centroid(poly: &Polygon) -> Point {
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut area = 0.0;

        let coords = &poly.exterior.coords;
        for i in 0..coords.len() - 1 {
            let cross = coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
            area += cross;
            cx += (coords[i].x + coords[i + 1].x) * cross;
            cy += (coords[i].y + coords[i + 1].y) * cross;
        }

        area /= 2.0;
        cx /= 6.0 * area;
        cy /= 6.0 * area;

        Point::new(Coordinate::new(cx, cy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_in_polygon() {
        let coords = vec![
            Coordinate::new(0.0, 0.0),
            Coordinate::new(4.0, 0.0),
            Coordinate::new(4.0, 4.0),
            Coordinate::new(0.0, 4.0),
            Coordinate::new(0.0, 0.0),
        ];
        let ring = LinearRing::new(coords).unwrap();
        let poly = Polygon::new(ring, vec![]);

        let inside = Coordinate::new(2.0, 2.0);
        let outside = Coordinate::new(5.0, 5.0);

        assert!(TopologicalOps::polygon_contains_point(&poly, &inside));
        assert!(!TopologicalOps::polygon_contains_point(&poly, &outside));
    }

    #[test]
    fn test_distance() {
        let p1 = Point::new(Coordinate::new(0.0, 0.0));
        let p2 = Point::new(Coordinate::new(3.0, 4.0));

        let dist = DistanceOps::distance(&Geometry::Point(p1), &Geometry::Point(p2)).unwrap();
        assert_eq!(dist, 5.0);
    }

    #[test]
    fn test_convex_hull() {
        let coords = vec![
            Coordinate::new(0.0, 0.0),
            Coordinate::new(1.0, 1.0),
            Coordinate::new(2.0, 0.0),
            Coordinate::new(1.5, 0.5),
        ];

        let hull = ConvexHullOps::convex_hull(&coords).unwrap();
        match hull {
            Geometry::Polygon(poly) => {
                assert!(poly.exterior.coords.len() >= 3);
            }
            _ => panic!("Expected polygon"),
        }
    }

    #[test]
    fn test_douglas_peucker() {
        let coords = vec![
            Coordinate::new(0.0, 0.0),
            Coordinate::new(1.0, 0.1),
            Coordinate::new(2.0, -0.1),
            Coordinate::new(3.0, 5.0),
        ];

        let ls = LineString::new(coords).unwrap();
        let simplified = SimplificationOps::douglas_peucker(&ls, 0.5).unwrap();

        assert!(simplified.coords.len() <= ls.coords.len());
    }
}
