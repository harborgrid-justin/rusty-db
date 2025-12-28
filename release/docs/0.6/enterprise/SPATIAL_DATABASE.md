# RustyDB v0.6 Spatial Database (PostGIS-Compatible)

**Version**: 0.6.0
**Last Updated**: December 2025
**Target Audience**: GIS Developers, Geospatial Analysts, Location-Based Service Engineers

---

## Overview

RustyDB Spatial Database provides comprehensive geospatial capabilities compatible with PostGIS standards, enabling storage, indexing, and analysis of geographic data. Support for vector geometries, spatial indexes, coordinate reference systems (CRS), network routing, and raster data makes RustyDB suitable for GIS applications, location-based services, and spatial analytics.

### Key Features

- **Geometry Types**: Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon
- **WKT/WKB Support**: Well-Known Text and Binary formats
- **Spatial Indexes**: R-Tree, Quadtree, Grid indexing
- **Coordinate Systems**: 6000+ EPSG codes supported
- **Topological Operators**: Intersects, contains, distance, buffer, union
- **Network Routing**: Dijkstra, A*, bidirectional search
- **Raster Support**: Raster algebra, band operations

---

## Geometry Types

### Point

```sql
-- Create point (longitude, latitude)
CREATE TABLE locations (
  id SERIAL PRIMARY KEY,
  name VARCHAR(100),
  location GEOMETRY(Point, 4326)  -- WGS84
);

-- Insert points
INSERT INTO locations (name, location)
VALUES
  ('San Francisco', ST_GeomFromText('POINT(-122.4194 37.7749)', 4326)),
  ('New York', ST_GeomFromText('POINT(-74.0060 40.7128)', 4326)),
  ('London', ST_GeomFromText('POINT(-0.1278 51.5074)', 4326));

-- Alternative: ST_MakePoint
INSERT INTO locations (name, location)
VALUES ('Paris', ST_SetSRID(ST_MakePoint(2.3522, 48.8566), 4326));
```

### LineString

```sql
-- Create route/path
CREATE TABLE routes (
  id SERIAL PRIMARY KEY,
  name VARCHAR(100),
  path GEOMETRY(LineString, 4326)
);

INSERT INTO routes (name, path)
VALUES ('Highway 101', ST_GeomFromText(
  'LINESTRING(-122.4194 37.7749, -122.0839 37.4221, -121.8949 37.3394)',
  4326
));
```

### Polygon

```sql
-- Create area/region
CREATE TABLE regions (
  id SERIAL PRIMARY KEY,
  name VARCHAR(100),
  boundary GEOMETRY(Polygon, 4326)
);

-- Insert polygon (exterior ring, holes optional)
INSERT INTO regions (name, boundary)
VALUES ('Park', ST_GeomFromText(
  'POLYGON((
    -122.5 37.8,
    -122.3 37.8,
    -122.3 37.7,
    -122.5 37.7,
    -122.5 37.8
  ))',
  4326
));

-- Polygon with hole
INSERT INTO regions (name, boundary)
VALUES ('Lake with Island', ST_GeomFromText(
  'POLYGON((0 0, 10 0, 10 10, 0 10, 0 0), (2 2, 8 2, 8 8, 2 8, 2 2))',
  4326
));
```

### Multi-Geometries

```sql
-- MultiPoint
INSERT INTO locations (name, location)
VALUES ('Store Locations', ST_GeomFromText(
  'MULTIPOINT((-122.4 37.7), (-122.5 37.8), (-122.6 37.9))',
  4326
));

-- MultiLineString
INSERT INTO routes (name, path)
VALUES ('Rail Network', ST_GeomFromText(
  'MULTILINESTRING((-122.4 37.7, -122.5 37.8), (-122.5 37.8, -122.6 37.9))',
  4326
));

-- MultiPolygon
INSERT INTO regions (name, boundary)
VALUES ('Island Chain', ST_GeomFromText(
  'MULTIPOLYGON(((0 0, 1 0, 1 1, 0 1, 0 0)), ((2 2, 3 2, 3 3, 2 3, 2 2)))',
  4326
));
```

---

## Spatial Indexes

### R-Tree Index

**Best for**: General-purpose spatial queries (range, nearest neighbor)

```sql
-- Create R-Tree index
CREATE INDEX locations_geom_idx ON locations USING GIST (location);

-- Query uses R-Tree for bounding box search
SELECT name
FROM locations
WHERE ST_DWithin(location, ST_MakePoint(-122.4194, 37.7749), 0.1);  -- ~11km
```

### Quadtree Index

**Best for**: Hierarchical spatial partitioning, deep recursion

```sql
-- Create Quadtree index
CREATE INDEX locations_quadtree_idx ON locations USING QUADTREE (location);

-- Suitable for varying density data
```

### Grid Index

**Best for**: Uniformly distributed data, simple range queries

```sql
-- Create grid index with cell size
CREATE INDEX locations_grid_idx ON locations USING GRID (location)
WITH (grid_size = 0.01);  -- ~1.1km cells
```

---

## Spatial Queries

### Distance Queries

```sql
-- Distance between two points (meters for WGS84)
SELECT ST_Distance(
  (SELECT location FROM locations WHERE name = 'San Francisco'),
  (SELECT location FROM locations WHERE name = 'New York')
) as distance_meters;

-- Output: 4135172.5 (meters ~= 2569 miles)

-- Points within distance
SELECT name,
       ST_Distance(location, ST_MakePoint(-122.4194, 37.7749)) as distance
FROM locations
WHERE ST_DWithin(location, ST_MakePoint(-122.4194, 37.7749), 100000)  -- 100km
ORDER BY distance;
```

### Nearest Neighbor

```sql
-- Find 5 nearest locations
SELECT name,
       ST_Distance(location, ST_MakePoint(-122.4194, 37.7749)) as distance
FROM locations
ORDER BY location <-> ST_MakePoint(-122.4194, 37.7749)  -- KNN operator
LIMIT 5;
```

### Containment

```sql
-- Point in polygon
SELECT l.name
FROM locations l
JOIN regions r ON ST_Contains(r.boundary, l.location)
WHERE r.name = 'Downtown';

-- Polygon contains polygon
SELECT r1.name as outer_region,
       r2.name as inner_region
FROM regions r1, regions r2
WHERE ST_Contains(r1.boundary, r2.boundary)
  AND r1.id != r2.id;
```

### Intersection

```sql
-- Geometries that intersect
SELECT r1.name, r2.name
FROM regions r1, regions r2
WHERE ST_Intersects(r1.boundary, r2.boundary)
  AND r1.id < r2.id;  -- Avoid duplicates

-- Intersection area
SELECT r1.name, r2.name,
       ST_Area(ST_Intersection(r1.boundary, r2.boundary)) as overlap_area
FROM regions r1, regions r2
WHERE ST_Intersects(r1.boundary, r2.boundary)
  AND r1.id != r2.id;
```

### Bounding Box

```sql
-- Get bounding box
SELECT name,
       ST_XMin(location) as min_lon,
       ST_YMin(location) as min_lat,
       ST_XMax(location) as max_lon,
       ST_YMax(location) as max_lat
FROM locations;

-- Expand bounding box
SELECT ST_Expand(location, 0.1) as bbox  -- Expand by 0.1 degrees
FROM locations
WHERE name = 'San Francisco';
```

---

## Topological Operators

### Buffer

```sql
-- Create buffer around point (in meters for geography)
SELECT name,
       ST_Buffer(location::geography, 5000)::geometry as buffer_5km
FROM locations;

-- Buffer around line (offset)
SELECT name,
       ST_Buffer(path, 0.01) as corridor
FROM routes;
```

### Union

```sql
-- Merge overlapping polygons
SELECT ST_Union(boundary) as merged_region
FROM regions
WHERE category = 'protected_area';

-- Dissolve by attribute
SELECT category,
       ST_Union(boundary) as category_region
FROM regions
GROUP BY category;
```

### Difference

```sql
-- Subtract one geometry from another
SELECT ST_Difference(
  (SELECT boundary FROM regions WHERE name = 'State'),
  (SELECT boundary FROM regions WHERE name = 'Protected')
) as developable_land;
```

### Symmetric Difference

```sql
-- Areas in either but not both
SELECT ST_SymDifference(r1.boundary, r2.boundary) as exclusive_areas
FROM regions r1, regions r2
WHERE r1.name = 'Region A' AND r2.name = 'Region B';
```

### Convex Hull

```sql
-- Smallest convex polygon containing all points
SELECT ST_ConvexHull(ST_Collect(location)) as bounding_area
FROM locations
WHERE category = 'store';
```

### Centroid

```sql
-- Geometric center
SELECT name,
       ST_Centroid(boundary) as center_point
FROM regions;
```

---

## Coordinate Reference Systems

### Transform Coordinates

```sql
-- Transform from WGS84 (4326) to Web Mercator (3857)
SELECT name,
       ST_Transform(location, 3857) as web_mercator
FROM locations;

-- Transform to UTM Zone 10N (32610) for accurate distance in SF
SELECT name,
       ST_Transform(location, 32610) as utm_coords
FROM locations
WHERE ST_DWithin(location, ST_MakePoint(-122.4194, 37.7749), 1.0);
```

### Get SRID

```sql
-- Get coordinate system
SELECT name,
       ST_SRID(location) as srid
FROM locations;

-- Set SRID
UPDATE locations
SET location = ST_SetSRID(location, 4326)
WHERE ST_SRID(location) = 0;
```

### Supported Systems

- **EPSG:4326**: WGS84 (GPS coordinates)
- **EPSG:3857**: Web Mercator (Google Maps, OpenStreetMap)
- **EPSG:3395**: World Mercator
- **EPSG:326XX**: UTM zones (accurate local measurements)
- 6000+ EPSG codes supported

---

## Network Routing

### Create Network

```sql
-- Road network
CREATE TABLE road_network (
  id SERIAL PRIMARY KEY,
  source_id INTEGER,
  target_id INTEGER,
  geom GEOMETRY(LineString, 4326),
  length FLOAT,  -- meters
  speed_limit INTEGER,  -- km/h
  road_type VARCHAR(50)
);

-- Nodes (intersections)
CREATE TABLE network_nodes (
  id SERIAL PRIMARY KEY,
  geom GEOMETRY(Point, 4326)
);

-- Populate network
INSERT INTO road_network (source_id, target_id, geom, length, speed_limit, road_type)
VALUES
  (1, 2, ST_MakeLine(ST_MakePoint(-122.4, 37.7), ST_MakePoint(-122.5, 37.8)), 15000, 60, 'highway'),
  (2, 3, ST_MakeLine(ST_MakePoint(-122.5, 37.8), ST_MakePoint(-122.6, 37.9)), 12000, 50, 'arterial');
```

### Shortest Path (Dijkstra)

```sql
-- Find shortest path
SELECT *
FROM shortest_path(
  'road_network',  -- Edge table
  source_node => 1,
  target_node => 10,
  weight_column => 'length'
);

-- Output:
-- path: [1, 4, 7, 10]
-- edges: [e1, e5, e9]
-- total_cost: 45000  -- meters
```

### A* Algorithm

```sql
-- A* with heuristic (faster for spatial networks)
SELECT *
FROM shortest_path_astar(
  edge_table => 'road_network',
  source => 1,
  target => 10,
  heuristic => 'euclidean'  -- or 'haversine' for lat/lon
);
```

### Routing with Turn Restrictions

```sql
-- Turn restrictions table
CREATE TABLE turn_restrictions (
  id SERIAL PRIMARY KEY,
  from_edge INTEGER,
  via_node INTEGER,
  to_edge INTEGER,
  restriction_type VARCHAR(20)  -- 'no_left_turn', 'no_u_turn'
);

-- Route with restrictions
SELECT *
FROM shortest_path_with_restrictions(
  edge_table => 'road_network',
  restrictions_table => 'turn_restrictions',
  source => 1,
  target => 10
);
```

### Isochrone (Reachability)

```sql
-- All nodes reachable within 30 minutes
SELECT node_id,
       travel_time_seconds,
       ST_AsText(geom) as location
FROM calculate_isochrone(
  edge_table => 'road_network',
  source_node => 1,
  max_cost => 1800,  -- 30 minutes
  cost_function => 'length / (speed_limit * 0.277778)'  -- Convert km/h to m/s
);
```

---

## Raster Support

### Create Raster

```sql
-- Elevation data
CREATE TABLE elevation_raster (
  id SERIAL PRIMARY KEY,
  rast RASTER,
  region VARCHAR(100)
);

-- Load raster from GeoTIFF
INSERT INTO elevation_raster (rast, region)
VALUES (ST_FromGeoTIFF('/data/elevation.tif'), 'California');
```

### Raster Queries

```sql
-- Get elevation at point
SELECT ST_Value(rast, ST_MakePoint(-122.4194, 37.7749)) as elevation_meters
FROM elevation_raster
WHERE region = 'California';

-- Raster statistics
SELECT ST_SummaryStats(rast) as stats
FROM elevation_raster;

-- Output:
-- {
--   "count": 1000000,
--   "sum": 50000000,
--   "mean": 50,
--   "stddev": 25.5,
--   "min": 0,
--   "max": 4200
-- }
```

### Raster Algebra

```sql
-- Slope calculation
SELECT ST_Slope(rast, 1, '32BF') as slope_raster
FROM elevation_raster;

-- Aspect (direction of slope)
SELECT ST_Aspect(rast, 1, '32BF') as aspect_raster
FROM elevation_raster;

-- Hillshade
SELECT ST_Hillshade(rast, 1, '32BF', 45, 315) as hillshade
FROM elevation_raster;
```

### Raster Overlay

```sql
-- Combine two rasters (map algebra)
SELECT ST_MapAlgebra(
  r1.rast,
  r2.rast,
  '[rast1] + [rast2]'  -- Add values
) as combined_raster
FROM elevation_raster r1, landcover_raster r2
WHERE ST_Intersects(r1.rast, r2.rast);
```

---

## Spatial Analysis

### Density Analysis

```sql
-- Kernel density estimation
SELECT ST_KernelDensity(
  ST_Collect(location),
  bandwidth => 1000,  -- meters
  grid_size => 100
) as density_raster
FROM locations
WHERE category = 'crime';
```

### Hot Spot Analysis

```sql
-- Getis-Ord Gi* statistic
SELECT location_id,
       getis_ord_gi_star(location_id, 'locations', 'value') as gi_score,
       CASE
         WHEN gi_score > 2.58 THEN 'Hot Spot (99% confidence)'
         WHEN gi_score > 1.96 THEN 'Hot Spot (95% confidence)'
         WHEN gi_score < -2.58 THEN 'Cold Spot (99% confidence)'
         WHEN gi_score < -1.96 THEN 'Cold Spot (95% confidence)'
         ELSE 'Not significant'
       END as classification
FROM locations;
```

### Spatial Clustering (DBSCAN)

```sql
-- Find spatial clusters
SELECT cluster_id,
       array_agg(location_id) as members,
       count(*) as cluster_size
FROM (
  SELECT location_id,
         ST_ClusterDBSCAN(location, eps => 0.01, minpoints => 5) OVER () as cluster_id
  FROM locations
) clustered
WHERE cluster_id IS NOT NULL
GROUP BY cluster_id;
```

---

## Performance Optimization

### Spatial Indexes

```sql
-- Always index geometry columns
CREATE INDEX ON locations USING GIST (location);

-- Compound spatial index
CREATE INDEX ON parcels USING GIST (boundary, owner_id);

-- Partial spatial index (only active records)
CREATE INDEX ON locations USING GIST (location)
WHERE active = true;
```

### Bounding Box Queries

```sql
-- Use && operator for bounding box (index-supported)
SELECT name
FROM locations
WHERE location && ST_MakeEnvelope(-122.5, 37.7, -122.3, 37.9, 4326);

-- Then apply expensive geometry check
SELECT name
FROM locations
WHERE location && ST_MakeEnvelope(-122.5, 37.7, -122.3, 37.9, 4326)
  AND ST_Contains(
    ST_MakeEnvelope(-122.5, 37.7, -122.3, 37.9, 4326),
    location
  );
```

### Simplify Geometries

```sql
-- Reduce complexity for faster queries
SELECT name,
       ST_Simplify(boundary, 0.001) as simplified_boundary
FROM regions
WHERE ST_NPoints(boundary) > 1000;

-- Preserve topology while simplifying
SELECT ST_SimplifyPreserveTopology(boundary, 0.001)
FROM regions;
```

### Geography vs. Geometry

```sql
-- Geography: accurate distances on sphere (slower)
CREATE TABLE locations_geo (
  id SERIAL PRIMARY KEY,
  name VARCHAR(100),
  location GEOGRAPHY(Point, 4326)
);

-- Geometry: fast planar calculations (less accurate for large areas)
CREATE TABLE locations_geom (
  id SERIAL PRIMARY KEY,
  name VARCHAR(100),
  location GEOMETRY(Point, 4326)
);

-- Use geography for < 1000 km distances
-- Use geometry for local, projected data
```

---

## Use Cases

### Location-Based Services

```sql
-- Find nearby restaurants
SELECT name,
       ST_Distance(location::geography, user_location::geography) as distance
FROM restaurants
WHERE ST_DWithin(location::geography, user_location::geography, 5000)  -- 5km
ORDER BY distance
LIMIT 10;
```

### Logistics & Routing

```sql
-- Delivery route optimization
-- Territory assignment
-- Warehouse location analysis
```

### Urban Planning

```sql
-- Zoning analysis
-- Land use planning
-- Infrastructure impact assessment
```

### Environmental Monitoring

```sql
-- Pollution dispersion modeling
-- Watershed delineation
-- Habitat analysis
```

---

## Best Practices

1. **Always Index**: Create GIST indexes on geometry columns
2. **Use Appropriate SRID**: Choose correct coordinate system for accuracy
3. **Bounding Box First**: Use && for index-supported filtering
4. **Simplify When Possible**: Reduce geometry complexity
5. **Geography for Global**: Use geography type for lat/lon data
6. **Validate Geometries**: Check ST_IsValid before operations
7. **Avoid ST_Distance in WHERE**: Use ST_DWithin for better performance
8. **Cluster Data**: CLUSTER table on spatial index for better locality
9. **Analyze Statistics**: Run ANALYZE after bulk inserts
10. **Monitor Performance**: EXPLAIN queries to verify index usage

---

**See Also**:
- [Specialized Engines Flow](/diagrams/08_specialized_engines_flow.md)
- [Spatial Functions Reference](../reference/SPATIAL_FUNCTIONS.md)
- [Performance Tuning](../operations/PERFORMANCE_TUNING.md)

**Document Version**: 1.0
**Last Updated**: December 2025
