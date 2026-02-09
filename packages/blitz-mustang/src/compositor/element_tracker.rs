//! Element position tracking for region-specific effects
//!
//! Copyright (c) 2026 The Exosphere Authors
//!
//! Dual-licensed under MIT or Apache-2.0.
//!
//! Tracks DOM element positions to apply compositor effects to specific screen regions.
//! Used for effects like backdrop-filter that need to know element boundaries.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use super::region::Region;

/// Tracked element with position and metadata
#[derive(Debug, Clone)]
pub struct TrackedElement {
    /// Unique element identifier
    pub id: String,
    /// Screen region (x, y, width, height)
    pub region: Region,
    /// Element type (div, button, etc.)
    pub element_type: String,
    /// CSS classes
    pub classes: Vec<String>,
    /// Last updated timestamp
    pub last_updated: std::time::Instant,
}

impl TrackedElement {
    /// Create a new tracked element
    pub fn new(id: String, region: Region, element_type: String) -> Self {
        Self {
            id,
            region,
            element_type,
            classes: Vec::new(),
            last_updated: std::time::Instant::now(),
        }
    }

    /// Add a CSS class
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.classes.push(class.into());
        self
    }

    /// Update the element's region
    pub fn update_region(&mut self, region: Region) {
        self.region = region;
        self.last_updated = std::time::Instant::now();
    }
}

/// Thread-safe element tracker for compositor effects
///
/// Shared between the DOM integration and the compositor to track
/// element positions for region-specific effects.
#[derive(Debug, Clone)]
pub struct SharedElementTracker {
    elements: Arc<Mutex<HashMap<String, TrackedElement>>>,
}

impl SharedElementTracker {
    /// Create a new empty element tracker
    pub fn new() -> Self {
        Self {
            elements: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Track a new element
    pub fn track(&self, element: TrackedElement) {
        if let Ok(mut elements) = self.elements.lock() {
            elements.insert(element.id.clone(), element);
        }
    }

    /// Update an element's position
    pub fn update_position(&self, id: &str, region: Region) {
        if let Ok(mut elements) = self.elements.lock() {
            if let Some(element) = elements.get_mut(id) {
                element.update_region(region);
            }
        }
    }

    /// Get an element by ID
    pub fn get(&self, id: &str) -> Option<TrackedElement> {
        self.elements.lock().ok()?.get(id).cloned()
    }

    /// Get all elements matching a selector (simple class/ID matching)
    pub fn query(&self, selector: &str) -> Vec<TrackedElement> {
        let elements = match self.elements.lock() {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        elements
            .values()
            .filter(|e| {
                // Simple selector matching
                if selector.starts_with('#') {
                    e.id == &selector[1..]
                } else if selector.starts_with('.') {
                    e.classes.contains(&selector[1..].to_string())
                } else {
                    e.element_type == selector
                }
            })
            .cloned()
            .collect()
    }

    /// Get all elements within a region
    pub fn query_region(&self, query_region: &Region) -> Vec<TrackedElement> {
        let elements = match self.elements.lock() {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        elements
            .values()
            .filter(|e| e.region.intersects(query_region))
            .cloned()
            .collect()
    }

    /// Remove an element
    pub fn remove(&self, id: &str) {
        if let Ok(mut elements) = self.elements.lock() {
            elements.remove(id);
        }
    }

    /// Clear all tracked elements
    pub fn clear(&self) {
        if let Ok(mut elements) = self.elements.lock() {
            elements.clear();
        }
    }

    /// Get the number of tracked elements
    pub fn len(&self) -> usize {
        self.elements.lock().map(|e| e.len()).unwrap_or(0)
    }

    /// Check if no elements are tracked
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all element IDs
    pub fn ids(&self) -> Vec<String> {
        self.elements
            .lock()
            .map(|e| e.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Clean up stale elements (not updated for a while)
    pub fn cleanup_stale(&self, max_age: std::time::Duration) -> usize {
        let now = std::time::Instant::now();
        let to_remove: Vec<String> = {
            let elements = match self.elements.lock() {
                Ok(e) => e,
                Err(_) => return 0,
            };

            elements
                .iter()
                .filter(|(_, e)| now.duration_since(e.last_updated) > max_age)
                .map(|(id, _)| id.clone())
                .collect()
        };

        let count = to_remove.len();
        if let Ok(mut elements) = self.elements.lock() {
            for id in to_remove {
                elements.remove(&id);
            }
        }
        count
    }
}

impl Default for SharedElementTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_tracker_new() {
        let tracker = SharedElementTracker::new();
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_track_element() {
        let tracker = SharedElementTracker::new();
        let element = TrackedElement::new(
            "test-1".to_string(),
            Region::new(0.0, 0.0, 100.0, 100.0),
            "div".to_string(),
        );

        tracker.track(element);
        assert_eq!(tracker.len(), 1);

        let retrieved = tracker.get("test-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().element_type, "div");
    }

    #[test]
    fn test_update_position() {
        let tracker = SharedElementTracker::new();
        let element = TrackedElement::new(
            "test-1".to_string(),
            Region::new(0.0, 0.0, 100.0, 100.0),
            "div".to_string(),
        );

        tracker.track(element);
        tracker.update_position("test-1", Region::new(10.0, 10.0, 200.0, 200.0));

        let updated = tracker.get("test-1").unwrap();
        assert_eq!(updated.region.x, 10.0);
        assert_eq!(updated.region.width, 200.0);
    }

    #[test]
    fn test_query_by_class() {
        let tracker = SharedElementTracker::new();
        let element = TrackedElement::new(
            "test-1".to_string(),
            Region::new(0.0, 0.0, 100.0, 100.0),
            "div".to_string(),
        )
        .with_class("backdrop");

        tracker.track(element);

        let results = tracker.query(".backdrop");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_remove_element() {
        let tracker = SharedElementTracker::new();
        let element = TrackedElement::new(
            "test-1".to_string(),
            Region::new(0.0, 0.0, 100.0, 100.0),
            "div".to_string(),
        );

        tracker.track(element);
        tracker.remove("test-1");

        assert!(tracker.is_empty());
        assert!(tracker.get("test-1").is_none());
    }
}
