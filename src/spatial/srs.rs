//! Spatial Reference Systems and Coordinate Transformations
//!
//! Provides Oracle Spatial-compatible SRS management:
//! - EPSG coordinate system registry
//! - Coordinate transformations
//! - Map projections (UTM, Web Mercator, etc.)
//! - Geodetic calculations
//! - Great circle distance calculations

use crate::error::{DbError, Result};
use crate::spatial::geometry::Coordinate;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Arc, RwLock};

/// Spatial Reference System identifier
pub type SridType = i32;

/// Well-known SRIDs
pub mod well_known_srid {
    use super::SridType;

    pub const WGS84: SridType = 4326;          // Geographic WGS84
    pub const WEB_MERCATOR: SridType = 3857;   // Web Mercator (Google Maps)
    pub const UTM_ZONE_10N: SridType = 32610;  // UTM Zone 10N
    pub const NAD83: SridType = 4269;          // North American Datum 1983
    pub const EPSG_3395: SridType = 3395;      // World Mercator
}

/// Spatial Reference System definition
#[derive(Debug, Clone)]
pub struct SpatialReferenceSystem {
    pub srid: SridType,
    pub auth_name: String,
    pub auth_srid: i32,
    pub srtext: String, // WKT representation
    pub proj4text: Option<String>,
    pub projection_type: ProjectionType,
    pub datum: Datum,
    pub ellipsoid: Ellipsoid,
}

/// Projection types
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectionType {
    Geographic,
    Mercator,
    TransverseMercator,
    LambertConformalConic,
    AlbersEqualArea,
    Stereographic,
    Unknown,
}

/// Geodetic datum
#[derive(Debug, Clone)]
pub struct Datum {
    pub name: String,
    pub ellipsoid: Ellipsoid,
    pub to_wgs84: Option<[f64; 7]>, // 7-parameter transformation
}

/// Reference ellipsoid
#[derive(Debug, Clone)]
pub struct Ellipsoid {
    pub name: String,
    pub semi_major_axis: f64, // a (meters)
    pub semi_minor_axis: f64, // b (meters)
    pub inverse_flattening: f64,
}

impl Ellipsoid {
    /// WGS84 ellipsoid
    pub fn wgs84() -> Self {
        Self {
            name: "WGS84".to_string(),
            semi_major_axis: 6378137.0,
            semi_minor_axis: 6356752.314245,
            inverse_flattening: 298.257223563,
        }
    }

    /// GRS 1980 ellipsoid
    pub fn grs80() -> Self {
        Self {
            name: "GRS 1980".to_string(),
            semi_major_axis: 6378137.0,
            semi_minor_axis: 6356752.314140,
            inverse_flattening: 298.257222101,
        }
    }

    /// Calculate eccentricity squared
    pub fn eccentricity_squared(&self) -> f64 {
        let f = 1.0 / self.inverse_flattening;
        2.0 * f - f * f
    }

    /// Calculate second eccentricity squared
    pub fn second_eccentricity_squared(&self) -> f64 {
        let e2 = self.eccentricity_squared();
        e2 / (1.0 - e2)
    }
}

/// SRS Registry for managing spatial reference systems
pub struct SrsRegistry {
    systems: Arc<RwLock<HashMap<SridType, SpatialReferenceSystem>>>,
}

impl SrsRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            systems: Arc::new(RwLock::new(HashMap::new())),
        };
        registry.initialize_common_srs();
        registry
    }

    /// Initialize commonly used spatial reference systems
    fn initialize_common_srs(&mut self) {
        let wgs84_ellipsoid = Ellipsoid::wgs84();

        // WGS84 Geographic
        let wgs84 = SpatialReferenceSystem {
            srid: well_known_srid::WGS84,
            auth_name: "EPSG".to_string(),
            auth_srid: 4326,
            srtext: "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]".to_string(),
            proj4text: Some("+proj=longlat +datum=WGS84 +no_defs".to_string()),
            projection_type: ProjectionType::Geographic,
            datum: Datum {
                name: "WGS_1984".to_string(),
                ellipsoid: wgs84_ellipsoid.clone(),
                to_wgs84: None,
            },
            ellipsoid: wgs84_ellipsoid.clone(),
        };

        // Web Mercator
        let web_mercator = SpatialReferenceSystem {
            srid: well_known_srid::WEB_MERCATOR,
            auth_name: "EPSG".to_string(),
            auth_srid: 3857,
            srtext: "PROJCS[\"WGS 84 / Pseudo-Mercator\",GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],PROJECTION[\"Mercator_1SP\"],PARAMETER[\"central_meridian\",0],PARAMETER[\"scale_factor\",1],PARAMETER[\"false_easting\",0],PARAMETER[\"false_northing\",0],UNIT[\"metre\",1]]".to_string(),
            proj4text: Some("+proj=merc +a=6378137 +b=6378137 +lat_ts=0.0 +lon_0=0.0 +x_0=0.0 +y_0=0 +k=1.0 +units=m +nadgrids=@null +wktext +no_defs".to_string()),
            projection_type: ProjectionType::Mercator,
            datum: Datum {
                name: "WGS_1984".to_string(),
                ellipsoid: wgs84_ellipsoid.clone(),
                to_wgs84: None,
            },
            ellipsoid: wgs84_ellipsoid,
        };

        self.systems.write().unwrap().insert(wgs84.srid, wgs84);
        self.systems.write().unwrap().insert(web_mercator.srid, web_mercator);
    }

    /// Register a new SRS
    pub fn register(&self, srs: SpatialReferenceSystem) -> Result<()> {
        self.systems.write().unwrap().insert(srs.srid, srs);
        Ok(())
    }

    /// Get SRS by SRID
    pub fn get(&self, srid: SridType) -> Option<SpatialReferenceSystem> {
        self.systems.read().unwrap().get(&srid).cloned()
    }

    /// List all registered SRIDs
    pub fn list_srids(&self) -> Vec<SridType> {
        self.systems.read().unwrap().keys().copied().collect()
    }
}

impl Default for SrsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Coordinate transformation engine
pub struct CoordinateTransformer {
    registry: Arc<SrsRegistry>,
}

impl CoordinateTransformer {
    pub fn new(registry: Arc<SrsRegistry>) -> Self {
        Self { registry }
    }

    /// Transform coordinate from source SRS to target SRS
    pub fn transform(
        &self,
        coord: &Coordinate,
        source_srid: SridType,
        target_srid: SridType,
    ) -> Result<Coordinate> {
        if source_srid == target_srid {
            return Ok(*coord);
        }

        let source_srs = self
            .registry
            .get(source_srid)
            .ok_or_else(|| DbError::InvalidInput(format!("Unknown SRID: {}", source_srid)))?;

        let target_srs = self
            .registry
            .get(target_srid)
            .ok_or_else(|| DbError::InvalidInput(format!("Unknown SRID: {}", target_srid)))?;

        // Transform to WGS84 if needed
        let wgs84_coord = if source_srid != well_known_srid::WGS84 {
            self.to_wgs84(coord, &source_srs)?
        } else {
            *coord
        };

        // Transform from WGS84 to target
        if target_srid != well_known_srid::WGS84 {
            self.from_wgs84(&wgs84_coord, &target_srs)
        } else {
            Ok(wgs84_coord)
        }
    }

    /// Transform to WGS84 from any SRS
    fn to_wgs84(&self, coord: &Coordinate, srs: &SpatialReferenceSystem) -> Result<Coordinate> {
        match srs.projection_type {
            ProjectionType::Geographic => Ok(*coord), // Already geographic
            ProjectionType::Mercator => self.mercator_to_wgs84(coord),
            _ => Err(DbError::NotImplemented(
                format!("Transformation from {:?} not implemented", srs.projection_type),
            )),
        }
    }

    /// Transform from WGS84 to any SRS
    fn from_wgs84(&self, coord: &Coordinate, srs: &SpatialReferenceSystem) -> Result<Coordinate> {
        match srs.projection_type {
            ProjectionType::Geographic => Ok(*coord), // Already geographic
            ProjectionType::Mercator => self.wgs84_to_mercator(coord),
            _ => Err(DbError::NotImplemented(
                format!("Transformation to {:?} not implemented", srs.projection_type),
            )),
        }
    }

    /// Convert Web Mercator to WGS84
    fn mercator_to_wgs84(&self, coord: &Coordinate) -> Result<Coordinate> {
        let x = coord.x;
        let y = coord.y;

        let lon = (x / 6378137.0) * (180.0 / PI);
        let lat = (2.0 * ((y / 6378137.0).exp().atan()) - PI / 2.0) * (180.0 / PI);

        Ok(Coordinate::new(lon, lat))
    }

    /// Convert WGS84 to Web Mercator
    fn wgs84_to_mercator(&self, coord: &Coordinate) -> Result<Coordinate> {
        let lon = coord.x;
        let lat = coord.y;

        // Clamp latitude to valid range for Web Mercator
        let lat = lat.max(-85.05112878).min(85.05112878);

        let x = 6378137.0 * lon * (PI / 180.0);
        let y = 6378137.0 * ((PI / 4.0 + lat * (PI / 180.0) / 2.0).tan().ln());

        Ok(Coordinate::new(x, y))
    }

    /// Batch transform coordinates
    pub fn transform_batch(
        &self,
        coords: &[Coordinate],
        source_srid: SridType,
        target_srid: SridType,
    ) -> Result<Vec<Coordinate>> {
        coords
            .iter()
            .map(|c| self.transform(c, source_srid, target_srid))
            .collect()
    }
}

/// UTM (Universal Transverse Mercator) projection
pub struct UtmProjection;

impl UtmProjection {
    /// Convert WGS84 lat/lon to UTM
    pub fn wgs84_to_utm(coord: &Coordinate) -> Result<(Coordinate, u8, bool)> {
        let lon = coord.x;
        let lat = coord.y;

        // Determine zone
        let zone = ((lon + 180.0) / 6.0).floor() as u8 + 1;
        let is_northern = lat >= 0.0;

        // UTM projection parameters
        let lon0 = ((zone as f64 - 1.0) * 6.0 - 180.0 + 3.0) * PI / 180.0; // Central meridian
        let lat_rad = lat * PI / 180.0;
        let lon_rad = lon * PI / 180.0;

        let ellipsoid = Ellipsoid::wgs84();
        let a = ellipsoid.semi_major_axis;
        let e2 = ellipsoid.eccentricity_squared();
        let k0 = 0.9996; // Scale factor

        let n = a / (1.0 - e2 * lat_rad.sin().powi(2)).sqrt();
        let t = lat_rad.tan();
        let c = e2 / (1.0 - e2) * lat_rad.cos().powi(2);
        let a_coef = (lon_rad - lon0) * lat_rad.cos();

        let m = a * (
            (1.0 - e2 / 4.0 - 3.0 * e2.powi(2) / 64.0 - 5.0 * e2.powi(3) / 256.0) * lat_rad
            - (3.0 * e2 / 8.0 + 3.0 * e2.powi(2) / 32.0 + 45.0 * e2.powi(3) / 1024.0) * (2.0 * lat_rad).sin()
            + (15.0 * e2.powi(2) / 256.0 + 45.0 * e2.powi(3) / 1024.0) * (4.0 * lat_rad).sin()
            - (35.0 * e2.powi(3) / 3072.0) * (6.0 * lat_rad).sin()
        );

        let x = k0 * n * (
            a_coef
            + (1.0 - t.powi(2) + c) * a_coef.powi(3) / 6.0
            + (5.0 - 18.0 * t.powi(2) + t.powi(4) + 72.0 * c - 58.0 * e2 / (1.0 - e2)) * a_coef.powi(5) / 120.0
        ) + 500000.0; // False easting

        let y = k0 * (
            m + n * lat_rad.tan() * (
                a_coef.powi(2) / 2.0
                + (5.0 - t.powi(2) + 9.0 * c + 4.0 * c.powi(2)) * a_coef.powi(4) / 24.0
                + (61.0 - 58.0 * t.powi(2) + t.powi(4) + 600.0 * c - 330.0 * e2 / (1.0 - e2)) * a_coef.powi(6) / 720.0
            )
        );

        let y = if is_northern {
            y
        } else {
            y + 10000000.0 // False northing for southern hemisphere
        };

        Ok((Coordinate::new(x, y), zone, is_northern))
    }

    /// Convert UTM to WGS84 lat/lon
    pub fn utm_to_wgs84(coord: &Coordinate, zone: u8, is_northern: bool) -> Result<Coordinate> {
        let x = coord.x - 500000.0; // Remove false easting
        let y = if is_northern {
            coord.y
        } else {
            coord.y - 10000000.0 // Remove false northing
        };

        let ellipsoid = Ellipsoid::wgs84();
        let a = ellipsoid.semi_major_axis;
        let e2 = ellipsoid.eccentricity_squared();
        let k0 = 0.9996;

        let lon0 = ((zone as f64 - 1.0) * 6.0 - 180.0 + 3.0) * PI / 180.0;

        let m = y / k0;
        let mu = m / (a * (1.0 - e2 / 4.0 - 3.0 * e2.powi(2) / 64.0 - 5.0 * e2.powi(3) / 256.0));

        let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());

        let lat_rad = mu
            + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
            + (21.0 * e1.powi(2) / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
            + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin()
            + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();

        let n = a / (1.0 - e2 * lat_rad.sin().powi(2)).sqrt();
        let t = lat_rad.tan();
        let c = e2 / (1.0 - e2) * lat_rad.cos().powi(2);
        let r = a * (1.0 - e2) / (1.0 - e2 * lat_rad.sin().powi(2)).powf(1.5);
        let d = x / (n * k0);

        let lat = lat_rad - (n * t / r) * (
            d.powi(2) / 2.0
            - (5.0 + 3.0 * t.powi(2) + 10.0 * c - 4.0 * c.powi(2) - 9.0 * e2 / (1.0 - e2)) * d.powi(4) / 24.0
            + (61.0 + 90.0 * t.powi(2) + 298.0 * c + 45.0 * t.powi(4) - 252.0 * e2 / (1.0 - e2) - 3.0 * c.powi(2)) * d.powi(6) / 720.0
        );

        let lon = lon0 + (
            d - (1.0 + 2.0 * t.powi(2) + c) * d.powi(3) / 6.0
            + (5.0 - 2.0 * c + 28.0 * t.powi(2) - 3.0 * c.powi(2) + 8.0 * e2 / (1.0 - e2) + 24.0 * t.powi(4)) * d.powi(5) / 120.0
        ) / lat_rad.cos();

        Ok(Coordinate::new(lon * 180.0 / PI, lat * 180.0 / PI))
    }
}

/// Geodetic calculations
pub struct GeodeticCalculator {
    ellipsoid: Ellipsoid,
}

impl GeodeticCalculator {
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self { ellipsoid }
    }

    pub fn wgs84() -> Self {
        Self::new(Ellipsoid::wgs84())
    }

    /// Calculate great circle distance using Vincenty formula
    pub fn vincenty_distance(&self, coord1: &Coordinate, coord2: &Coordinate) -> f64 {
        let lat1 = coord1.y * PI / 180.0;
        let lon1 = coord1.x * PI / 180.0;
        let lat2 = coord2.y * PI / 180.0;
        let lon2 = coord2.x * PI / 180.0;

        let a = self.ellipsoid.semi_major_axis;
        let b = self.ellipsoid.semi_minor_axis;
        let f = 1.0 / self.ellipsoid.inverse_flattening;

        let l = lon2 - lon1;
        let u1 = ((1.0 - f) * lat1.tan()).atan();
        let u2 = ((1.0 - f) * lat2.tan()).atan();

        let sin_u1 = u1.sin();
        let cos_u1 = u1.cos();
        let sin_u2 = u2.sin();
        let cos_u2 = u2.cos();

        let mut lambda = l;
        let mut lambda_prev;
        let mut iter_limit = 100;

        let mut cos_sq_alpha = 0.0;
        let mut sin_sigma = 0.0;
        let mut cos_sigma = 0.0;
        let mut sigma = 0.0;
        let mut cos_2_sigma_m = 0.0;

        loop {
            let sin_lambda = lambda.sin();
            let cos_lambda = lambda.cos();

            sin_sigma = ((cos_u2 * sin_lambda).powi(2)
                + (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda).powi(2))
                .sqrt();

            if sin_sigma == 0.0 {
                return 0.0; // Coincident points
            }

            cos_sigma = sin_u1 * sin_u2 + cos_u1 * cos_u2 * cos_lambda;
            sigma = sin_sigma.atan2(cos_sigma);

            let sin_alpha = cos_u1 * cos_u2 * sin_lambda / sin_sigma;
            cos_sq_alpha = 1.0 - sin_alpha.powi(2);

            cos_2_sigma_m = if cos_sq_alpha != 0.0 {
                cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_sq_alpha
            } else {
                0.0
            };

            let c = f / 16.0 * cos_sq_alpha * (4.0 + f * (4.0 - 3.0 * cos_sq_alpha));

            lambda_prev = lambda;
            lambda = l + (1.0 - c) * f * sin_alpha
                * (sigma + c * sin_sigma * (cos_2_sigma_m + c * cos_sigma * (-1.0 + 2.0 * cos_2_sigma_m.powi(2))));

            iter_limit -= 1;
            if (lambda - lambda_prev).abs() < 1e-12 || iter_limit == 0 {
                break;
            }
        }

        if iter_limit == 0 {
            return f64::NAN; // Formula failed to converge
        }

        let u_sq = cos_sq_alpha * (a.powi(2) - b.powi(2)) / b.powi(2);
        let a_coef = 1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
        let b_coef = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));

        let delta_sigma = b_coef * sin_sigma
            * (cos_2_sigma_m + b_coef / 4.0
                * (cos_sigma * (-1.0 + 2.0 * cos_2_sigma_m.powi(2))
                    - b_coef / 6.0 * cos_2_sigma_m * (-3.0 + 4.0 * sin_sigma.powi(2))
                        * (-3.0 + 4.0 * cos_2_sigma_m.powi(2))));

        b * a_coef * (sigma - delta_sigma)
    }

    /// Calculate great circle distance using Haversine formula (faster, less accurate)
    pub fn haversine_distance(&self, coord1: &Coordinate, coord2: &Coordinate) -> f64 {
        let lat1 = coord1.y * PI / 180.0;
        let lon1 = coord1.x * PI / 180.0;
        let lat2 = coord2.y * PI / 180.0;
        let lon2 = coord2.x * PI / 180.0;

        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;

        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().asin();

        self.ellipsoid.semi_major_axis * c
    }

    /// Calculate bearing from coord1 to coord2
    pub fn bearing(&self, coord1: &Coordinate, coord2: &Coordinate) -> f64 {
        let lat1 = coord1.y * PI / 180.0;
        let lon1 = coord1.x * PI / 180.0;
        let lat2 = coord2.y * PI / 180.0;
        let lon2 = coord2.x * PI / 180.0;

        let dlon = lon2 - lon1;

        let y = dlon.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();

        let bearing_rad = y.atan2(x);
        (bearing_rad * 180.0 / PI + 360.0) % 360.0
    }

    /// Calculate destination point given start, bearing, and distance
    pub fn destination(&self, start: &Coordinate, bearing_deg: f64, distance: f64) -> Coordinate {
        let lat1 = start.y * PI / 180.0;
        let lon1 = start.x * PI / 180.0;
        let bearing = bearing_deg * PI / 180.0;

        let angular_distance = distance / self.ellipsoid.semi_major_axis;

        let lat2 = (lat1.sin() * angular_distance.cos()
            + lat1.cos() * angular_distance.sin() * bearing.cos())
            .asin();

        let lon2 = lon1
            + (bearing.sin() * angular_distance.sin() * lat1.cos())
                .atan2(angular_distance.cos() - lat1.sin() * lat2.sin());

        Coordinate::new(lon2 * 180.0 / PI, lat2 * 180.0 / PI)
    }

    /// Calculate area of a polygon on the ellipsoid
    pub fn polygon_area(&self, coords: &[Coordinate]) -> f64 {
        if coords.len() < 3 {
            return 0.0;
        }

        let mut area = 0.0;

        for i in 0..coords.len() - 1 {
            let p1 = &coords[i];
            let p2 = &coords[i + 1];

            area += (p2.x - p1.x) * PI / 180.0
                * (2.0 + (p1.y * PI / 180.0).sin() + (p2.y * PI / 180.0).sin());
        }

        area = area.abs() * self.ellipsoid.semi_major_axis.powi(2) / 2.0;

        area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_mercator_conversion() {
        let registry = Arc::new(SrsRegistry::new());
        let transformer = CoordinateTransformer::new(registry);

        let wgs84 = Coordinate::new(-122.4194, 37.7749); // San Francisco
        let mercator = transformer
            .transform(&wgs84, well_known_srid::WGS84, well_known_srid::WEB_MERCATOR)
            .unwrap();

        // Transform back
        let back = transformer
            .transform(&mercator, well_known_srid::WEB_MERCATOR, well_known_srid::WGS84)
            .unwrap();

        assert!((back.x - wgs84.x).abs() < 0.0001);
        assert!((back.y - wgs84.y).abs() < 0.0001);
    }

    #[test]
    fn test_utm_conversion() {
        let coord = Coordinate::new(-122.4194, 37.7749); // San Francisco
        let (utm, zone, is_northern) = UtmProjection::wgs84_to_utm(&coord).unwrap();

        assert_eq!(zone, 10);
        assert!(is_northern);

        let back = UtmProjection::utm_to_wgs84(&utm, zone, is_northern).unwrap();

        assert!((back.x - coord.x).abs() < 0.0001);
        assert!((back.y - coord.y).abs() < 0.0001);
    }

    #[test]
    fn test_haversine_distance() {
        let calc = GeodeticCalculator::wgs84();

        let sf = Coordinate::new(-122.4194, 37.7749); // San Francisco
        let la = Coordinate::new(-118.2437, 34.0522); // Los Angeles

        let distance = calc.haversine_distance(&sf, &la);

        // Distance should be approximately 559 km
        assert!((distance - 559000.0).abs() < 10000.0);
    }

    #[test]
    fn test_bearing() {
        let calc = GeodeticCalculator::wgs84();

        let p1 = Coordinate::new(0.0, 0.0);
        let p2 = Coordinate::new(1.0, 1.0);

        let bearing = calc.bearing(&p1, &p2);

        assert!(bearing >= 0.0 && bearing < 360.0);
    }
}


