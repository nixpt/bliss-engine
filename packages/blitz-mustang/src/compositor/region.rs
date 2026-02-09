//! Region types for Mustang
//!
//! Copyright (c) 2026 The Exosphere Authors
//!
//! Dual-licensed under MIT or Apache-2.0.
//!
/// Region bounds in screen coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Region {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this region
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Get the area of this region
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Expand region by padding on all sides
    pub fn expand(&self, padding: f32) -> Self {
        Self {
            x: self.x - padding,
            y: self.y - padding,
            width: self.width + padding * 2.0,
            height: self.height + padding * 2.0,
        }
    }

    /// Contract region by padding on all sides
    pub fn contract(&self, padding: f32) -> Self {
        Self {
            x: self.x + padding,
            y: self.y + padding,
            width: (self.width - padding * 2.0).max(0.0),
            height: (self.height - padding * 2.0).max(0.0),
        }
    }

    /// Get the center point of this region
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Create a region from a center point and size
    pub fn from_center(center_x: f32, center_y: f32, width: f32, height: f32) -> Self {
        Self {
            x: center_x - width / 2.0,
            y: center_y - height / 2.0,
            width,
            height,
        }
    }

    /// Check if this region intersects with another
    pub fn intersects(&self, other: &Region) -> bool {
        !(self.x >= other.x + other.width
            || self.x + self.width <= other.x
            || self.y >= other.y + other.height
            || self.y + self.height <= other.y)
    }

    /// Get the intersection of two regions
    pub fn intersection(&self, other: &Region) -> Option<Region> {
        if !self.intersects(other) {
            return None;
        }

        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        Some(Region::new(x1, y1, x2 - x1, y2 - y1))
    }

    /// Get the union of two regions (bounding box)
    pub fn union(&self, other: &Region) -> Region {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);

        Region::new(x1, y1, x2 - x1, y2 - y1)
    }

    /// Convert to kurbo::Rect for Vello operations
    #[cfg(feature = "gpu")]
    pub fn to_rect(&self) -> kurbo::Rect {
        kurbo::Rect::new(
            self.x as f64,
            self.y as f64,
            (self.x + self.width) as f64,
            (self.y + self.height) as f64,
        )
    }

    /// Create from kurbo::Rect for Vello operations
    #[cfg(feature = "gpu")]
    pub fn from_rect(rect: &kurbo::Rect) -> Self {
        Self {
            x: rect.x0 as f32,
            y: rect.y0 as f32,
            width: (rect.x1 - rect.x0) as f32,
            height: (rect.y1 - rect.y0) as f32,
        }
    }
}

impl Default for Region {
    fn default() -> Self {
        Self::new(0.0, 0.0, 100.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_new() {
        let region = Region::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(region.x, 10.0);
        assert_eq!(region.y, 20.0);
        assert_eq!(region.width, 100.0);
        assert_eq!(region.height, 50.0);
    }

    #[test]
    fn test_region_contains() {
        let region = Region::new(0.0, 0.0, 100.0, 100.0);
        assert!(region.contains(50.0, 50.0));
        assert!(region.contains(0.0, 0.0));
        assert!(region.contains(99.0, 99.0));
        assert!(!region.contains(100.0, 50.0));
        assert!(!region.contains(50.0, 100.0));
        assert!(!region.contains(-1.0, 50.0));
    }

    #[test]
    fn test_region_area() {
        let region = Region::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(region.area(), 5000.0);
    }

    #[test]
    fn test_region_expand() {
        let region = Region::new(10.0, 20.0, 100.0, 50.0);
        let expanded = region.expand(5.0);
        assert_eq!(expanded.x, 5.0);
        assert_eq!(expanded.y, 15.0);
        assert_eq!(expanded.width, 110.0);
        assert_eq!(expanded.height, 60.0);
    }

    #[test]
    fn test_region_contract() {
        let region = Region::new(10.0, 20.0, 100.0, 50.0);
        let contracted = region.contract(5.0);
        assert_eq!(contracted.x, 15.0);
        assert_eq!(contracted.y, 25.0);
        assert_eq!(contracted.width, 90.0);
        assert_eq!(contracted.height, 40.0);
    }

    #[test]
    fn test_region_center() {
        let region = Region::new(0.0, 0.0, 100.0, 50.0);
        let center = region.center();
        assert_eq!(center, (50.0, 25.0));
    }

    #[test]
    fn test_region_from_center() {
        let region = Region::from_center(50.0, 25.0, 100.0, 50.0);
        assert_eq!(region.x, 0.0);
        assert_eq!(region.y, 0.0);
        assert_eq!(region.width, 100.0);
        assert_eq!(region.height, 50.0);
    }

    #[test]
    fn test_region_intersects() {
        let region1 = Region::new(0.0, 0.0, 100.0, 100.0);
        let region2 = Region::new(50.0, 50.0, 100.0, 100.0);
        let region3 = Region::new(200.0, 200.0, 100.0, 100.0);

        assert!(region1.intersects(&region2));
        assert!(!region1.intersects(&region3));
    }

    #[test]
    fn test_region_intersection() {
        let region1 = Region::new(0.0, 0.0, 100.0, 100.0);
        let region2 = Region::new(50.0, 50.0, 100.0, 100.0);

        let intersection = region1.intersection(&region2).unwrap();
        assert_eq!(intersection.x, 50.0);
        assert_eq!(intersection.y, 50.0);
        assert_eq!(intersection.width, 50.0);
        assert_eq!(intersection.height, 50.0);
    }

    #[test]
    fn test_region_union() {
        let region1 = Region::new(0.0, 0.0, 100.0, 100.0);
        let region2 = Region::new(50.0, 50.0, 100.0, 100.0);

        let union = region1.union(&region2);
        assert_eq!(union.x, 0.0);
        assert_eq!(union.y, 0.0);
        assert_eq!(union.width, 150.0);
        assert_eq!(union.height, 150.0);
    }
}
