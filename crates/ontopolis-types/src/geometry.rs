use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ChunkCoord, LocalFrameId, SpatialChartId, WorldGeometrySchemaId};

pub const MAX_SURFACE_CHARTS: usize = 256;

/// Opaque global geometry contract plus its canonical surface-chart atlas.
///
/// The schema resolves topology, chart seams, curvature, and global metric in
/// a versioned registry. Core state does not use a semantic topology enum.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldGeometryContract {
    schema: WorldGeometrySchemaId,
    charts: Vec<SpatialChartId>,
}

impl WorldGeometryContract {
    pub fn new(
        schema: WorldGeometrySchemaId,
        mut charts: Vec<SpatialChartId>,
    ) -> Result<Self, GeometryError> {
        if charts.is_empty() || charts.len() > MAX_SURFACE_CHARTS {
            return Err(GeometryError::InvalidChartCount {
                count: charts.len(),
            });
        }
        charts.sort_unstable();
        if charts.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(GeometryError::DuplicateChart);
        }
        Ok(Self { schema, charts })
    }

    pub const fn schema(&self) -> WorldGeometrySchemaId {
        self.schema
    }

    pub fn charts(&self) -> &[SpatialChartId] {
        &self.charts
    }
}

/// Fixed-point position on a two-dimensional geographic chart plus elevation
/// along that chart's local surface normal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SurfacePositionMm {
    pub chart: SpatialChartId,
    pub u_mm: i64,
    pub v_mm: i64,
    pub elevation_mm: i64,
}

impl SurfacePositionMm {
    pub const fn new(chart: SpatialChartId, u_mm: i64, v_mm: i64, elevation_mm: i64) -> Self {
        Self {
            chart,
            u_mm,
            v_mm,
            elevation_mm,
        }
    }
}

/// A chart-qualified chunk address. Bare `ChunkCoord` remains local-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChartChunkCoord {
    pub chart: SpatialChartId,
    pub chunk: ChunkCoord,
}

impl ChartChunkCoord {
    pub const fn new(chart: SpatialChartId, chunk: ChunkCoord) -> Self {
        Self { chart, chunk }
    }
}

/// Millimetre position in a bounded local three-dimensional Euclidean frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocalPointMm {
    pub frame: LocalFrameId,
    pub x_mm: i64,
    pub y_mm: i64,
    pub z_mm: i64,
}

impl LocalPointMm {
    pub const fn new(frame: LocalFrameId, x_mm: i64, y_mm: i64, z_mm: i64) -> Self {
        Self {
            frame,
            x_mm,
            y_mm,
            z_mm,
        }
    }
}

/// Bounded tangent-frame transform between local 3D physics and one surface
/// chart. `x/y` follow chart tangents; `z` follows the local surface normal.
/// Negative `z` represents depth below the anchor surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalMetricFrame {
    id: LocalFrameId,
    origin: SurfacePositionMm,
    maximum_offset_mm: i64,
}

impl LocalMetricFrame {
    pub fn new(
        id: LocalFrameId,
        origin: SurfacePositionMm,
        maximum_offset_mm: i64,
    ) -> Result<Self, GeometryError> {
        if maximum_offset_mm <= 0 {
            return Err(GeometryError::InvalidFrameExtent);
        }
        Ok(Self {
            id,
            origin,
            maximum_offset_mm,
        })
    }

    pub const fn id(self) -> LocalFrameId {
        self.id
    }

    pub const fn origin(self) -> SurfacePositionMm {
        self.origin
    }

    pub fn to_surface(self, point: LocalPointMm) -> Result<SurfacePositionMm, GeometryError> {
        if point.frame != self.id {
            return Err(GeometryError::FrameMismatch);
        }
        self.validate_offsets(point.x_mm, point.y_mm, point.z_mm)?;
        Ok(SurfacePositionMm {
            chart: self.origin.chart,
            u_mm: self
                .origin
                .u_mm
                .checked_add(point.x_mm)
                .ok_or(GeometryError::CoordinateOverflow)?,
            v_mm: self
                .origin
                .v_mm
                .checked_add(point.y_mm)
                .ok_or(GeometryError::CoordinateOverflow)?,
            elevation_mm: self
                .origin
                .elevation_mm
                .checked_add(point.z_mm)
                .ok_or(GeometryError::CoordinateOverflow)?,
        })
    }

    pub fn to_local(self, position: SurfacePositionMm) -> Result<LocalPointMm, GeometryError> {
        if position.chart != self.origin.chart {
            return Err(GeometryError::ChartMismatch);
        }
        let x_mm = position
            .u_mm
            .checked_sub(self.origin.u_mm)
            .ok_or(GeometryError::CoordinateOverflow)?;
        let y_mm = position
            .v_mm
            .checked_sub(self.origin.v_mm)
            .ok_or(GeometryError::CoordinateOverflow)?;
        let z_mm = position
            .elevation_mm
            .checked_sub(self.origin.elevation_mm)
            .ok_or(GeometryError::CoordinateOverflow)?;
        self.validate_offsets(x_mm, y_mm, z_mm)?;
        Ok(LocalPointMm::new(self.id, x_mm, y_mm, z_mm))
    }

    fn validate_offsets(self, x_mm: i64, y_mm: i64, z_mm: i64) -> Result<(), GeometryError> {
        let maximum = self.maximum_offset_mm as u64;
        if [x_mm, y_mm, z_mm]
            .into_iter()
            .any(|value| value.unsigned_abs() > maximum)
        {
            return Err(GeometryError::OutsideLocalFrame);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum GeometryError {
    #[error("invalid surface chart count: {count}")]
    InvalidChartCount { count: usize },
    #[error("surface chart atlas contains a duplicate chart")]
    DuplicateChart,
    #[error("local metric frame extent must be positive")]
    InvalidFrameExtent,
    #[error("local point belongs to a different metric frame")]
    FrameMismatch,
    #[error("surface position belongs to a different chart")]
    ChartMismatch,
    #[error("coordinate lies outside the bounded local metric frame")]
    OutsideLocalFrame,
    #[error("fixed-point coordinate arithmetic overflow")]
    CoordinateOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_three_dimensional_transform_is_exact_and_includes_depth() {
        let frame = LocalMetricFrame::new(
            LocalFrameId::new(4),
            SurfacePositionMm::new(SpatialChartId::new(2), 1_000, 2_000, 300),
            10_000,
        )
        .unwrap();
        let local = LocalPointMm::new(LocalFrameId::new(4), 50, -75, -120);
        let surface = frame.to_surface(local).unwrap();
        assert_eq!(
            surface,
            SurfacePositionMm::new(SpatialChartId::new(2), 1_050, 1_925, 180)
        );
        assert_eq!(frame.to_local(surface).unwrap(), local);
    }

    #[test]
    fn chart_and_local_frame_boundaries_are_explicit() {
        let frame = LocalMetricFrame::new(
            LocalFrameId::new(1),
            SurfacePositionMm::new(SpatialChartId::new(1), 0, 0, 0),
            100,
        )
        .unwrap();
        assert_eq!(
            frame.to_surface(LocalPointMm::new(LocalFrameId::new(1), 101, 0, 0)),
            Err(GeometryError::OutsideLocalFrame)
        );
        assert_eq!(
            frame.to_local(SurfacePositionMm::new(SpatialChartId::new(2), 0, 0, 0)),
            Err(GeometryError::ChartMismatch)
        );
    }
}
