//! Bezier curve manipulation and editing tools.
//!
//! Provides advanced bezier curve operations including point editing,
//! curve conversion, path simplification, smoothing, and length
//! calculation. Operates on [`kurbo`] path types, which are useful when
//! inspecting or editing glyph outlines extracted from a font.

use kurbo::{BezPath, CubicBez, PathEl, Point, QuadBez, Vec2};

/// Bezier curve editor with grid snapping and point selection.
pub struct BezierEditor {
    selected_points: Vec<usize>,
    snap_to_grid: bool,
    grid_size: f64,
}

impl BezierEditor {
    /// Create a new editor with grid snapping enabled and a 1.0 grid size.
    pub fn new() -> Self {
        Self {
            selected_points: Vec::new(),
            snap_to_grid: true,
            grid_size: 1.0,
        }
    }

    /// Enable or disable grid snapping.
    pub fn set_snap_to_grid(&mut self, snap: bool) {
        self.snap_to_grid = snap;
    }

    /// Set the grid size used when snapping.
    pub fn set_grid_size(&mut self, size: f64) {
        self.grid_size = size;
    }

    /// Current grid size.
    pub fn grid_size(&self) -> f64 {
        self.grid_size
    }

    /// Whether grid snapping is enabled.
    pub fn is_snap_to_grid(&self) -> bool {
        self.snap_to_grid
    }

    /// Select a point by index (idempotent).
    pub fn select_point(&mut self, index: usize) {
        if !self.selected_points.contains(&index) {
            self.selected_points.push(index);
        }
    }

    /// Deselect a point by index.
    pub fn deselect_point(&mut self, index: usize) {
        self.selected_points.retain(|&i| i != index);
    }

    /// Clear all selected points.
    pub fn clear_selection(&mut self) {
        self.selected_points.clear();
    }

    /// Get the currently selected point indices.
    pub fn selected_points(&self) -> &[usize] {
        &self.selected_points
    }

    /// Move all selected points by `delta`, snapping to the grid when enabled.
    pub fn move_selected_points(&mut self, delta: Vec2, points: &mut [Point]) {
        for &index in &self.selected_points {
            if index < points.len() {
                points[index] = Point::new(
                    points[index].x + delta.x,
                    points[index].y + delta.y,
                );
                if self.snap_to_grid {
                    points[index] = self.snap_point_to_grid(points[index]);
                }
            }
        }
    }

    fn snap_point_to_grid(&self, point: Point) -> Point {
        if self.snap_to_grid {
            Point::new(
                (point.x / self.grid_size).round() * self.grid_size,
                (point.y / self.grid_size).round() * self.grid_size,
            )
        } else {
            point
        }
    }

    /// Convert a quadratic bezier segment into the cubic representation.
    pub fn quad_to_cubic(quad: QuadBez) -> CubicBez {
        let p0 = quad.p0;
        let p1 = quad.p1;
        let p2 = quad.p2;
        let cp1 = p0 + (p1 - p0) * (2.0 / 3.0);
        let cp2 = p2 + (p1 - p2) * (2.0 / 3.0);
        CubicBez::new(p0, cp1, cp2, p2)
    }

    /// Approximate a cubic bezier segment as a quadratic one.
    pub fn cubic_to_quad(cubic: CubicBez) -> QuadBez {
        let cp = Point::new(
            (cubic.p1.x + cubic.p2.x) * 0.5,
            (cubic.p1.y + cubic.p2.y) * 0.5,
        );
        QuadBez::new(cubic.p0, cp, cubic.p3)
    }

    /// Remove collinear line-to segments from a path within `tolerance`.
    pub fn simplify_path(path: &BezPath, tolerance: f64) -> BezPath {
        let mut simplified = BezPath::new();
        let elements: Vec<PathEl> = path.elements().to_vec();
        if elements.is_empty() {
            return simplified;
        }

        let last_point = |p: &BezPath| -> Option<Point> {
            p.elements().last().and_then(|e| match e {
                PathEl::MoveTo(pt) | PathEl::LineTo(pt) => Some(*pt),
                PathEl::QuadTo(_, pt) | PathEl::CurveTo(_, _, pt) => Some(*pt),
                PathEl::ClosePath => None,
            })
        };

        let mut i = 0;
        while i < elements.len() {
            match elements[i] {
                PathEl::MoveTo(p) => simplified.move_to(p),
                PathEl::LineTo(p) => {
                    if i + 1 < elements.len()
                        && let PathEl::LineTo(next_p) = elements[i + 1] {
                            let anchor = last_point(&simplified).unwrap_or(Point::ZERO);
                            if Self::are_collinear(anchor, p, next_p, tolerance) {
                                i += 1;
                                continue;
                            }
                        }
                    simplified.line_to(p);
                }
                PathEl::QuadTo(p1, p2) => simplified.quad_to(p1, p2),
                PathEl::CurveTo(p1, p2, p3) => simplified.curve_to(p1, p2, p3),
                PathEl::ClosePath => simplified.close_path(),
            }
            i += 1;
        }
        simplified
    }

    fn are_collinear(p1: Point, p2: Point, p3: Point, tolerance: f64) -> bool {
        let v1 = Vec2::new(p2.x - p1.x, p2.y - p1.y);
        let v2 = Vec2::new(p3.x - p2.x, p3.y - p2.y);
        let cross = v1.x * v2.y - v1.y * v2.x;
        cross.abs() < tolerance
    }

    /// Smooth every selected point toward the average of its neighbours.
    pub fn smooth_points(&mut self, points: &mut [Point]) {
        for &index in &self.selected_points {
            if index < points.len() {
                self.smooth_point(index, points);
            }
        }
    }

    fn smooth_point(&self, index: usize, points: &mut [Point]) {
        if points.len() < 3 {
            return;
        }
        let prev_index = if index == 0 { points.len() - 1 } else { index - 1 };
        let next_index = if index == points.len() - 1 { 0 } else { index + 1 };
        let prev = points[prev_index];
        let next = points[next_index];
        points[index] = Point::new((prev.x + next.x) / 2.0, (prev.y + next.y) / 2.0);
    }

    /// Sum of straight and approximate curve segment lengths along a path.
    pub fn path_length(path: &BezPath) -> f64 {
        let mut length = 0.0;
        let mut current = Point::ZERO;
        for el in path.elements() {
            match el {
                PathEl::MoveTo(p) => current = *p,
                PathEl::LineTo(p) => {
                    length += Self::distance(current, *p);
                    current = *p;
                }
                PathEl::QuadTo(p1, p2) => {
                    length += Self::quad_length(&QuadBez::new(current, *p1, *p2));
                    current = *p2;
                }
                PathEl::CurveTo(p1, p2, p3) => {
                    length += Self::cubic_length(&CubicBez::new(current, *p1, *p2, *p3));
                    current = *p3;
                }
                PathEl::ClosePath => {}
            }
        }
        length
    }

    fn distance(p1: Point, p2: Point) -> f64 {
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn quad_length(quad: &QuadBez) -> f64 {
        let steps = 10;
        let mut length = 0.0;
        let mut prev = quad.p0;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let t2 = t * t;
            let mt = 1.0 - t;
            let mt2 = mt * mt;
            let current = Point::new(
                mt2 * quad.p0.x + 2.0 * mt * t * quad.p1.x + t2 * quad.p2.x,
                mt2 * quad.p0.y + 2.0 * mt * t * quad.p1.y + t2 * quad.p2.y,
            );
            length += Self::distance(prev, current);
            prev = current;
        }
        length
    }

    fn cubic_length(cubic: &CubicBez) -> f64 {
        let steps = 10;
        let mut length = 0.0;
        let mut prev = cubic.p0;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let t2 = t * t;
            let t3 = t2 * t;
            let mt = 1.0 - t;
            let mt2 = mt * mt;
            let mt3 = mt2 * mt;
            let current = Point::new(
                mt3 * cubic.p0.x
                    + 3.0 * mt2 * t * cubic.p1.x
                    + 3.0 * mt * t2 * cubic.p2.x
                    + t3 * cubic.p3.x,
                mt3 * cubic.p0.y
                    + 3.0 * mt2 * t * cubic.p1.y
                    + 3.0 * mt * t2 * cubic.p2.y
                    + t3 * cubic.p3.y,
            );
            length += Self::distance(prev, current);
            prev = current;
        }
        length
    }
}

impl Default for BezierEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_editor_creation() {
        let editor = BezierEditor::new();
        assert!(editor.is_snap_to_grid());
        assert_eq!(editor.grid_size(), 1.0);
    }

    #[test]
    fn test_point_selection() {
        let mut editor = BezierEditor::new();
        editor.select_point(0);
        editor.select_point(1);
        assert_eq!(editor.selected_points().len(), 2);
        editor.deselect_point(0);
        assert_eq!(editor.selected_points().len(), 1);
        editor.clear_selection();
        assert!(editor.selected_points().is_empty());
    }

    #[test]
    fn test_quad_to_cubic_conversion() {
        let quad = QuadBez::new(Point::new(0.0, 0.0), Point::new(50.0, 100.0), Point::new(100.0, 0.0));
        let cubic = BezierEditor::quad_to_cubic(quad);
        assert_eq!(cubic.p0, quad.p0);
        assert_eq!(cubic.p3, quad.p2);
    }

    #[test]
    fn test_path_simplification() {
        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 0.0));
        path.line_to(Point::new(20.0, 0.0));
        path.close_path();
        let simplified = BezierEditor::simplify_path(&path, 0.1);
        assert!(simplified.elements().len() <= path.elements().len());
    }

    #[test]
    fn test_distance_calculation() {
        let d = BezierEditor::distance(Point::new(0.0, 0.0), Point::new(3.0, 4.0));
        assert!((d - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_path_length() {
        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(3.0, 4.0));
        assert!((BezierEditor::path_length(&path) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_move_selected_points_snaps_to_grid() {
        let mut editor = BezierEditor::new();
        editor.set_grid_size(10.0);
        editor.select_point(0);
        let mut pts = [Point::new(1.0, 1.0)];
        // (1,1) + (14,14) = (15,15), which rounds to the 10-grid as (20,20).
        editor.move_selected_points(Vec2::new(14.0, 14.0), &mut pts);
        assert_eq!(pts[0], Point::new(20.0, 20.0));
    }
}
