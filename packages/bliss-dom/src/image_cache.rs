//! Shared image cache for Bliss documents
//!
//! Provides a centralized cache for decoded images that can be shared
//! across multiple documents to reduce memory usage.
//!
//! This module re-exports the common image cache functionality from bliss-traits
//! and provides Bliss-DOM specific adapters.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::node::ImageData;
use crate::util::ImageType;

// Re-export the common cache functionality
pub use bliss_traits::image_cache::{
    create_default_image_cache, create_image_cache_with_limit, create_image_cache_with_limits,
    create_image_cache_with_ttl, ImageCache as TraitImageCache, LruImageCache, SharedImageCache,
};

/// Trait for image cache implementations that work with bliss-dom ImageData
pub trait DomImageCache: Send + Sync + 'static {
    /// Get an image from cache by URL
    fn get(&self, url: &str) -> Option<Arc<ImageData>>;

    /// Insert an image into cache
    fn insert(&self, url: String, image: ImageData);

    /// Evict an image from the cache
    fn evict(&self, url: &str);

    /// Get current memory usage in bytes (approximate)
    fn memory_usage(&self) -> usize;

    /// Clear all cached images
    fn clear(&self);
}

/// DOM image cache backed by a simple in-process HashMap.
///
/// Accepts a `SharedImageCache` at construction for API compatibility,
/// but maintains its own typed storage keyed on `bliss-dom::ImageData`.
pub struct DomImageCacheAdapter {
    cache: Mutex<HashMap<String, Arc<ImageData>>>,
    max_bytes: usize,
    current_bytes: Mutex<usize>,
}

impl DomImageCacheAdapter {
    /// Create a new adapter (the `SharedImageCache` arg is accepted for API compat but unused)
    pub fn new(_inner: SharedImageCache) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            max_bytes: 100 * 1024 * 1024, // 100MB default
            current_bytes: Mutex::new(0),
        }
    }

    /// Create with custom memory limit
    pub fn with_limit(_inner: SharedImageCache, max_bytes: usize) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            max_bytes,
            current_bytes: Mutex::new(0),
        }
    }
}

fn image_size(image: &ImageData) -> usize {
    match image {
        ImageData::Raster(raster) => raster.data.len(),
        #[cfg(feature = "svg")]
        ImageData::Svg(_) => 1024, // 1KB estimate
        ImageData::None => 0,
    }
}

impl DomImageCache for DomImageCacheAdapter {
    fn get(&self, url: &str) -> Option<Arc<ImageData>> {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.get(url).cloned()
    }

    fn insert(&self, url: String, image: ImageData) {
        let size = image_size(&image);

        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let mut current_bytes = self.current_bytes.lock().unwrap_or_else(|e| e.into_inner());

        // Remove old entry if exists
        if let Some(old_entry) = cache.get(&url) {
            let old_size = image_size(old_entry.as_ref());
            *current_bytes = current_bytes.saturating_sub(old_size);
        }

        cache.insert(url, Arc::new(image));
        *current_bytes += size;
    }

    fn evict(&self, url: &str) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let mut current_bytes = self.current_bytes.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(old_entry) = cache.remove(url) {
            let old_size = image_size(old_entry.as_ref());
            *current_bytes = current_bytes.saturating_sub(old_size);
        }
    }

    fn memory_usage(&self) -> usize {
        *self.current_bytes.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn clear(&self) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let mut current_bytes = self.current_bytes.lock().unwrap_or_else(|e| e.into_inner());

        cache.clear();
        *current_bytes = 0;
    }
}

/// Type alias for DOM image cache instances
pub type DomSharedImageCache = Arc<dyn DomImageCache>;

/// Create a default DOM image cache with reasonable settings
pub fn create_default_dom_image_cache() -> DomSharedImageCache {
    Arc::new(DomImageCacheAdapter::new(create_default_image_cache()))
}

/// Create a DOM image cache with a custom memory limit
pub fn create_dom_image_cache_with_limit(max_bytes: usize) -> DomSharedImageCache {
    Arc::new(DomImageCacheAdapter::with_limit(
        create_image_cache_with_limit(max_bytes),
        max_bytes,
    ))
}

/// Create a DOM image cache with both memory and entry limits
pub fn create_dom_image_cache_with_limits(
    max_bytes: usize,
    max_entries: usize,
) -> DomSharedImageCache {
    Arc::new(DomImageCacheAdapter::new(create_image_cache_with_limits(
        max_bytes,
        max_entries,
    )))
}

/// Create a DOM image cache with TTL support
pub fn create_dom_image_cache_with_ttl(max_bytes: usize, ttl: Duration) -> DomSharedImageCache {
    Arc::new(DomImageCacheAdapter::new(create_image_cache_with_ttl(
        max_bytes, ttl,
    )))
}

/// Registry of pending image loads
///
/// This is separate from the cache and tracks in-flight image downloads
/// to avoid duplicate requests for the same URL.
pub struct PendingImageRegistry {
    /// Map from URL to list of (node_id, image_type) pairs waiting for the image
    pending: Mutex<HashMap<String, Vec<(usize, ImageType)>>>,
}

impl PendingImageRegistry {
    /// Create a new pending image registry
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Register a node as waiting for an image
    pub fn register(&self, url: String, node_id: usize, image_type: ImageType) {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.entry(url).or_default().push((node_id, image_type));
    }

    /// Get and clear all nodes waiting for an image
    pub fn take_waiting(&self, url: &str) -> Vec<(usize, ImageType)> {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.remove(url).unwrap_or_default()
    }

    /// Check if any nodes are waiting for an image
    pub fn has_waiting(&self, url: &str) -> bool {
        let pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.contains_key(url)
    }

    /// Clear all pending registrations
    pub fn clear(&self) {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.clear();
    }

    /// Get count of pending URLs
    pub fn pending_count(&self) -> usize {
        let pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.len()
    }
}

impl Default for PendingImageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined image manager that owns both cache and pending registry
pub struct ImageManager {
    /// The shared image cache
    pub cache: DomSharedImageCache,
    /// Registry of pending image loads
    pub pending: Arc<PendingImageRegistry>,
}

impl ImageManager {
    /// Create a new image manager with default cache
    pub fn new() -> Self {
        Self {
            cache: create_default_dom_image_cache(),
            pending: Arc::new(PendingImageRegistry::new()),
        }
    }

    /// Create with custom cache
    pub fn with_cache(cache: DomSharedImageCache) -> Self {
        Self {
            cache,
            pending: Arc::new(PendingImageRegistry::new()),
        }
    }

    /// Get image from cache
    pub fn get_cached(&self, url: &str) -> Option<Arc<ImageData>> {
        self.cache.get(url)
    }

    /// Insert image into cache
    pub fn cache_image(&self, url: String, image: ImageData) {
        self.cache.insert(url, image);
    }

    /// Evict image from cache
    pub fn evict_image(&self, url: &str) {
        self.cache.evict(url);
    }

    /// Get memory usage
    pub fn memory_usage(&self) -> usize {
        self.cache.memory_usage()
    }

    /// Clear all cached images
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

impl Default for ImageManager {
    fn default() -> Self {
        Self::new()
    }
}
