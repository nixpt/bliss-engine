//! Image cache abstraction for Bliss
//!
//! Provides shared caching functionality for decoded images that can be used
//! across multiple documents to reduce memory usage and improve performance.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Trait for image data - this should be implemented by specific DOM implementations
pub trait ImageData: Send + Sync + 'static {
    /// Get approximate size in bytes
    fn size_bytes(&self) -> usize;
}

/// Trait for image cache implementations
pub trait ImageCache: Send + Sync + 'static {
    /// Get an image from cache by URL
    fn get(&self, url: &str) -> Option<Arc<dyn ImageData>>;

    /// Insert an image into cache
    fn insert(&self, url: String, image: Arc<dyn ImageData>);

    /// Evict an image from cache
    fn evict(&self, url: &str);

    /// Get current memory usage in bytes (approximate)
    fn memory_usage(&self) -> usize;

    /// Clear all cached images
    fn clear(&self);
}

/// Entry in LRU cache
struct CacheEntry {
    /// The cached image data
    image: Arc<dyn ImageData>,
    /// Last access time for LRU eviction
    last_accessed: Instant,
    /// Approximate size in bytes
    size: usize,
}

/// LRU (Least Recently Used) image cache implementation
pub struct LruImageCache {
    /// The cache storage
    cache: Mutex<HashMap<String, CacheEntry>>,
    /// Maximum memory usage in bytes
    max_bytes: usize,
    /// Current memory usage in bytes
    current_bytes: Mutex<usize>,
    /// Maximum number of entries (optional)
    max_entries: Option<usize>,
    /// TTL for cache entries (optional)
    ttl: Option<Duration>,
}

impl LruImageCache {
    /// Create a new LRU cache with the specified memory limit
    pub fn new(max_bytes: usize) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            max_bytes,
            current_bytes: Mutex::new(0),
            max_entries: None,
            ttl: None,
        }
    }

    /// Set maximum number of entries
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = Some(max_entries);
        self
    }

    /// Set TTL for cache entries
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Evict least recently used entries to free up space
    fn evict_lru(&self, needed_space: usize) {
        let mut cache = self.cache.lock().unwrap();
        let mut current_bytes = self.current_bytes.lock().unwrap();

        // Find entries to evict
        let mut entries_by_time: Vec<_> = cache
            .iter()
            .map(|(url, entry)| (url.clone(), entry.size, entry.last_accessed))
            .collect();
        entries_by_time.sort_by_key(|(_, _, last_accessed)| *last_accessed);

        let space_to_free = needed_space;
        let mut freed_space = 0;

        for (url, size, _) in entries_by_time {
            if freed_space >= space_to_free {
                break;
            }

            freed_space += size;
            *current_bytes = current_bytes.saturating_sub(size);
            cache.remove(&url);
        }
    }

    /// Clean up expired entries
    fn cleanup_expired(&self) {
        if self.ttl.is_none() {
            return;
        }

        let ttl = self.ttl.unwrap();
        let mut cache = self.cache.lock().unwrap();
        let mut current_bytes = self.current_bytes.lock().unwrap();

        let now = Instant::now();
        cache.retain(|_, entry| {
            let is_expired = now.duration_since(entry.last_accessed) > ttl;
            if is_expired {
                *current_bytes = current_bytes.saturating_sub(entry.size);
            }
            !is_expired
        });
    }

    /// Enforce entry count limit
    fn enforce_entry_limit(&self) {
        let Some(max_entries) = self.max_entries else {
            return;
        };

        let mut cache = self.cache.lock().unwrap();
        let mut current_bytes = self.current_bytes.lock().unwrap();

        if cache.len() <= max_entries {
            return;
        }

        // Remove oldest entries until we're under the limit
        let mut entries_by_time: Vec<_> = cache
            .iter()
            .map(|(url, entry)| (url.clone(), entry.size, entry.last_accessed))
            .collect();
        entries_by_time.sort_by_key(|(_, _, last_accessed)| *last_accessed);

        let entries_to_remove = cache.len() - max_entries;
        for (url, size, _) in entries_by_time.into_iter().take(entries_to_remove) {
            *current_bytes = current_bytes.saturating_sub(size);
            cache.remove(&url);
        }
    }
}

impl ImageCache for LruImageCache {
    fn get(&self, url: &str) -> Option<Arc<dyn ImageData>> {
        // Cleanup expired entries first
        self.cleanup_expired();

        let mut cache = self.cache.lock().unwrap();

        if let Some(entry) = cache.get_mut(url) {
            entry.last_accessed = Instant::now();
            Some(Arc::clone(&entry.image))
        } else {
            None
        }
    }

    fn insert(&self, url: String, image: Arc<dyn ImageData>) {
        // Cleanup expired entries first
        self.cleanup_expired();

        let size = image.size_bytes();
        let mut cache = self.cache.lock().unwrap();
        let mut current_bytes = self.current_bytes.lock().unwrap();

        // Check if we need to evict entries to make space
        if *current_bytes + size > self.max_bytes {
            let needed_space = *current_bytes + size - self.max_bytes;
            drop(cache);
            drop(current_bytes);
            self.evict_lru(needed_space);
            cache = self.cache.lock().unwrap();
            current_bytes = self.current_bytes.lock().unwrap();
        }

        // Insert or update the entry
        if let Some(old_entry) = cache.insert(
            url,
            CacheEntry {
                image,
                last_accessed: Instant::now(),
                size,
            },
        ) {
            *current_bytes = current_bytes.saturating_sub(old_entry.size);
        }
        *current_bytes += size;

        // Enforce entry count limit
        drop(cache);
        drop(current_bytes);
        self.enforce_entry_limit();
    }

    fn evict(&self, url: &str) {
        let mut cache = self.cache.lock().unwrap();
        let mut current_bytes = self.current_bytes.lock().unwrap();

        if let Some(entry) = cache.remove(url) {
            *current_bytes = current_bytes.saturating_sub(entry.size);
        }
    }

    fn memory_usage(&self) -> usize {
        *self.current_bytes.lock().unwrap()
    }

    fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        let mut current_bytes = self.current_bytes.lock().unwrap();

        cache.clear();
        *current_bytes = 0;
    }
}

impl Default for LruImageCache {
    fn default() -> Self {
        Self::new(100 * 1024 * 1024) // 100MB default
    }
}

/// Type alias for shared image cache instances
pub type SharedImageCache = Arc<dyn ImageCache>;

/// Create a default image cache with reasonable settings
pub fn create_default_image_cache() -> SharedImageCache {
    Arc::new(LruImageCache::default())
}

/// Create an image cache with a custom memory limit
pub fn create_image_cache_with_limit(max_bytes: usize) -> SharedImageCache {
    Arc::new(LruImageCache::new(max_bytes))
}

/// Create an image cache with both memory and entry limits
pub fn create_image_cache_with_limits(max_bytes: usize, max_entries: usize) -> SharedImageCache {
    Arc::new(LruImageCache::new(max_bytes).with_max_entries(max_entries))
}

/// Create an image cache with TTL support
pub fn create_image_cache_with_ttl(max_bytes: usize, ttl: Duration) -> SharedImageCache {
    Arc::new(LruImageCache::new(max_bytes).with_ttl(ttl))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestImageData {
        data: Vec<u8>,
    }

    impl ImageData for TestImageData {
        fn size_bytes(&self) -> usize {
            self.data.len()
        }
    }

    #[test]
    fn test_basic_cache_operations() {
        let cache = LruImageCache::new(1024); // 1KB limit
        let image = Arc::new(TestImageData {
            data: vec![1, 2, 3, 4],
        });

        // Test insertion
        cache.insert(
            "test://image1".to_string(),
            Arc::clone(&image) as Arc<dyn ImageData>,
        );

        // Test retrieval
        let retrieved = cache.get("test://image1");
        assert!(retrieved.is_some());

        // Test eviction
        cache.evict("test://image1");
        assert!(cache.get("test://image1").is_none());
    }

    #[test]
    fn test_memory_limit() {
        let cache = LruImageCache::new(8); // 8 bytes limit
        let image1 = Arc::new(TestImageData { data: vec![1; 4] }) as Arc<dyn ImageData>;
        let image2 = Arc::new(TestImageData { data: vec![2; 4] }) as Arc<dyn ImageData>;
        let image3 = Arc::new(TestImageData { data: vec![3; 4] }) as Arc<dyn ImageData>;

        // Insert first image
        cache.insert("img1".to_string(), image1);
        assert_eq!(cache.memory_usage(), 4);

        // Insert second image
        cache.insert("img2".to_string(), image2);
        assert_eq!(cache.memory_usage(), 8);

        // Insert third image - should evict oldest
        cache.insert("img3".to_string(), image3);
        assert_eq!(cache.memory_usage(), 8);

        // First image should be evicted
        assert!(cache.get("img1").is_none());
        assert!(cache.get("img2").is_some());
        assert!(cache.get("img3").is_some());
    }

    #[test]
    fn test_ttl() {
        let cache = LruImageCache::new(1024).with_ttl(Duration::from_millis(10));
        let image = Arc::new(TestImageData {
            data: vec![1, 2, 3, 4],
        }) as Arc<dyn ImageData>;

        cache.insert("test".to_string(), Arc::clone(&image));
        assert!(cache.get("test").is_some());

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(15));
        assert!(cache.get("test").is_none());
    }

    #[test]
    fn test_entry_limit() {
        let cache = LruImageCache::new(1024).with_max_entries(2);
        let image1 = Arc::new(TestImageData { data: vec![1; 2] }) as Arc<dyn ImageData>;
        let image2 = Arc::new(TestImageData { data: vec![2; 2] }) as Arc<dyn ImageData>;
        let image3 = Arc::new(TestImageData { data: vec![3; 2] }) as Arc<dyn ImageData>;

        cache.insert("img1".to_string(), image1);
        cache.insert("img2".to_string(), image2);
        cache.insert("img3".to_string(), image3);

        // Should only have the 2 most recent entries
        assert!(cache.get("img1").is_none());
        assert!(cache.get("img2").is_some());
        assert!(cache.get("img3").is_some());
    }
}
