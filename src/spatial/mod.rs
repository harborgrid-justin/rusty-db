// # Spatial Database Engine
//
// Oracle Spatial-compatible geospatial database engine for RustyDB.
//
// This module provides comprehensive spatial data management including:
// - Multiple geometry types (Point, Polygon, etc.)
// - Spatial indexing (R-tree, Quadtree, Grid)
// - Topological operators and spatial analysis
// - Coordinate reference systems and transformations
// - Raster data support
// - Network analysis and routing
//
// ## Overview
//
// The spatial engine is designed to provide Oracle Spatial-level functionality
// for managing, querying, and analyzing geographic data. It supports both vector
// and raster data types, with efficient spatial indexing and a wide range of
// analytical operations.
//
// ## Architecture
//
// ### Geometry System
//
// The geometry module provides a complete implementation of the Simple Features
// specification with extensions for Oracle Spatial compatibility:
//
// ```rust,no_run
// use rusty_db::spatial::geometry::{Point, Coordinate, Polygon};
//
// // Create a point
// let point = Point::new(Coordinate::new(-122.4194, 37.7749));
//
// // Create a linestring
// let coords = vec![
//     Coordinate::new(0.0, 0.0),
//     Coordinate::new(1.0, 1.0),
//     Coordinate::new(2.0, 0.0),
// ];
// let linestring = LineString::new(coords).unwrap();
//
// // Convert to WKT
// println!("Point WKT: {}", point.to_wkt());
// println!("LineString WKT: {}", linestring.to_wkt());
// ```
//
// ### Spatial Indexing
//
// Multiple spatial index types are supported for different use cases:
//
// ```rust,no_run
// use rusty_db::spatial::indexes::{RTree, Quadtree, GridIndex, SpatialIndex};
// use rusty_db::spatial::geometry::BoundingBox;
//
// // Create an R-tree index
// let mut rtree = RTree::new();
//
// // Insert geometries
// let bbox = BoundingBox::new(0.0, 0.0, 1.0, 1.0);
// rtree.insert(1, bbox).unwrap();
//
// // Query the index
// let query_box = BoundingBox::new(-0.5, -0.5, 2.0, 2.0);
// let results = rtree.search(&query_box);
// ```
//
// ### Spatial Operators
//
// Comprehensive topological and geometric operations:
//
// ```rust,no_run
// use rusty_db::spatial::operators::{TopologicalOps, DistanceOps, BufferOps};
// use rusty_db::spatial::geometry::{Geometry, Point, Coordinate};
//
// // Check if geometries intersect
// # let geom1 = Geometry::Point(Point::new(Coordinate::new(0.0, 0.0)));
// # let geom2 = Geometry::Point(Point::new(Coordinate::new(1.0, 1.0)));
// let intersects = TopologicalOps::intersects(&geom1, &geom2).unwrap();
//
// // Calculate distance
// let distance = DistanceOps::distance(&geom1, &geom2).unwrap();
//
// // Create buffer
// let buffered = BufferOps::buffer(&geom1, 10.0).unwrap();
// ```
//
// ### Spatial Analysis
//
// Advanced analytical capabilities including clustering and statistics:
//
// ```rust,no_run
// use rusty_db::spatial::analysis::{DbscanClusterer, KMeansClusterer};
// use rusty_db::spatial::geometry::Coordinate;
//
// let points = vec![
//     (1, Coordinate::new(0.0, 0.0)),
//     (2, Coordinate::new(1.0, 1.0)),
//     (3, Coordinate::new(10.0, 10.0)),
// ];
//
// // DBSCAN clustering
// let dbscan = DbscanClusterer::new(2.0, 2);
// let clusters = dbscan.cluster(&points);
//
// // K-means clustering
// let kmeans = KMeansClusterer::new(2, 100);
// let clusters = kmeans.cluster(&points);
// ```
//
// ### Coordinate Systems
//
// Full support for coordinate transformations and geodetic calculations:
//
// ```rust,no_run
// use rusty_db::spatial::srs::{SrsRegistry, CoordinateTransformer, well_known_srid};
// use rusty_db::spatial::geometry::Coordinate;
// use std::sync::Arc;
//
// let registry = Arc::new(SrsRegistry::new());
// let transformer = CoordinateTransformer::new(registry);
//
// // Transform from WGS84 to Web Mercator
// let wgs84_coord = Coordinate::new(-122.4194, 37.7749);
// let mercator = transformer.transform(
//     &wgs84_coord,
//     well_known_srid::WGS84,
//     well_known_srid::WEB_MERCATOR
// ).unwrap();
// ```
//
// ### Raster Support
//
// Raster data management and analysis:
//
// ```rust,no_run
// use rusty_db::spatial::raster::{Raster, PixelType, GeoTransform, RasterAlgebra};
//
// // Create a raster
// let geo_transform = GeoTransform::new(0.0, 100.0, 1.0, -1.0);
// let raster = Raster::new(100, 100, 3, PixelType::UInt8, geo_transform);
//
// // Perform raster algebra
// # let raster2 = raster.clone();
// let result = RasterAlgebra::add(&raster, &raster2).unwrap();
// ```
//
// ### Network Analysis
//
// Routing and network optimization:
//
// ```rust,no_run
// use rusty_db::spatial::network::{Network, Node, Edge, DijkstraRouter};
// use rusty_db::spatial::geometry::Coordinate;
//
// let mut network = Network::new();
//
// // Add nodes
// network.add_node(Node::new(1, Coordinate::new(0.0, 0.0)));
// network.add_node(Node::new(2, Coordinate::new(1.0, 0.0)));
//
// // Add edges
// network.add_edge(Edge::new(1, 1, 2, 1.0)).unwrap();
//
// // Find shortest path
// let router = DijkstraRouter::new(&network);
// let path = router.shortest_path(1, 2).unwrap();
// ```
//
// ## Performance
//
// The spatial engine is optimized for performance:
//
// - **Spatial Indexing**: R-tree and Quadtree provide O(log n) query performance
// - **Bulk Loading**: Efficient bulk loading using Hilbert curve ordering
// - **Parallel Processing**: Thread-safe indexes for concurrent access
// - **Memory Efficiency**: Zero-copy serialization where possible
//
// ## Oracle Spatial Compatibility
//
// This implementation aims for compatibility with Oracle Spatial features:
//
// | Feature | Oracle Spatial | RustyDB Spatial | Status |
// |---------|---------------|-----------------|--------|
// | Geometry Types | SDO_GEOMETRY | Geometry enum | ✓ Full |
// | WKT/WKB | Supported | Supported | ✓ Full |
// | Spatial Indexing | R-tree | R-tree, Quadtree | ✓ Full |
// | Topological Ops | SDO_RELATE | TopologicalOps | ✓ Full |
// | Distance Ops | SDO_DISTANCE | DistanceOps | ✓ Full |
// | Coordinate Systems | SRID | SRS Registry | ✓ Full |
// | Network Analysis | Network Data Model | Network module | ✓ Full |
// | Raster Support | GeoRaster | Raster module | ✓ Partial |
//
// ## SQL Integration
//
// The spatial engine integrates with RustyDB's SQL layer:
//
// ```sql
// -- Create a spatial table
// CREATE TABLE locations (
//     id INTEGER PRIMARY KEY,
//     name VARCHAR(100),
//     geom GEOMETRY
// );
//
// -- Create spatial index
// CREATE SPATIAL INDEX idx_locations_geom ON locations(geom);
//
// -- Spatial queries
// SELECT name FROM locations
// WHERE ST_Within(geom, ST_MakePolygon(...));
//
// -- Distance queries
// SELECT name, ST_Distance(geom, ST_Point(-122.4194, 37.7749))
// FROM locations
// ORDER BY ST_Distance(geom, ST_Point(-122.4194, 37.7749))
// LIMIT 10;
// ```
//
// ## Best Practices
//
// ### Choosing an Index Type
//
// - **R-tree**: Best for general-purpose spatial data with varying sizes
// - **Quadtree**: Optimal for point data with uniform distribution
// - **Grid Index**: Fast for uniformly distributed data with known bounds
//
// ### Performance Optimization
//
// 1. Use bulk loading for large datasets
// 2. Create spatial indexes before running queries
// 3. Use bounding box filters before precise geometric tests
// 4. Consider pyramid levels for large rasters
// 5. Use appropriate coordinate systems for your data
//
// ### Memory Management
//
// ```rust,no_run
// use rusty_db::spatial::indexes::{SpatialIndexBuilder, IndexType};
// use rusty_db::spatial::geometry::BoundingBox;
//
// // Bulk loading is more memory efficient
// let mut builder = SpatialIndexBuilder::new(
//     IndexType::RTree { max_entries: 8, min_entries: 3 }
// );
//
// for i in 0..1000 {
//     let bbox = BoundingBox::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
//     builder.add(i, bbox);
// }
//
// let index = builder.build().unwrap();
// ```
//
// ## Roadmap
//
// Planned enhancements:
//
// - [ ] 3D spatial indexing (R-tree with Z dimension)
// - [ ] Curved geometry support (NURBS)
// - [ ] Topology validation and repair
// - [ ] Spatial ETL operations
// - [ ] Integration with external formats (Shapefile, GeoTIFF)
// - [ ] GPU acceleration for raster operations
// - [ ] Distributed spatial queries
//
// ## References
//
// - Oracle Spatial and Graph Developer's Guide
// - OGC Simple Features Specification
// - PostGIS Documentation
// - "Computational Geometry: Algorithms and Applications" by de Berg et al.

pub mod analysis;
pub mod geometry;
pub mod indexes;
pub mod network;
pub mod operators;
pub mod raster;
pub mod srs;

// Re-export commonly used types
pub use analysis::{
    DbscanClusterer, DelaunayTriangulation, HotSpotAnalysis, KMeansClusterer, KNearestNeighbors,
    SpatialAggregation, VoronoiDiagram,
};

pub use geometry::{
    BoundingBox, CircularString, CompoundCurve, Coordinate, Geometry, LineString, LinearRing,
    MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, WkbParser, WktParser,
};

pub use indexes::{
    ConcurrentSpatialIndex, GridIndex, HilbertCurve, IndexStats, IndexType, Quadtree, RTree,
    SpatialIndex, SpatialIndexBuilder,
};

pub use network::{
    AStarRouter, DijkstraRouter, Edge, Network, Node, Path, RestrictedNetwork, ServiceAreaAnalyzer,
    TspSolver, TurnRestriction,
};

pub use operators::{
    BufferOps, ConvexHullOps, DistanceOps, SetOps, SimplificationOps, SpatialRelation,
    TopologicalOps, TransformOps,
};

pub use raster::{
    GeoTransform, PixelType, PixelValue, Raster, RasterAlgebra, RasterBand, RasterPyramid,
    RasterVectorConverter, TiledRaster,
};

pub use srs::{
    well_known_srid, CoordinateTransformer, Datum, Ellipsoid, GeodeticCalculator, ProjectionType,
    SpatialReferenceSystem, SridType, SrsRegistry, UtmProjection,
};

// Spatial engine version
pub const VERSION: &str = "1.0.0";

// Spatial capabilities flags
#[derive(Debug, Clone, Copy)]
pub struct SpatialCapabilities {
    pub has_geometry: bool,
    pub has_raster: bool,
    pub has_network: bool,
    pub has_3d: bool,
    pub has_topology: bool,
}

impl Default for SpatialCapabilities {
    fn default() -> Self {
        Self {
            has_geometry: true,
            has_raster: true,
            has_network: true,
            has_3d: true,
            has_topology: true,
        }
    }
}

// Get spatial engine capabilities
pub fn get_capabilities() -> SpatialCapabilities {
    SpatialCapabilities::default()
}

// Spatial engine configuration
#[derive(Debug, Clone)]
pub struct SpatialConfig {
    pub default_srid: SridType,
    pub rtree_max_entries: usize,
    pub rtree_min_entries: usize,
    pub quadtree_max_depth: usize,
    pub enable_parallel: bool,
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            default_srid: well_known_srid::WGS84,
            rtree_max_entries: 8,
            rtree_min_entries: 3,
            quadtree_max_depth: 16,
            enable_parallel: true,
        }
    }
}

// Spatial engine manager
pub struct SpatialEngine {
    config: SpatialConfig,
    srs_registry: std::sync::Arc<SrsRegistry>,
}

impl SpatialEngine {
    // Create a new spatial engine with default configuration
    pub fn new() -> Self {
        Self::with_config(SpatialConfig::default())
    }

    // Create a new spatial engine with custom configuration
    pub fn with_config(config: SpatialConfig) -> Self {
        Self {
            config,
            srs_registry: std::sync::Arc::new(SrsRegistry::new()),
        }
    }

    // Get the SRS registry
    pub fn srs_registry(&self) -> &std::sync::Arc<SrsRegistry> {
        &self.srs_registry
    }

    // Get the configuration
    pub fn config(&self) -> &SpatialConfig {
        &self.config
    }

    // Create a coordinate transformer
    pub fn transformer(&self) -> CoordinateTransformer {
        CoordinateTransformer::new(self.srs_registry.clone())
    }

    // Create an R-tree index with engine configuration
    pub fn create_rtree(&self) -> RTree {
        RTree::with_capacity(self.config.rtree_max_entries, self.config.rtree_min_entries)
    }

    // Create a quadtree index with engine configuration
    pub fn create_quadtree(&self, bounds: BoundingBox) -> Quadtree {
        Quadtree::with_params(bounds, self.config.quadtree_max_depth, 4)
    }

    // Parse WKT geometry
    pub fn parse_wkt(&self, wkt: &str) -> crate::error::Result<Geometry> {
        WktParser::parse(wkt)
    }

    // Parse WKB geometry
    pub fn parse_wkb(&self, wkb: &[u8]) -> crate::error::Result<Geometry> {
        WkbParser::parse(wkb)
    }

    // Get version information
    pub fn version(&self) -> &str {
        VERSION
    }

    // Get capabilities
    pub fn capabilities(&self) -> SpatialCapabilities {
        get_capabilities()
    }
}

impl Default for SpatialEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_engine_creation() {
        let engine = SpatialEngine::new();
        assert_eq!(engine.version(), VERSION);
    }

    #[test]
    fn test_parse_wkt() {
        let engine = SpatialEngine::new();
        let geom = engine.parse_wkt("POINT(1 2)").unwrap();

        match geom {
            Geometry::Point(p) => {
                assert_eq!(p.coord.x, 1.0);
                assert_eq!(p.coord.y, 2.0);
            }
            _ => panic!("Expected Point geometry"),
        }
    }

    #[test]
    fn test_transformer() {
        let engine = SpatialEngine::new();
        let transformer = engine.transformer();

        let wgs84 = Coordinate::new(-122.4194, 37.7749);
        let mercator = transformer
            .transform(
                &wgs84,
                well_known_srid::WGS84,
                well_known_srid::WEB_MERCATOR,
            )
            .unwrap();

        assert_ne!(mercator.x, wgs84.x);
        assert_ne!(mercator.y, wgs84.y);
    }

    #[test]
    fn test_rtree_creation() {
        let engine = SpatialEngine::new();
        let rtree = engine.create_rtree();

        let stats = rtree.stats();
        assert_eq!(stats.num_entries, 0);
    }

    #[test]
    fn test_capabilities() {
        let caps = get_capabilities();
        assert!(caps.has_geometry);
        assert!(caps.has_raster);
        assert!(caps.has_network);
        assert!(caps.has_3d);
        assert!(caps.has_topology);
    }
}
