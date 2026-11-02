//! Axis-Aligned Bounding Box (AABB) 3D collision volume.

use glam::DVec3;

/// 3D Axis-Aligned Bounding Box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Minimum coordinates (x_min, y_min, z_min).
    pub min: DVec3,
    /// Maximum coordinates (x_max, y_max, z_max).
    pub max: DVec3,
}

impl Aabb {
    /// Creates a new AABB from min and max bounds.
    #[inline]
    pub const fn new(min: DVec3, max: DVec3) -> Self {
        Self { min, max }
    }

    /// Creates a standard 1x1x1 cube AABB at integer block coordinates.
    #[inline]
    pub fn from_block(x: i32, y: i32, z: i32) -> Self {
        let min = DVec3::new(x as f64, y as f64, z as f64);
        let max = min + DVec3::ONE;
        Self { min, max }
    }

    /// Creates a player bounding box centered horizontally at (x, z) with feet at y.
    ///
    /// In Minecraft 1.8.9:
    /// - Player width = 0.6 blocks (radius = 0.3)
    /// - Player height = 1.8 blocks
    #[inline]
    pub fn player_box(pos: DVec3) -> Self {
        let half_w = 0.3;
        let height = 1.8;
        Self {
            min: DVec3::new(pos.x - half_w, pos.y, pos.z - half_w),
            max: DVec3::new(pos.x + half_w, pos.y + height, pos.z + half_w),
        }
    }

    /// Checks if this AABB intersects with another AABB.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }

    /// Computes offset along X axis against another box to prevent penetration.
    pub fn calculate_x_offset(&self, other: &Self, mut offset: f64) -> f64 {
        if other.max.y > self.min.y
            && other.min.y < self.max.y
            && other.max.z > self.min.z
            && other.min.z < self.max.z
        {
            if offset > 0.0 && other.max.x <= self.min.x {
                let diff = self.min.x - other.max.x;
                if diff < offset {
                    offset = diff;
                }
            } else if offset < 0.0 && other.min.x >= self.max.x {
                let diff = self.max.x - other.min.x;
                if diff > offset {
                    offset = diff;
                }
            }
        }
        offset
    }

    /// Computes offset along Y axis against another box to prevent penetration.
    pub fn calculate_y_offset(&self, other: &Self, mut offset: f64) -> f64 {
        if other.max.x > self.min.x
            && other.min.x < self.max.x
            && other.max.z > self.min.z
            && other.min.z < self.max.z
        {
            if offset > 0.0 && other.max.y <= self.min.y {
                let diff = self.min.y - other.max.y;
                if diff < offset {
                    offset = diff;
                }
            } else if offset < 0.0 && other.min.y >= self.max.y {
                let diff = self.max.y - other.min.y;
                if diff > offset {
                    offset = diff;
                }
            }
        }
        offset
    }

    /// Computes offset along Z axis against another box to prevent penetration.
    pub fn calculate_z_offset(&self, other: &Self, mut offset: f64) -> f64 {
        if other.max.x > self.min.x
            && other.min.x < self.max.x
            && other.max.y > self.min.y
            && other.min.y < self.max.y
        {
            if offset > 0.0 && other.max.z <= self.min.z {
                let diff = self.min.z - other.max.z;
                if diff < offset {
                    offset = diff;
                }
            } else if offset < 0.0 && other.min.z >= self.max.z {
                let diff = self.max.z - other.min.z;
                if diff > offset {
                    offset = diff;
                }
            }
        }
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_box_dimensions() {
        let pos = DVec3::new(10.0, 64.0, 10.0);
        let aabb = Aabb::player_box(pos);

        assert_eq!(aabb.min.x, 9.7);
        assert_eq!(aabb.max.x, 10.3);
        assert_eq!(aabb.min.y, 64.0);
        assert_eq!(aabb.max.y, 65.8);
    }

    #[test]
    fn intersecting_boxes() {
        let box1 = Aabb::from_block(0, 0, 0);
        let box2 = Aabb::new(DVec3::new(0.5, 0.5, 0.5), DVec3::new(1.5, 1.5, 1.5));
        assert!(box1.intersects(&box2));

        let box3 = Aabb::from_block(2, 0, 0);
        assert!(!box1.intersects(&box3));
    }
}
