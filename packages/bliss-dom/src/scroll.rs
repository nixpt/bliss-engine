//! Scroll-snap target computation (BLISS-SCROLL-1).
//!
//! A scroll container can declare snap offsets per axis (in scroll-offset
//! space); when a scroll gesture settles the container snaps to the nearest.
//! Declared with [`BaseDocument::set_scroll_snap`].
//!
//! The points are plain offsets (the caller already resolved whatever
//! `start`/`center`/`end` alignment it wanted), so this stays a pure target
//! computation. Wiring CSS `scroll-snap-type` / `scroll-snap-align` to these
//! points is a follow-up.

/// Snap offsets for a scroll container, one list per axis.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapPoints {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// The snap offset in `points` nearest to `offset`, or `None` when there are
/// none. Ties keep the earlier point.
pub fn nearest_snap(offset: f64, points: &[f64]) -> Option<f64> {
    points.iter().copied().min_by(|a, b| (a - offset).abs().total_cmp(&(b - offset).abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_nearest() {
        let points = [0.0, 100.0, 200.0];
        assert_eq!(nearest_snap(10.0, &points), Some(0.0));
        assert_eq!(nearest_snap(90.0, &points), Some(100.0));
        assert_eq!(nearest_snap(151.0, &points), Some(200.0));
    }

    #[test]
    fn empty_and_single() {
        assert_eq!(nearest_snap(5.0, &[]), None);
        assert_eq!(nearest_snap(5.0, &[42.0]), Some(42.0));
    }

    #[test]
    fn ties_keep_the_earlier_point() {
        assert_eq!(nearest_snap(50.0, &[0.0, 100.0]), Some(0.0));
    }

    #[test]
    fn settled_container_snaps_to_the_nearest_offset() {
        // The snap *decision* (which offset the container would snap to), read
        // without depending on headless scroll-extent geometry. Applying it goes
        // through `scroll_by`, whose clamping is engine behaviour.
        use crate::{BaseDocument, DocumentConfig, qual_name};

        let mut doc = BaseDocument::new(DocumentConfig::default());
        let mut mutator = doc.mutate();
        let container = mutator.create_element(qual_name!("div"), vec![]);
        mutator.append_children(0, &[container]);
        drop(mutator);

        assert_eq!(doc.scroll_snap_target(container), None, "no points -> no target");

        doc.nodes[container].scroll_offset.y = 137.0;
        doc.set_scroll_snap(
            container,
            SnapPoints {
                x: Vec::new(),
                y: vec![0.0, 100.0, 200.0],
            },
        );
        assert_eq!(doc.scroll_snap_target(container), Some((0.0, 100.0)));

        doc.clear_scroll_snap(container);
        assert_eq!(doc.scroll_snap_target(container), None);
    }
}
