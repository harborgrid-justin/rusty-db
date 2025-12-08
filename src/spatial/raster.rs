//! Raster Data Support
//!
//! Oracle Spatial-compatible raster operations:
//! - Raster data types and storage
//! - Raster algebra operations
//! - Raster to vector conversion
//! - Pyramids and overviews for performance
//! - Tile-based storage for large rasters

use crate::error::Result;
use crate::spatial::geometry::{BoundingBox, Coordinate, LinearRing, Polygon};
use std::collections::HashMap;

/// Pixel data types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelType {
    UInt8,
    UInt16,
    UInt32,
    Int8,
    Int16,
    Int32,
    Float32,
    Float64,
}

impl PixelType {
    pub fn size_bytes(&self) -> usize {
        match self {
            PixelType::UInt8 | PixelType::Int8 => 1,
            PixelType::UInt16 | PixelType::Int16 => 2,
            PixelType::UInt32 | PixelType::Int32 | PixelType::Float32 => 4,
            PixelType::Float64 => 8,
        }
    }
}

/// Pixel value enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelValue {
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Float32(f32),
    Float64(f64),
    NoData,
}

impl PixelValue {
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            PixelValue::UInt8(v) => Some(*v as f64),
            PixelValue::UInt16(v) => Some(*v as f64),
            PixelValue::UInt32(v) => Some(*v as f64),
            PixelValue::Int8(v) => Some(*v as f64),
            PixelValue::Int16(v) => Some(*v as f64),
            PixelValue::Int32(v) => Some(*v as f64),
            PixelValue::Float32(v) => Some(*v as f64),
            PixelValue::Float64(v) => Some(*v),
            PixelValue::NoData => None,
        }
    }
}

/// Raster band
#[derive(Debug, Clone)]
pub struct RasterBand {
    pub width: usize,
    pub height: usize,
    pub pixel_type: PixelType,
    pub data: Vec<u8>,
    pub no_data_value: Option<f64>,
    pub statistics: Option<BandStatistics>,
}

#[derive(Debug, Clone)]
pub struct BandStatistics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std_dev: f64,
}

impl RasterBand {
    pub fn new(width: usize, height: usize, pixel_type: PixelType) -> Self {
        let size = width * height * pixel_type.size_bytes();
        Self {
            width,
            height,
            pixel_type,
            data: vec![0; size],
            no_data_value: None,
            statistics: None,
        }
    }

    /// Get pixel value at (x, y)
    pub fn get_pixel(&self, x: usize, y: usize) -> Result<PixelValue> {
        if x >= self.width || y >= self.height {
            return Err(DbError::InvalidInput("Pixel coordinates out of bounds".to_string()));
        }

        let idx = (y * self.width + x) * self.pixel_type.size_bytes();

        let _value = match self.pixel_type {
            PixelType::UInt8 => PixelValue::UInt8(self.data[idx]),
            PixelType::UInt16 => {
                let bytes = [self.data[idx], self.data[idx + 1]];
                PixelValue::UInt16(u16::from_le_bytes(bytes))
            }
            PixelType::Float32 => {
                let bytes = [
                    self.data[idx],
                    self.data[idx + 1],
                    self.data[idx + 2],
                    self.data[idx + 3],
                ];
                PixelValue::Float32(f32::from_le_bytes(bytes))
            }
            _ => PixelValue::NoData,
        };

        Ok(value)
    }

    /// Set pixel value at (x, y)
    pub fn set_pixel(&mut self, x: usize, y: usize, value: PixelValue) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Err(DbError::InvalidInput("Pixel coordinates out of bounds".to_string()));
        }

        let idx = (y * self.width + x) * self.pixel_type.size_bytes();

        match (self.pixel_type, value) {
            (PixelType::UInt8, PixelValue::UInt8(v)) => {
                self.data[idx] = v;
            }
            (PixelType::UInt16, PixelValue::UInt16(v)) => {
                let bytes = v.to_le_bytes();
                self.data[idx] = bytes[0];
                self.data[idx + 1] = bytes[1];
            }
            (PixelType::Float32, PixelValue::Float32(v)) => {
                let bytes = v.to_le_bytes();
                for _i in 0..4 {
                    self.data[idx + i] = bytes[i];
                }
            }
            _ => {
                return Err(DbError::InvalidInput(
                    "Pixel value type mismatch".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Calculate band statistics
    pub fn calculate_statistics(&mut self) -> Result<()> {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let mut count = 0;

        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(value) = self.get_pixel(x, y)?.to_f64() {
                    if let Some(no_data) = self.no_data_value {
                        if (value - no_data).abs() < 1e-10 {
                            continue;
                        }
                    }

                    min = min.min(value);
                    max = max.max(value);
                    sum += value;
                    count += 1;
                }
            }
        }

        if count == 0 {
            return Ok(());
        }

        let mean = sum / count as f64;

        // Calculate standard deviation
        let mut sum_sq_diff = 0.0;
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(value) = self.get_pixel(x, y)?.to_f64() {
                    if let Some(no_data) = self.no_data_value {
                        if (value - no_data).abs() < 1e-10 {
                            continue;
                        }
                    }
                    sum_sq_diff += (value - mean).powi(2);
                }
            }
        }

        let std_dev = (sum_sq_diff / count as f64).sqrt();

        self.statistics = Some(BandStatistics {
            min,
            max,
            mean,
            std_dev,
        });

        Ok(())
    }
}

/// Georeferencing information
#[derive(Debug, Clone)]
pub struct GeoTransform {
    pub origin_x: f64,      // X coordinate of upper-left corner
    pub origin_y: f64,      // Y coordinate of upper-left corner
    pub pixel_width: f64,   // W-E pixel resolution
    pub pixel_height: f64,  // N-S pixel resolution (usually negative)
    pub rotation_x: f64,    // Rotation parameter
    pub rotation_y: f64,    // Rotation parameter
}

impl GeoTransform {
    pub fn new(origin_x: f64, origin_y: f64, pixel_width: f64, pixel_height: f64) -> Self {
        Self {
            origin_x,
            origin_y,
            pixel_width,
            pixel_height,
            rotation_x: 0.0,
            rotation_y: 0.0,
        }
    }

    /// Convert pixel coordinates to world coordinates
    pub fn pixel_to_world(&self, pixel_x: f64, pixel_y: f64) -> Coordinate {
        let x = self.origin_x + pixel_x * self.pixel_width + pixel_y * self.rotation_x;
        let y = self.origin_y + pixel_x * self.rotation_y + pixel_y * self.pixel_height;
        Coordinate::new(x, y)
    }

    /// Convert world coordinates to pixel coordinates
    pub fn world_to_pixel(&self, world_x: f64, world_y: f64) -> (f64, f64) {
        let det = self.pixel_width * self.pixel_height - self.rotation_x * self.rotation_y;

        if det.abs() < 1e-10 {
            return (0.0, 0.0);
        }

        let dx = world_x - self.origin_x;
        let dy = world_y - self.origin_y;

        let pixel_x = (self.pixel_height * dx - self.rotation_x * dy) / det;
        let pixel_y = (-self.rotation_y * dx + self.pixel_width * dy) / det;

        (pixel_x, pixel_y)
    }
}

/// Main raster dataset
#[derive(Debug, Clone)]
pub struct Raster {
    pub width: usize,
    pub height: usize,
    pub bands: Vec<RasterBand>,
    pub geo_transform: GeoTransform,
    pub srid: Option<i32>,
    pub metadata: HashMap<String, String>,
}

impl Raster {
    pub fn new(
        width: usize,
        height: usize,
        num_bands: usize,
        pixel_type: PixelType,
        geo_transform: GeoTransform,
    ) -> Self {
        let bands = (0..num_bands)
            .map(|_| RasterBand::new(width, height, pixel_type))
            .collect();

        Self {
            width,
            height,
            bands,
            geo_transform,
            srid: None,
            metadata: HashMap::new(),
        }
    }

    /// Get bounding box of the raster
    pub fn bbox(&self) -> BoundingBox {
        let min_coord = self.geo_transform.pixel_to_world(0.0, 0.0);
        let max_coord = self
            .geo_transform
            .pixel_to_world(self.width as f64, self.height as f64);

        BoundingBox::new(
            min_coord.x.min(max_coord.x),
            min_coord.y.min(max_coord.y),
            min_coord.x.max(max_coord.x),
            min_coord.y.max(max_coord.y),
        )
    }

    /// Get pixel value at world coordinates
    pub fn get_value_at(&self, coord: &Coordinate, band_idx: usize) -> Result<PixelValue> {
        if band_idx >= self.bands.len() {
            return Err(DbError::InvalidInput("Band index out of range".to_string()));
        }

        let (pixel_x, pixel_y) = self.geo_transform.world_to_pixel(coord.x, coord.y);

        if pixel_x < 0.0
            || pixel_y < 0.0
            || pixel_x >= self.width as f64
            || pixel_y >= self.height as f64
        {
            return Ok(PixelValue::NoData);
        }

        self.bands[band_idx].get_pixel(pixel_x as usize, pixel_y as usize)
    }
}

/// Raster algebra operations
pub struct RasterAlgebra;

impl RasterAlgebra {
    /// Add two rasters
    pub fn add(raster1: &Raster, raster2: &Raster) -> Result<Raster> {
        Self::binary_op(raster1, raster2, |a, b| a + b)
    }

    /// Subtract raster2 from raster1
    pub fn subtract(raster1: &Raster, raster2: &Raster) -> Result<Raster> {
        Self::binary_op(raster1, raster2, |a, b| a - b)
    }

    /// Multiply two rasters
    pub fn multiply(raster1: &Raster, raster2: &Raster) -> Result<Raster> {
        Self::binary_op(raster1, raster2, |a, b| a * b)
    }

    /// Divide raster1 by raster2
    pub fn divide(raster1: &Raster, raster2: &Raster) -> Result<Raster> {
        Self::binary_op(raster1, raster2, |a, b| if b != 0.0 { a / b } else { 0.0 })
    }

    /// Generic binary operation
    fn binary_op<F>(raster1: &Raster, raster2: &Raster, op: F) -> Result<Raster>
    where
        F: Fn(f64, f64) -> f64,
    {
        if raster1.width != raster2.width || raster1.height != raster2.height {
            return Err(DbError::InvalidInput("Raster dimensions must match".to_string()));
        }

        if raster1.bands.len() != raster2.bands.len() {
            return Err(DbError::InvalidInput("Number of bands must match".to_string()));
        }

        let mut result = Raster::new(
            raster1.width,
            raster1.height,
            raster1.bands.len(),
            PixelType::Float64,
            raster1.geo_transform.clone(),
        );

        for band_idx in 0..raster1.bands.len() {
            for y in 0..raster1.height {
                for x in 0..raster1.width {
                    let val1 = raster1.bands[band_idx].get_pixel(x, y)?.to_f64();
                    let val2 = raster2.bands[band_idx].get_pixel(x, y)?.to_f64();

                    let result_val = match (val1, val2) {
                        (Some(v1), Some(v2)) => PixelValue::Float64(op(v1, v2)),
                        _ => PixelValue::NoData,
                    };

                    result.bands[band_idx].set_pixel(x, y, result_val)?;
                }
            }
        }

        Ok(result)
    }

    /// Calculate NDVI (Normalized Difference Vegetation Index)
    pub fn ndvi(nir_band: &RasterBand, red_band: &RasterBand) -> Result<RasterBand> {
        if nir_band.width != red_band.width || nir_band.height != red_band.height {
            return Err(DbError::InvalidInput("Band dimensions must match".to_string()));
        }

        let mut result = RasterBand::new(nir_band.width, nir_band.height, PixelType::Float32);

        for y in 0..nir_band.height {
            for x in 0..nir_band.width {
                let nir = nir_band.get_pixel(x, y)?.to_f64();
                let red = red_band.get_pixel(x, y)?.to_f64();

                let ndvi = match (nir, red) {
                    (Some(n), Some(r)) if (n + r).abs() > 1e-10 => {
                        PixelValue::Float32(((n - r) / (n + r)) as f32)
                    }
                    _ => PixelValue::NoData,
                };

                result.set_pixel(x, y, ndvi)?;
            }
        }

        Ok(result)
    }

    /// Apply a focal filter (moving window operation)
    pub fn focal_filter(band: &RasterBand, kernel_size: usize) -> Result<RasterBand> {
        let mut result = RasterBand::new(band.width, band.height, PixelType::Float32);
        let half_kernel = kernel_size / 2;

        for y in 0..band.height {
            for x in 0..band.width {
                let mut sum = 0.0;
                let mut count = 0;

                for ky in 0..kernel_size {
                    for kx in 0..kernel_size {
                        let px = x as i32 + kx as i32 - half_kernel as i32;
                        let py = y as i32 + ky as i32 - half_kernel as i32;

                        if px >= 0 && py >= 0 && px < band.width as i32 && py < band.height as i32 {
                            if let Some(val) = band.get_pixel(px as usize, py as usize)?.to_f64() {
                                sum += val;
                                count += 1;
                            }
                        }
                    }
                }

                let result_val = if count > 0 {
                    PixelValue::Float32((sum / count as f64) as f32)
                } else {
                    PixelValue::NoData
                };

                result.set_pixel(x, y, result_val)?;
            }
        }

        Ok(result)
    }
}

/// Raster to vector conversion
pub struct RasterVectorConverter;

impl RasterVectorConverter {
    /// Convert raster to polygons (contour polygons)
    pub fn vectorize(band: &RasterBand, geo_transform: &GeoTransform) -> Result<Vec<Polygon>> {
        let mut polygons = Vec::new();

        // Simple threshold-based vectorization
        let threshold = if let Some(stats) = &band.statistics {
            stats.mean
        } else {
            128.0
        };

        let mut visited = vec![vec![false; band.width]; band.height];

        for y in 0..band.height {
            for x in 0..band.width {
                if visited[y][x] {
                    continue;
                }

                if let Some(val) = band.get_pixel(x, y).ok().and_then(|v| v.to_f64()) {
                    if val >= threshold {
                        if let Some(polygon) = Self::trace_polygon(
                            band,
                            geo_transform,
                            x,
                            y,
                            threshold,
                            &mut visited,
                        )? {
                            polygons.push(polygon);
                        }
                    }
                }
            }
        }

        Ok(polygons)
    }

    fn trace_polygon(
        band: &RasterBand,
        geo_transform: &GeoTransform,
        start_x: usize,
        start_y: usize,
        threshold: f64,
        visited: &mut Vec<Vec<bool>>,
    ) -> Result<Option<Polygon>> {
        // Simplified flood fill to find contiguous region
        let mut region = Vec::new();
        let mut stack = vec![(start_x, start_y)];

        while let Some((x, y)) = stack.pop() {
            if x >= band.width || y >= band.height || visited[y][x] {
                continue;
            }

            if let Some(val) = band.get_pixel(x, y).ok().and_then(|v| v.to_f64()) {
                if val < threshold {
                    continue;
                }

                visited[y][x] = true;
                region.push((x, y));

                // Add neighbors
                if x > 0 {
                    stack.push((x - 1, y));
                }
                if x < band.width - 1 {
                    stack.push((x + 1, y));
                }
                if y > 0 {
                    stack.push((x, y - 1));
                }
                if y < band.height - 1 {
                    stack.push((x, y + 1));
                }
            }
        }

        if region.is_empty() {
            return Ok(None);
        }

        // Create bounding polygon from region
        let coords: Vec<Coordinate> = region
            .iter()
            .map(|(x, y)| geo_transform.pixel_to_world(*x as f64, *y as f64))
            .collect();

        if coords.len() < 3 {
            return Ok(None);
        }

        // Create convex hull of the region (simplified)
        let mut hull_coords = coords;
        hull_coords.push(hull_coords[0]);

        let ring = LinearRing::new(hull_coords)?;
        Ok(Some(Polygon::new(ring, vec![])))
    }

    /// Extract contour lines at specified values
    pub fn contour_lines(
        band: &RasterBand,
        geo_transform: &GeoTransform,
        levels: &[f64],
    ) -> Result<Vec<Vec<Coordinate>>> {
        let mut contours = Vec::new();

        for &level in levels {
            let contour = Self::trace_contour(band, geo_transform, level)?;
            contours.extend(contour);
        }

        Ok(contours)
    }

    fn trace_contour(
        band: &RasterBand,
        geo_transform: &GeoTransform,
        level: f64,
    ) -> Result<Vec<Vec<Coordinate>>> {
        let mut contours = Vec::new();

        // Simplified marching squares algorithm
        for y in 0..band.height - 1 {
            for x in 0..band.width - 1 {
                let v00 = band.get_pixel(x, y).ok().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let v10 = band.get_pixel(x + 1, y).ok().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let v01 = band.get_pixel(x, y + 1).ok().and_then(|v| v.to_f64()).unwrap_or(0.0);
                let v11 = band.get_pixel(x + 1, y + 1).ok().and_then(|v| v.to_f64()).unwrap_or(0.0);

                // Determine marching squares case
                let mut case = 0;
                if v00 >= level {
                    case |= 1;
                }
                if v10 >= level {
                    case |= 2;
                }
                if v11 >= level {
                    case |= 4;
                }
                if v01 >= level {
                    case |= 8;
                }

                // Extract line segments based on case (simplified)
                if case != 0 && case != 15 {
                    let c1 = geo_transform.pixel_to_world(x as f64, y as f64);
                    let c2 = geo_transform.pixel_to_world((x + 1) as f64, (y + 1) as f64);
                    contours.push(vec![c1, c2]);
                }
            }
        }

        Ok(contours)
    }
}

/// Raster pyramid for multi-resolution support
pub struct RasterPyramid {
    pub levels: Vec<Raster>,
}

impl RasterPyramid {
    /// Build pyramid from base raster
    pub fn build(base: &Raster, num_levels: usize) -> Result<Self> {
        let mut levels = vec![base.clone()];

        for level in 1..num_levels {
            let prev = &levels[level - 1];
            let downsampled = Self::downsample(prev)?;
            levels.push(downsampled);
        }

        Ok(Self { levels })
    }

    /// Downsample raster by factor of 2
    fn downsample(raster: &Raster) -> Result<Raster> {
        let new_width = raster.width / 2;
        let new_height = raster.height / 2;

        if new_width == 0 || new_height == 0 {
            return Err(DbError::InvalidInput("Raster too small to downsample".to_string()));
        }

        let mut geo_transform = raster.geo_transform.clone();
        geo_transform.pixel_width *= 2.0;
        geo_transform.pixel_height *= 2.0;

        let mut result = Raster::new(
            new_width,
            new_height,
            raster.bands.len(),
            PixelType::Float32,
            geo_transform,
        );

        for band_idx in 0..raster.bands.len() {
            for y in 0..new_height {
                for x in 0..new_width {
                    // Average 2x2 pixels
                    let mut sum = 0.0;
                    let mut count = 0;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let src_x = x * 2 + dx;
                            let src_y = y * 2 + dy;

                            if let Some(val) = raster.bands[band_idx]
                                .get_pixel(src_x, src_y)
                                .ok()
                                .and_then(|v| v.to_f64())
                            {
                                sum += val;
                                count += 1;
                            }
                        }
                    }

                    let avg = if count > 0 {
                        PixelValue::Float32((sum / count as f64) as f32)
                    } else {
                        PixelValue::NoData
                    };

                    result.bands[band_idx].set_pixel(x, y, avg)?;
                }
            }
        }

        Ok(result)
    }

    /// Get appropriate level for given scale
    pub fn get_level(&self, scale: f64) -> &Raster {
        let level_idx = (scale.log2().floor() as usize).min(self.levels.len() - 1);
        &self.levels[level_idx]
    }
}

/// Tile-based raster storage for large datasets
pub struct TiledRaster {
    pub tile_width: usize,
    pub tile_height: usize,
    pub tiles: HashMap<(usize, usize), Raster>,
    pub full_width: usize,
    pub full_height: usize,
    pub geo_transform: GeoTransform,
}

impl TiledRaster {
    pub fn new(
        full_width: usize,
        full_height: usize,
        tile_width: usize,
        tile_height: usize,
        geo_transform: GeoTransform,
    ) -> Self {
        Self {
            tile_width,
            tile_height,
            tiles: HashMap::new(),
            full_width,
            full_height,
            geo_transform,
        }
    }

    /// Get tile coordinates for a pixel
    fn get_tile_coords(&self, x: usize, y: usize) -> (usize, usize) {
        (x / self.tile_width, y / self.tile_height)
    }

    /// Get or create a tile
    pub fn get_tile_mut(&mut self, tile_x: usize, tile_y: usize) -> &mut Raster {
        self.tiles.entry((tile_x, tile_y)).or_insert_with(|| {
            let width = self.tile_width.min(self.full_width - tile_x * self.tile_width);
            let height = self.tile_height.min(self.full_height - tile_y * self.tile_height);

            Raster::new(width, height, 1, PixelType::UInt8, self.geo_transform.clone())
        })
    }

    /// Get pixel value across tiles
    pub fn get_pixel(&self, x: usize, y: usize) -> Result<PixelValue> {
        let (tile_x, tile_y) = self.get_tile_coords(x, y);

        if let Some(tile) = self.tiles.get(&(tile_x, tile_y)) {
            let local_x = x % self.tile_width;
            let local_y = y % self.tile_height;
            tile.bands[0].get_pixel(local_x, local_y)
        } else {
            Ok(PixelValue::NoData)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raster_creation() {
        let geo_transform = GeoTransform::new(0.0, 100.0, 1.0, -1.0);
        let raster = Raster::new(100, 100, 1, PixelType::UInt8, geo_transform);

        assert_eq!(raster.width, 100);
        assert_eq!(raster.height, 100);
        assert_eq!(raster.bands.len(), 1);
    }

    #[test]
    fn test_pixel_operations() {
        let mut band = RasterBand::new(10, 10, PixelType::UInt8);

        band.set_pixel(5, 5, PixelValue::UInt8(255)).unwrap();
        let _value = band.get_pixel(5, 5).unwrap();

        assert_eq!(value, PixelValue::UInt8(255));
    }

    #[test]
    fn test_geo_transform() {
        let geo_transform = GeoTransform::new(100.0, 200.0, 1.0, -1.0);

        let world = geo_transform.pixel_to_world(10.0, 20.0);
        assert_eq!(world.x, 110.0);
        assert_eq!(world.y, 180.0);

        let (px, py) = geo_transform.world_to_pixel(110.0, 180.0);
        assert!((px - 10.0).abs() < 1e-10);
        assert!((py - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_raster_algebra() {
        let geo_transform = GeoTransform::new(0.0, 0.0, 1.0, 1.0);
        let mut raster1 = Raster::new(2, 2, 1, PixelType::UInt8, geo_transform.clone());
        let mut raster2 = Raster::new(2, 2, 1, PixelType::UInt8, geo_transform);

        raster1.bands[0].set_pixel(0, 0, PixelValue::UInt8(10)).unwrap();
        raster2.bands[0].set_pixel(0, 0, PixelValue::UInt8(5)).unwrap();

        let _result = RasterAlgebra::add(&raster1, &raster2).unwrap();
        let _value = result.bands[0].get_pixel(0, 0).unwrap();

        assert_eq!(value.to_f64(), Some(15.0));
    }
}


