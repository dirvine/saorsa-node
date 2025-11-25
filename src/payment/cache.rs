//! LRU cache for verified XorNames.
//!
//! Caches XorNames that have been verified to exist on the autonomi network,
//! reducing the number of network queries needed for repeated/popular data.

use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// XorName type - 32-byte content hash.
/// TODO: Import from saorsa-core or ant-protocol when available.
pub type XorName = [u8; 32];

/// Default cache capacity (100,000 entries ≈ 3.2MB memory).
const DEFAULT_CACHE_CAPACITY: usize = 100_000;

/// LRU cache for verified XorNames.
///
/// This cache stores XorNames that have been verified to exist on the
/// autonomi network, avoiding repeated network queries for the same data.
#[derive(Clone)]
pub struct VerifiedCache {
    inner: Arc<Mutex<LruCache<XorName, ()>>>,
    stats: Arc<Mutex<CacheStats>>,
}

/// Cache statistics for monitoring.
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of entries added.
    pub additions: u64,
}

impl CacheStats {
    /// Calculate hit rate as a percentage.
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

impl VerifiedCache {
    /// Create a new cache with default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    /// Create a new cache with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).expect("1 is non-zero"));
        Self {
            inner: Arc::new(Mutex::new(LruCache::new(cap))),
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// Check if a XorName is in the cache.
    ///
    /// Returns `true` if the XorName is cached (verified to exist on autonomi).
    pub fn contains(&self, xorname: &XorName) -> bool {
        let mut cache = self.inner.lock();
        let found = cache.get(xorname).is_some();

        let mut stats = self.stats.lock();
        if found {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }

        found
    }

    /// Add a XorName to the cache.
    ///
    /// This should be called after verifying that data exists on the autonomi network.
    pub fn insert(&self, xorname: XorName) {
        let mut cache = self.inner.lock();
        cache.put(xorname, ());

        let mut stats = self.stats.lock();
        stats.additions += 1;
    }

    /// Get current cache statistics.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().clone()
    }

    /// Get the current number of entries in the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// Check if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

impl Default for VerifiedCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = VerifiedCache::new();

        let xorname1 = [1u8; 32];
        let xorname2 = [2u8; 32];

        // Initially empty
        assert!(cache.is_empty());
        assert!(!cache.contains(&xorname1));

        // Insert and check
        cache.insert(xorname1);
        assert!(cache.contains(&xorname1));
        assert!(!cache.contains(&xorname2));
        assert_eq!(cache.len(), 1);

        // Insert another
        cache.insert(xorname2);
        assert!(cache.contains(&xorname1));
        assert!(cache.contains(&xorname2));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_stats() {
        let cache = VerifiedCache::new();
        let xorname = [1u8; 32];

        // Miss
        assert!(!cache.contains(&xorname));
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);

        // Add
        cache.insert(xorname);
        let stats = cache.stats();
        assert_eq!(stats.additions, 1);

        // Hit
        assert!(cache.contains(&xorname));
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);

        // Hit rate should be 50%
        assert!((stats.hit_rate() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_cache_lru_eviction() {
        // Small cache for testing eviction
        let cache = VerifiedCache::with_capacity(2);

        let xorname1 = [1u8; 32];
        let xorname2 = [2u8; 32];
        let xorname3 = [3u8; 32];

        cache.insert(xorname1);
        cache.insert(xorname2);
        assert_eq!(cache.len(), 2);

        // Insert third, should evict xorname1 (least recently used)
        cache.insert(xorname3);
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains(&xorname1)); // evicted
        // Note: after contains call on evicted item, stats will show a miss
    }

    #[test]
    fn test_cache_clear() {
        let cache = VerifiedCache::new();

        cache.insert([1u8; 32]);
        cache.insert([2u8; 32]);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }
}
