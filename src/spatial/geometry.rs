//! Geometry Types and Serialization
//!
//! Provides Oracle Spatial-compatible geometry types with support for:
//! - Basic 2D/3D geometries (Point, LineString, Polygon)
//! - Multi-geometries (MultiPoint, MultiLineString, MultiPolygon)
//! - Complex geometries (CircularString, CompoundCurve, GeometryCollection)
//! - Measured geometries (M coordinate)
//! - WKT/WKB serialization
//! - GeoJSON support

use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Coordinate representation with optional Z (elevation) and M (measure) values
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub m: Option<f64>,
}

impl Coordinate {
    /// Create a 2D coordinate
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, z: None, m: None }
    }

    /// Create a 3D coordinate with Z value
    pub fn new_3d(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z: Some(z), m: None }
    }

    /// Create a measured coordinate with M value
    pub fn new_m(x: f64, y: f64, m: f64) -> Self {
        Self { x, y, z: None, m: Some(m) }
    }

    /// Create a 3D measured coordinate with both Z and M values
    pub fn new_zm(x: f64, y: f64, z: f64, m: f64) -> Self {
        Self { x, y, z: Some(z), m: Some(m) }
    }

    /// Calculate Euclidean distance to another coordinate (2D)
    pub fn distance_2d(&self, other: &Coordinate) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Calculate 3D Euclidean distance
    pub fn distance_3d(&self, other: &Coordinate) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z.unwrap_or(0.0) - other.z.unwrap_or(0.0);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Bounding box for spatial indexing and queries
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub min_z: Option<f64>,
    pub max_z: Option<f64>,
}

impl BoundingBox {
    /// Create a new 2D bounding box
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
            min_z: None,
            max_z: None,
        }
    }

    /// Create from a list of coordinates
    pub fn from_coords(coords: &[Coordinate]) -> Option<Self> {
        if coords.is_empty() {
            return None;
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_z = None;
        let mut max_z = None;

        let has_z = coords.iter().any(|c| c.z.is_some());

        for coord in coords {
            min_x = min_x.min(coord.x);
            min_y = min_y.min(coord.y);
            max_x = max_x.max(coord.x);
            max_y = max_y.max(coord.y);

            if has_z {
                if let Some(z) = coord.z {
                    min_z = Some(min_z.unwrap_or(z).min(z));
                    max_z = Some(max_z.unwrap_or(z).max(z));
                }
            }
        }

        Some(Self {
            min_x,
            min_y,
            max_x,
            max_y,
            min_z,
            max_z,
        })
    }

    /// Check if this box intersects another
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }

    /// Check if this box contains a coordinate
    pub fn contains_coord(&self, coord: &Coordinate) -> bool {
        coord.x >= self.min_x
            && coord.x <= self.max_x
            && coord.y >= self.min_y
            && coord.y <= self.max_y
    }

    /// Calculate the area of the bounding box
    pub fn area(&self) -> f64 {
        (self.max_x - self.min_x) * (self.max_y - self.min_y)
    }

    /// Calculate the volume (for 3D boxes)
    pub fn volume(&self) -> Option<f64> {
        if let (Some(min_z), Some(max_z)) = (self.min_z, self.max_z) {
            Some(self.area() * (max_z - min_z))
        } else {
            None
        }
    }

    /// Expand to include another bounding box
    pub fn expand(&mut self, other: &BoundingBox) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);

        if let (Some(min_z), Some(other_min_z)) = (self.min_z, other.min_z) {
            self.min_z = Some(min_z.min(other_min_z));
        }
        if let (Some(max_z), Some(other_max_z)) = (self.max_z, other.max_z) {
            self.max_z = Some(max_z.max(other_max_z));
        }
    }

    /// Calculate the center point
    pub fn center(&self) -> Coordinate {
        let x = (self.min_x + self.max_x) / 2.0;
        let y = (self.min_y + self.max_y) / 2.0;

        if let (Some(min_z), Some(max_z)) = (self.min_z, self.max_z) {
            Coordinate::new_3d(x, y, (min_z + max_z) / 2.0)
        } else {
            Coordinate::new(x, y)
        }
    }
}

/// Main geometry enum supporting all Oracle Spatial geometry types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    Point(Point),
    LineString(LineString),
    Polygon(Polygon),
    MultiPoint(MultiPoint),
    MultiLineString(MultiLineString),
    MultiPolygon(MultiPolygon),
    GeometryCollection(GeometryCollection),
    CircularString(CircularString),
    CompoundCurve(CompoundCurve),
}

impl Geometry {
    /// Get the geometry type as a string
    pub fn geometry_type(&self) -> &str {
        match self {
            Geometry::Point(_) => "Point",
            Geometry::LineString(_) => "LineString",
            Geometry::Polygon(_) => "Polygon",
            Geometry::MultiPoint(_) => "MultiPoint",
            Geometry::MultiLineString(_) => "MultiLineString",
            Geometry::MultiPolygon(_) => "MultiPolygon",
            Geometry::GeometryCollection(_) => "GeometryCollection",
            Geometry::CircularString(_) => "CircularString",
            Geometry::CompoundCurve(_) => "CompoundCurve",
        }
    }

    /// Get the bounding box of the geometry
    pub fn bbox(&self) -> Option<BoundingBox> {
        match self {
            Geometry::Point(p) => p.bbox(),
            Geometry::LineString(ls) => ls.bbox(),
            Geometry::Polygon(p) => p.bbox(),
            Geometry::MultiPoint(mp) => mp.bbox(),
            Geometry::MultiLineString(mls) => mls.bbox(),
            Geometry::MultiPolygon(mp) => mp.bbox(),
            Geometry::GeometryCollection(gc) => gc.bbox(),
            Geometry::CircularString(cs) => cs.bbox(),
            Geometry::CompoundCurve(cc) => cc.bbox(),
        }
    }

    /// Check if the geometry is 3D
    pub fn is_3d(&self) -> bool {
        match self {
            Geometry::Point(p) => p.coord.z.is_some(),
            Geometry::LineString(ls) => ls.coords.first().and_then(|c| c.z).is_some(),
            Geometry::Polygon(p) => p.exterior.coords.first().and_then(|c| c.z).is_some(),
            _ => false,
        }
    }

    /// Check if the geometry is measured
    pub fn is_measured(&self) -> bool {
        match self {
            Geometry::Point(p) => p.coord.m.is_some(),
            Geometry::LineString(ls) => ls.coords.first().and_then(|c| c.m).is_some(),
            Geometry::Polygon(p) => p.exterior.coords.first().and_then(|c| c.m).is_some(),
            _ => false,
        }
    }

    /// Convert to WKT (Well-Known Text)
    pub fn to_wkt(&self) -> String {
        match self {
            Geometry::Point(p) => p.to_wkt(),
            Geometry::LineString(ls) => ls.to_wkt(),
            Geometry::Polygon(p) => p.to_wkt(),
            Geometry::MultiPoint(mp) => mp.to_wkt(),
            Geometry::MultiLineString(mls) => mls.to_wkt(),
            Geometry::MultiPolygon(mp) => mp.to_wkt(),
            Geometry::GeometryCollection(gc) => gc.to_wkt(),
            Geometry::CircularString(cs) => cs.to_wkt(),
            Geometry::CompoundCurve(cc) => cc.to_wkt(),
        }
    }

    /// Convert to WKB (Well-Known Binary)
    pub fn to_wkb(&self) -> Vec<u8> {
        let mut wkb = Vec::new();
        wkb.push(1); // Little endian

        match self {
            Geometry::Point(p) => p.write_wkb(&mut wkb),
            Geometry::LineString(ls) => ls.write_wkb(&mut wkb),
            Geometry::Polygon(p) => p.write_wkb(&mut wkb),
            _ => {} // Simplified for example
        }

        wkb
    }

    /// Convert to GeoJSON
    pub fn to_geojson(&self) -> serde_json::Value {
        match self {
            Geometry::Point(p) => p.to_geojson(),
            Geometry::LineString(ls) => ls.to_geojson(),
            Geometry::Polygon(p) => p.to_geojson(),
            Geometry::MultiPoint(mp) => mp.to_geojson(),
            Geometry::MultiLineString(mls) => mls.to_geojson(),
            Geometry::MultiPolygon(mp) => mp.to_geojson(),
            Geometry::GeometryCollection(gc) => gc.to_geojson(),
            _ => serde_json::json!({"type": "Unknown"}),
        }
    }
}

/// Point geometry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub coord: Coordinate,
    pub srid: Option<i32>,
}

impl Point {
    pub fn new(coord: Coordinate) -> Self {
        Self { coord, srid: None }
    }

    pub fn with_srid(coord: Coordinate, srid: i32) -> Self {
        Self { coord, srid: Some(srid) }
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        Some(BoundingBox::new(self.coord.x, self.coord.y, self.coord.x, self.coord.y))
    }

    pub fn to_wkt(&self) -> String {
        let z_suffix = if self.coord.z.is_some() { " Z" } else { "" };
        let m_suffix = if self.coord.m.is_some() { " M" } else { "" };

        let coord_str = match (self.coord.z, self.coord.m) {
            (Some(z), Some(m)) => format!("{} {} {} {}", self.coord.x, self.coord.y, z, m),
            (Some(z), None) => format!("{} {} {}", self.coord.x, self.coord.y, z),
            (None, Some(m)) => format!("{} {} {}", self.coord.x, self.coord.y, m),
            (None, None) => format!("{} {}", self.coord.x, self.coord.y),
        };

        format!("POINT{}{}({})", z_suffix, m_suffix, coord_str)
    }

    pub fn write_wkb(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&1u32.to_le_bytes()); // Point type
        buf.extend_from_slice(&self.coord.x.to_le_bytes());
        buf.extend_from_slice(&self.coord.y.to_le_bytes());
        if let Some(z) = self.coord.z {
            buf.extend_from_slice(&z.to_le_bytes());
        }
    }

    pub fn to_geojson(&self) -> serde_json::Value {
        let coordinates = if let Some(z) = self.coord.z {
            serde_json::json!([self.coord.x, self.coord.y, z])
        } else {
            serde_json::json!([self.coord.x, self.coord.y])
        };

        serde_json::json!({
            "type": "Point",
            "coordinates": coordinates
        })
    }
}

/// LineString geometry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineString {
    pub coords: Vec<Coordinate>,
    pub srid: Option<i32>,
}

impl LineString {
    pub fn new(coords: Vec<Coordinate>) -> Result<Self> {
        if coords.len() < 2 {
            return Err(DbError::InvalidInput(
                "LineString must have at least 2 coordinates".to_string(),
            ));
        }
        Ok(Self { coords, srid: None })
    }

    pub fn with_srid(coords: Vec<Coordinate>, srid: i32) -> Result<Self> {
        if coords.len() < 2 {
            return Err(DbError::InvalidInput(
                "LineString must have at least 2 coordinates".to_string(),
            ));
        }
        Ok(Self { coords, srid: Some(srid) })
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        BoundingBox::from_coords(&self.coords)
    }

    pub fn length(&self) -> f64 {
        self.coords
            .windows(2)
            .map(|w| w[0].distance_2d(&w[1]))
            .sum()
    }

    pub fn length_3d(&self) -> f64 {
        self.coords
            .windows(2)
            .map(|w| w[0].distance_3d(&w[1]))
            .sum()
    }

    pub fn to_wkt(&self) -> String {
        let coords_str: Vec<String> = self.coords.iter().map(|c| {
            match (c.z, c.m) {
                (Some(z), Some(m)) => format!("{} {} {} {}", c.x, c.y, z, m),
                (Some(z), None) => format!("{} {} {}", c.x, c.y, z),
                (None, Some(m)) => format!("{} {} {}", c.x, c.y, m),
                (None, None) => format!("{} {}", c.x, c.y),
            }
        }).collect();

        format!("LINESTRING({})", coords_str.join(", "))
    }

    pub fn write_wkb(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&2u32.to_le_bytes()); // LineString type
        buf.extend_from_slice(&(self.coords.len() as u32).to_le_bytes());
        for coord in &self.coords {
            buf.extend_from_slice(&coord.x.to_le_bytes());
            buf.extend_from_slice(&coord.y.to_le_bytes());
        }
    }

    pub fn to_geojson(&self) -> serde_json::Value {
        let coordinates: Vec<_> = self.coords.iter().map(|c| {
            if let Some(z) = c.z {
                serde_json::json!([c.x, c.y, z])
            } else {
                serde_json::json!([c.x, c.y])
            }
        }).collect();

        serde_json::json!({
            "type": "LineString",
            "coordinates": coordinates
        })
    }
}

/// LinearRing - closed LineString for polygon boundaries
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearRing {
    pub coords: Vec<Coordinate>,
}

impl LinearRing {
    pub fn new(coords: Vec<Coordinate>) -> Result<Self> {
        if coords.len() < 4 {
            return Err(DbError::InvalidInput(
                "LinearRing must have at least 4 coordinates".to_string(),
            ));
        }

        // Check if closed
        let first = coords.first().unwrap();
        let last = coords.last().unwrap();
        if first.x != last.x || first.y != last.y {
            return Err(DbError::InvalidInput(
                "LinearRing must be closed (first and last points must be equal)".to_string(),
            ));
        }

        Ok(Self { coords })
    }

    pub fn area(&self) -> f64 {
        // Shoelace formula
        let mut area = 0.0;
        for i in 0..self.coords.len() - 1 {
            area += self.coords[i].x * self.coords[i + 1].y;
            area -= self.coords[i + 1].x * self.coords[i].y;
        }
        area.abs() / 2.0
    }

    pub fn is_clockwise(&self) -> bool {
        // Calculate signed area
        let mut area = 0.0;
        for i in 0..self.coords.len() - 1 {
            area += (self.coords[i + 1].x - self.coords[i].x)
                * (self.coords[i + 1].y + self.coords[i].y);
        }
        area > 0.0
    }
}

/// Polygon geometry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub exterior: LinearRing,
    pub interiors: Vec<LinearRing>,
    pub srid: Option<i32>,
}

impl Polygon {
    pub fn new(exterior: LinearRing, interiors: Vec<LinearRing>) -> Self {
        Self {
            exterior,
            interiors,
            srid: None,
        }
    }

    pub fn with_srid(exterior: LinearRing, interiors: Vec<LinearRing>, srid: i32) -> Self {
        Self {
            exterior,
            interiors,
            srid: Some(srid),
        }
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        BoundingBox::from_coords(&self.exterior.coords)
    }

    pub fn area(&self) -> f64 {
        let exterior_area = self.exterior.area();
        let interior_area: f64 = self.interiors.iter().map(|r| r.area()).sum();
        exterior_area - interior_area
    }

    pub fn to_wkt(&self) -> String {
        let mut rings = vec![self.format_ring_wkt(&self.exterior)];
        for interior in &self.interiors {
            rings.push(self.format_ring_wkt(interior));
        }
        format!("POLYGON({})", rings.join(", "))
    }

    fn format_ring_wkt(&self, ring: &LinearRing) -> String {
        let coords_str: Vec<String> = ring.coords.iter().map(|c| {
            format!("{} {}", c.x, c.y)
        }).collect();
        format!("({})", coords_str.join(", "))
    }

    pub fn write_wkb(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&3u32.to_le_bytes()); // Polygon type
        buf.extend_from_slice(&((1 + self.interiors.len()) as u32).to_le_bytes());

        // Write exterior ring
        buf.extend_from_slice(&(self.exterior.coords.len() as u32).to_le_bytes());
        for coord in &self.exterior.coords {
            buf.extend_from_slice(&coord.x.to_le_bytes());
            buf.extend_from_slice(&coord.y.to_le_bytes());
        }

        // Write interior rings
        for interior in &self.interiors {
            buf.extend_from_slice(&(interior.coords.len() as u32).to_le_bytes());
            for coord in &interior.coords {
                buf.extend_from_slice(&coord.x.to_le_bytes());
                buf.extend_from_slice(&coord.y.to_le_bytes());
            }
        }
    }

    pub fn to_geojson(&self) -> serde_json::Value {
        let mut rings = vec![self.ring_to_geojson_coords(&self.exterior)];
        for interior in &self.interiors {
            rings.push(self.ring_to_geojson_coords(interior));
        }

        serde_json::json!({
            "type": "Polygon",
            "coordinates": rings
        })
    }

    fn ring_to_geojson_coords(&self, ring: &LinearRing) -> Vec<serde_json::Value> {
        ring.coords.iter().map(|c| {
            if let Some(z) = c.z {
                serde_json::json!([c.x, c.y, z])
            } else {
                serde_json::json!([c.x, c.y])
            }
        }).collect()
    }
}

/// MultiPoint geometry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiPoint {
    pub points: Vec<Point>,
    pub srid: Option<i32>,
}

impl MultiPoint {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points, srid: None }
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        let coords: Vec<Coordinate> = self.points.iter().map(|p| p.coord).collect();
        BoundingBox::from_coords(&coords)
    }

    pub fn to_wkt(&self) -> String {
        let points_str: Vec<String> = self.points.iter().map(|p| {
            format!("({} {})", p.coord.x, p.coord.y)
        }).collect();
        format!("MULTIPOINT({})", points_str.join(", "))
    }

    pub fn to_geojson(&self) -> serde_json::Value {
        let coordinates: Vec<_> = self.points.iter().map(|p| {
            if let Some(z) = p.coord.z {
                serde_json::json!([p.coord.x, p.coord.y, z])
            } else {
                serde_json::json!([p.coord.x, p.coord.y])
            }
        }).collect();

        serde_json::json!({
            "type": "MultiPoint",
            "coordinates": coordinates
        })
    }
}

/// MultiLineString geometry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiLineString {
    pub linestrings: Vec<LineString>,
    pub srid: Option<i32>,
}

impl MultiLineString {
    pub fn new(linestrings: Vec<LineString>) -> Self {
        Self { linestrings, srid: None }
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        let mut bbox: Option<BoundingBox> = None;
        for ls in &self.linestrings {
            if let Some(ls_bbox) = ls.bbox() {
                if let Some(ref mut b) = bbox {
                    b.expand(&ls_bbox);
                } else {
                    bbox = Some(ls_bbox);
                }
            }
        }
        bbox
    }

    pub fn total_length(&self) -> f64 {
        self.linestrings.iter().map(|ls| ls.length()).sum()
    }

    pub fn to_wkt(&self) -> String {
        let lines_str: Vec<String> = self.linestrings.iter().map(|ls| {
            let coords_str: Vec<String> = ls.coords.iter().map(|c| {
                format!("{} {}", c.x, c.y)
            }).collect();
            format!("({})", coords_str.join(", "))
        }).collect();
        format!("MULTILINESTRING({})", lines_str.join(", "))
    }

    pub fn to_geojson(&self) -> serde_json::Value {
        let coordinates: Vec<_> = self.linestrings.iter().map(|ls| {
            ls.coords.iter().map(|c| {
                if let Some(z) = c.z {
                    serde_json::json!([c.x, c.y, z])
                } else {
                    serde_json::json!([c.x, c.y])
                }
            }).collect::<Vec<_>>()
        }).collect();

        serde_json::json!({
            "type": "MultiLineString",
            "coordinates": coordinates
        })
    }
}

/// MultiPolygon geometry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiPolygon {
    pub polygons: Vec<Polygon>,
    pub srid: Option<i32>,
}

impl MultiPolygon {
    pub fn new(polygons: Vec<Polygon>) -> Self {
        Self { polygons, srid: None }
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        let mut bbox: Option<BoundingBox> = None;
        for poly in &self.polygons {
            if let Some(poly_bbox) = poly.bbox() {
                if let Some(ref mut b) = bbox {
                    b.expand(&poly_bbox);
                } else {
                    bbox = Some(poly_bbox);
                }
            }
        }
        bbox
    }

    pub fn total_area(&self) -> f64 {
        self.polygons.iter().map(|p| p.area()).sum()
    }

    pub fn to_wkt(&self) -> String {
        let polys_str: Vec<String> = self.polygons.iter().map(|p| {
            let mut rings = vec![self.format_ring_wkt(&p.exterior)];
            for interior in &p.interiors {
                rings.push(self.format_ring_wkt(interior));
            }
            format!("({})", rings.join(", "))
        }).collect();
        format!("MULTIPOLYGON({})", polys_str.join(", "))
    }

    fn format_ring_wkt(&self, ring: &LinearRing) -> String {
        let coords_str: Vec<String> = ring.coords.iter().map(|c| {
            format!("{} {}", c.x, c.y)
        }).collect();
        format!("({})", coords_str.join(", "))
    }

    pub fn to_geojson(&self) -> serde_json::Value {
        let coordinates: Vec<_> = self.polygons.iter().map(|poly| {
            let mut rings = vec![self.ring_to_geojson_coords(&poly.exterior)];
            for interior in &poly.interiors {
                rings.push(self.ring_to_geojson_coords(interior));
            }
            rings
        }).collect();

        serde_json::json!({
            "type": "MultiPolygon",
            "coordinates": coordinates
        })
    }

    fn ring_to_geojson_coords(&self, ring: &LinearRing) -> Vec<serde_json::Value> {
        ring.coords.iter().map(|c| {
            if let Some(z) = c.z {
                serde_json::json!([c.x, c.y, z])
            } else {
                serde_json::json!([c.x, c.y])
            }
        }).collect()
    }
}

/// GeometryCollection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryCollection {
    pub geometries: Vec<Geometry>,
    pub srid: Option<i32>,
}

impl GeometryCollection {
    pub fn new(geometries: Vec<Geometry>) -> Self {
        Self { geometries, srid: None }
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        let mut bbox: Option<BoundingBox> = None;
        for geom in &self.geometries {
            if let Some(geom_bbox) = geom.bbox() {
                if let Some(ref mut b) = bbox {
                    b.expand(&geom_bbox);
                } else {
                    bbox = Some(geom_bbox);
                }
            }
        }
        bbox
    }

    pub fn to_wkt(&self) -> String {
        let geoms_str: Vec<String> = self.geometries.iter().map(|g| g.to_wkt()).collect();
        format!("GEOMETRYCOLLECTION({})", geoms_str.join(", "))
    }

    pub fn to_geojson(&self) -> serde_json::Value {
        let geometries: Vec<_> = self.geometries.iter().map(|g| g.to_geojson()).collect();

        serde_json::json!({
            "type": "GeometryCollection",
            "geometries": geometries
        })
    }
}

/// CircularString - curve defined by circular arcs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircularString {
    pub coords: Vec<Coordinate>,
    pub srid: Option<i32>,
}

impl CircularString {
    pub fn new(coords: Vec<Coordinate>) -> Result<Self> {
        if coords.len() < 3 || coords.len() % 2 == 0 {
            return Err(DbError::InvalidInput(
                "CircularString must have odd number of coordinates >= 3".to_string(),
            ));
        }
        Ok(Self { coords, srid: None })
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        BoundingBox::from_coords(&self.coords)
    }

    pub fn to_wkt(&self) -> String {
        let coords_str: Vec<String> = self.coords.iter().map(|c| {
            format!("{} {}", c.x, c.y)
        }).collect();
        format!("CIRCULARSTRING({})", coords_str.join(", "))
    }

    /// Calculate arc length (approximation)
    pub fn length(&self) -> f64 {
        let mut total_length = 0.0;
        for i in (0..self.coords.len() - 2).step_by(2) {
            let p1 = &self.coords[i];
            let p2 = &self.coords[i + 1];
            let p3 = &self.coords[i + 2];

            // Calculate arc length through 3 points
            total_length += self.arc_length_3points(p1, p2, p3);
        }
        total_length
    }

    fn arc_length_3points(&self, p1: &Coordinate, p2: &Coordinate, p3: &Coordinate) -> f64 {
        // Simplified calculation - in production would use proper arc calculation
        let chord1 = p1.distance_2d(p2);
        let chord2 = p2.distance_2d(p3);
        chord1 + chord2
    }
}

/// CompoundCurve - combination of circular and linear segments
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompoundCurve {
    pub segments: Vec<CurveSegment>,
    pub srid: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CurveSegment {
    Linear(LineString),
    Circular(CircularString),
}

impl CompoundCurve {
    pub fn new(segments: Vec<CurveSegment>) -> Result<Self> {
        if segments.is_empty() {
            return Err(DbError::InvalidInput(
                "CompoundCurve must have at least one segment".to_string(),
            ));
        }
        Ok(Self { segments, srid: None })
    }

    pub fn bbox(&self) -> Option<BoundingBox> {
        let mut bbox: Option<BoundingBox> = None;
        for segment in &self.segments {
            let seg_bbox = match segment {
                CurveSegment::Linear(ls) => ls.bbox(),
                CurveSegment::Circular(cs) => cs.bbox(),
            };

            if let Some(seg_bbox) = seg_bbox {
                if let Some(ref mut b) = bbox {
                    b.expand(&seg_bbox);
                } else {
                    bbox = Some(seg_bbox);
                }
            }
        }
        bbox
    }

    pub fn to_wkt(&self) -> String {
        let segments_str: Vec<String> = self.segments.iter().map(|seg| {
            match seg {
                CurveSegment::Linear(ls) => ls.to_wkt(),
                CurveSegment::Circular(cs) => cs.to_wkt(),
            }
        }).collect();
        format!("COMPOUNDCURVE({})", segments_str.join(", "))
    }

    pub fn length(&self) -> f64 {
        self.segments.iter().map(|seg| {
            match seg {
                CurveSegment::Linear(ls) => ls.length(),
                CurveSegment::Circular(cs) => cs.length(),
            }
        }).sum()
    }
}

/// WKT Parser
pub struct WktParser;

impl WktParser {
    /// Parse WKT string to Geometry
    pub fn parse(wkt: &str) -> Result<Geometry> {
        let wkt = wkt.trim();

        if wkt.starts_with("POINT") {
            Self::parse_point(wkt)
        } else if wkt.starts_with("LINESTRING") {
            Self::parse_linestring(wkt)
        } else if wkt.starts_with("POLYGON") {
            Self::parse_polygon(wkt)
        } else if wkt.starts_with("MULTIPOINT") {
            Self::parse_multipoint(wkt)
        } else {
            Err(DbError::InvalidInput(format!("Unknown WKT type: {}", wkt)))
        }
    }

    fn parse_point(wkt: &str) -> Result<Geometry> {
        let coords_str = wkt
            .strip_prefix("POINT")
            .and_then(|s| s.trim().strip_prefix('('))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| DbError::InvalidInput("Invalid POINT WKT".to_string()))?;

        let parts: Vec<&str> = coords_str.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(DbError::InvalidInput("Invalid POINT coordinates".to_string()));
        }

        let x = parts[0].parse::<f64>()
            .map_err(|_| DbError::InvalidInput("Invalid X coordinate".to_string()))?;
        let y = parts[1].parse::<f64>()
            .map_err(|_| DbError::InvalidInput("Invalid Y coordinate".to_string()))?;

        let coord = if parts.len() >= 3 {
            let z = parts[2].parse::<f64>()
                .map_err(|_| DbError::InvalidInput("Invalid Z coordinate".to_string()))?;
            Coordinate::new_3d(x, y, z)
        } else {
            Coordinate::new(x, y)
        };

        Ok(Geometry::Point(Point::new(coord)))
    }

    fn parse_linestring(wkt: &str) -> Result<Geometry> {
        let coords_str = wkt
            .strip_prefix("LINESTRING")
            .and_then(|s| s.trim().strip_prefix('('))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| DbError::InvalidInput("Invalid LINESTRING WKT".to_string()))?;

        let coords = Self::parse_coordinate_list(coords_str)?;
        Ok(Geometry::LineString(LineString::new(coords)?))
    }

    fn parse_polygon(wkt: &str) -> Result<Geometry> {
        let rings_str = wkt
            .strip_prefix("POLYGON")
            .and_then(|s| s.trim().strip_prefix('('))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| DbError::InvalidInput("Invalid POLYGON WKT".to_string()))?;

        let rings: Vec<&str> = Self::split_rings(rings_str);
        if rings.is_empty() {
            return Err(DbError::InvalidInput("Polygon must have at least one ring".to_string()));
        }

        let exterior_coords = Self::parse_coordinate_list(rings[0])?;
        let exterior = LinearRing::new(exterior_coords)?;

        let mut interiors = Vec::new();
        for ring_str in rings.iter().skip(1) {
            let coords = Self::parse_coordinate_list(ring_str)?;
            interiors.push(LinearRing::new(coords)?);
        }

        Ok(Geometry::Polygon(Polygon::new(exterior, interiors)))
    }

    fn parse_multipoint(wkt: &str) -> Result<Geometry> {
        let points_str = wkt
            .strip_prefix("MULTIPOINT")
            .and_then(|s| s.trim().strip_prefix('('))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| DbError::InvalidInput("Invalid MULTIPOINT WKT".to_string()))?;

        let coords = Self::parse_coordinate_list(points_str)?;
        let points: Vec<Point> = coords.into_iter().map(Point::new).collect();

        Ok(Geometry::MultiPoint(MultiPoint::new(points)))
    }

    fn parse_coordinate_list(s: &str) -> Result<Vec<Coordinate>> {
        s.split(',')
            .map(|coord_str| {
                let parts: Vec<&str> = coord_str.trim().split_whitespace().collect();
                if parts.len() < 2 {
                    return Err(DbError::InvalidInput("Invalid coordinate".to_string()));
                }

                let x = parts[0].parse::<f64>()
                    .map_err(|_| DbError::InvalidInput("Invalid X".to_string()))?;
                let y = parts[1].parse::<f64>()
                    .map_err(|_| DbError::InvalidInput("Invalid Y".to_string()))?;

                Ok(if parts.len() >= 3 {
                    let z = parts[2].parse::<f64>()
                        .map_err(|_| DbError::InvalidInput("Invalid Z".to_string()))?;
                    Coordinate::new_3d(x, y, z)
                } else {
                    Coordinate::new(x, y)
                })
            })
            .collect()
    }

    fn split_rings(s: &str) -> Vec<&str> {
        let mut rings = Vec::new();
        let mut depth = 0;
        let mut start = 0;

        for (i, c) in s.chars().enumerate() {
            match c {
                '(' => {
                    if depth == 0 {
                        start = i + 1;
                    }
                    depth += 1;
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        rings.push(&s[start..i]);
                    }
                }
                _ => {}
            }
        }

        rings
    }
}

/// WKB Parser
pub struct WkbParser;

impl WkbParser {
    /// Parse WKB bytes to Geometry
    pub fn parse(wkb: &[u8]) -> Result<Geometry> {
        if wkb.is_empty() {
            return Err(DbError::InvalidInput("Empty WKB".to_string()));
        }

        let is_little_endian = wkb[0] == 1;
        let geom_type = if is_little_endian {
            u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]])
        } else {
            u32::from_be_bytes([wkb[1], wkb[2], wkb[3], wkb[4]])
        };

        match geom_type {
            1 => Self::parse_point(&wkb[5..], is_little_endian),
            2 => Self::parse_linestring(&wkb[5..], is_little_endian),
            3 => Self::parse_polygon(&wkb[5..], is_little_endian),
            _ => Err(DbError::InvalidInput(format!("Unknown WKB type: {}", geom_type))),
        }
    }

    fn parse_point(data: &[u8], is_little_endian: bool) -> Result<Geometry> {
        if data.len() < 16 {
            return Err(DbError::InvalidInput("Invalid POINT WKB".to_string()));
        }

        let x = Self::read_f64(data, 0, is_little_endian);
        let y = Self::read_f64(data, 8, is_little_endian);

        Ok(Geometry::Point(Point::new(Coordinate::new(x, y))))
    }

    fn parse_linestring(data: &[u8], is_little_endian: bool) -> Result<Geometry> {
        if data.len() < 4 {
            return Err(DbError::InvalidInput("Invalid LINESTRING WKB".to_string()));
        }

        let num_points = Self::read_u32(data, 0, is_little_endian) as usize;
        let mut coords = Vec::with_capacity(num_points);

        for i in 0..num_points {
            let offset = 4 + i * 16;
            let x = Self::read_f64(data, offset, is_little_endian);
            let y = Self::read_f64(data, offset + 8, is_little_endian);
            coords.push(Coordinate::new(x, y));
        }

        Ok(Geometry::LineString(LineString::new(coords)?))
    }

    fn parse_polygon(data: &[u8], is_little_endian: bool) -> Result<Geometry> {
        if data.len() < 4 {
            return Err(DbError::InvalidInput("Invalid POLYGON WKB".to_string()));
        }

        let num_rings = Self::read_u32(data, 0, is_little_endian) as usize;
        let mut offset = 4;

        // Parse exterior ring
        let num_points = Self::read_u32(data, offset, is_little_endian) as usize;
        offset += 4;

        let mut exterior_coords = Vec::with_capacity(num_points);
        for _ in 0..num_points {
            let x = Self::read_f64(data, offset, is_little_endian);
            let y = Self::read_f64(data, offset + 8, is_little_endian);
            exterior_coords.push(Coordinate::new(x, y));
            offset += 16;
        }

        let exterior = LinearRing::new(exterior_coords)?;
        let mut interiors = Vec::new();

        // Parse interior rings
        for _ in 1..num_rings {
            let num_points = Self::read_u32(data, offset, is_little_endian) as usize;
            offset += 4;

            let mut interior_coords = Vec::with_capacity(num_points);
            for _ in 0..num_points {
                let x = Self::read_f64(data, offset, is_little_endian);
                let y = Self::read_f64(data, offset + 8, is_little_endian);
                interior_coords.push(Coordinate::new(x, y));
                offset += 16;
            }

            interiors.push(LinearRing::new(interior_coords)?);
        }

        Ok(Geometry::Polygon(Polygon::new(exterior, interiors)))
    }

    fn read_f64(data: &[u8], offset: usize, is_little_endian: bool) -> f64 {
        let bytes = [
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ];

        if is_little_endian {
            f64::from_le_bytes(bytes)
        } else {
            f64::from_be_bytes(bytes)
        }
    }

    fn read_u32(data: &[u8], offset: usize, is_little_endian: bool) -> u32 {
        let bytes = [
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ];

        if is_little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_distance() {
        let c1 = Coordinate::new(0.0, 0.0);
        let c2 = Coordinate::new(3.0, 4.0);
        assert_eq!(c1.distance_2d(&c2), 5.0);
    }

    #[test]
    fn test_point_wkt() {
        let p = Point::new(Coordinate::new(1.0, 2.0));
        assert_eq!(p.to_wkt(), "POINT(1 2)");
    }

    #[test]
    fn test_linestring() {
        let coords = vec![
            Coordinate::new(0.0, 0.0),
            Coordinate::new(1.0, 1.0),
            Coordinate::new(2.0, 0.0),
        ];
        let ls = LineString::new(coords).unwrap();
        assert!(ls.length() > 0.0);
    }

    #[test]
    fn test_polygon_area() {
        let coords = vec![
            Coordinate::new(0.0, 0.0),
            Coordinate::new(4.0, 0.0),
            Coordinate::new(4.0, 3.0),
            Coordinate::new(0.0, 3.0),
            Coordinate::new(0.0, 0.0),
        ];
        let ring = LinearRing::new(coords).unwrap();
        let poly = Polygon::new(ring, vec![]);
        assert_eq!(poly.area(), 12.0);
    }

    #[test]
    fn test_wkt_parser() {
        let wkt = "POINT(1.5 2.5)";
        let geom = WktParser::parse(wkt).unwrap();
        match geom {
            Geometry::Point(p) => {
                assert_eq!(p.coord.x, 1.5);
                assert_eq!(p.coord.y, 2.5);
            }
            _ => panic!("Expected Point"),
        }
    }

    #[test]
    fn test_bbox_intersection() {
        let bbox1 = BoundingBox::new(0.0, 0.0, 2.0, 2.0);
        let bbox2 = BoundingBox::new(1.0, 1.0, 3.0, 3.0);
        let bbox3 = BoundingBox::new(3.0, 3.0, 5.0, 5.0);

        assert!(bbox1.intersects(&bbox2));
        assert!(!bbox1.intersects(&bbox3));
    }
}
