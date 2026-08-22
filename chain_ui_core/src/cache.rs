// chain_ui_core/src/cache.rs
//
// A bounded, thread-local cache for pre-rendered HTML fragments.
// This version utilizes an array-backed doubly-linked list to achieve
// true O(1) inserts, lookups, and evictions with zero double-hashing.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::Arc;

// ============================================================
// Robust FxHash — Fixed to include the multiplication step
// to ensure high distribution quality and avoid collisions.
// ============================================================
pub struct FxHasher {
    hash: usize,
}

const FX_MULT_CONSTANT: usize = if cfg!(target_pointer_width = "64") {
    0x517cc1b727220a95
} else {
    0x227b44bd
};

impl Default for FxHasher {
    #[inline]
    fn default() -> Self {
        Self { hash: 0 }
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(std::mem::size_of::<usize>());
        for chunk in &mut chunks {
            let word = usize::from_ne_bytes(chunk.try_into().unwrap());
            self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(FX_MULT_CONSTANT);
        }
        for &byte in chunks.remainder() {
            self.hash = (self.hash.rotate_left(5) ^ (byte as usize)).wrapping_mul(FX_MULT_CONSTANT);
        }
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash as u64
    }
}

// ============================================================
// Identity Hasher — Prevents the HashMap from hashing the
// pre-calculated u64 key a second time.
// ============================================================
#[derive(Default, Clone, Copy)]
struct IdentityBuildHasher;

impl BuildHasher for IdentityBuildHasher {
    type Hasher = IdentityHasher;
    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        IdentityHasher { hash: 0 }
    }
}

struct IdentityHasher {
    hash: u64,
}

impl Hasher for IdentityHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
    #[inline]
    fn write(&mut self, _bytes: &[u8]) {
        // Fallback safety: If this hits, someone tried to use this hasher 
        // for something other than a u64 key.
        debug_assert!(false, "IdentityHasher should only be used for primitive u64 keys!");
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }
}

/// Hard ceiling on how many distinct entries the cache will ever hold.
const MAX_CACHE_ENTRIES: usize = 2048;
const EMPTY_INDEX: usize = usize::MAX;

struct Node {
    key: u64,
    bytes: Arc<[u8]>,
    prev: usize,
    next: usize,
}

struct Cache {
    // Maps the pre-calculated u64 key to an index inside the `nodes` vector.
    map: HashMap<u64, usize, IdentityBuildHasher>,
    nodes: Vec<Node>,
    head: usize, // Points to the Least Recently Used (oldest) item
    tail: usize, // Points to the Most Recently Used (newest) item
}

impl Cache {
    fn new() -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(MAX_CACHE_ENTRIES, IdentityBuildHasher),
            nodes: Vec::with_capacity(MAX_CACHE_ENTRIES),
            head: EMPTY_INDEX,
            tail: EMPTY_INDEX,
        }
    }

    /// Detaches an existing node from its current linked list neighbors
    #[inline]
    fn detach(&mut self, idx: usize) {
        let prev = self.nodes[idx].prev;
        let next = self.nodes[idx].next;

        if prev != EMPTY_INDEX {
            self.nodes[prev].next = next;
        } else {
            self.head = next;
        }

        if next != EMPTY_INDEX {
            self.nodes[next].prev = prev;
        } else {
            self.tail = prev;
        }
    }

    /// Appends a node to the tail of the list, marking it Most Recently Used (MRU)
    #[inline]
    fn append_to_tail(&mut self, idx: usize) {
        self.nodes[idx].prev = self.tail;
        self.nodes[idx].next = EMPTY_INDEX;

        if self.tail != EMPTY_INDEX {
            self.nodes[self.tail].next = idx;
        }
        self.tail = idx;

        if self.head == EMPTY_INDEX {
            self.head = idx;
        }
    }
}

thread_local! {
    static COMPONENT_CACHE: RefCell<Cache> = RefCell::new(Cache::new());
}

/// Looks up or generates a cached component by key. Operates entirely in
/// O(1) time complexity for both hits, misses, and evictions.
#[inline]
pub fn component<K, G>(key_data: K, generator: G) -> Arc<[u8]>
where
    K: Hash,
    G: FnOnce() -> Vec<u8>,
{
    let mut hasher = FxHasher::default();
    key_data.hash(&mut hasher);
    let key = hasher.finish();

    COMPONENT_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();

        // 1. Cache Hit Path
        if let Some(&node_idx) = cache.map.get(&key) {
            cache.detach(node_idx);
            cache.append_to_tail(node_idx);
            return cache.nodes[node_idx].bytes.clone();
        }

        // 2. Cache Miss Path
        let bytes: Arc<[u8]> = generator().into();

        if cache.map.len() >= MAX_CACHE_ENTRIES {
            // Evict the true oldest item at the head of the list (O(1))
            let evict_idx = cache.head;
            let evict_key = cache.nodes[evict_idx].key;
            cache.map.remove(&evict_key);
            cache.detach(evict_idx);

            // Reuse the existing vector slot allocation directly
            cache.nodes[evict_idx] = Node {
                key,
                bytes: bytes.clone(),
                prev: EMPTY_INDEX,
                next: EMPTY_INDEX,
            };
            cache.append_to_tail(evict_idx);
            cache.map.insert(key, evict_idx);
        } else {
            // System is warming up; allocate a new node slot
            let node_idx = cache.nodes.len();
            cache.nodes.push(Node {
                key,
                bytes: bytes.clone(),
                prev: EMPTY_INDEX,
                next: EMPTY_INDEX,
            });
            cache.append_to_tail(node_idx);
            cache.map.insert(key, node_idx);
        }

        bytes
    })
}


/// Unconditionally writes a value into the cache under `key`,
/// overwriting whatever was there before (if anything). Unlike
/// component(), this always runs — there's no generator closure to
/// skip, because the whole point is to force an update regardless of
/// whether the key already exists. Used by stale-while-revalidate
/// patterns that need to refresh a cache entry after showing its old
/// value.
pub fn set<K: Hash>(key_data: K, bytes: Vec<u8>) -> Arc<[u8]> {
    let mut hasher = FxHasher::default();
    key_data.hash(&mut hasher);
    let key = hasher.finish();
    let bytes: Arc<[u8]> = bytes.into();

    COMPONENT_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();

        if let Some(&node_idx) = cache.map.get(&key) {
            cache.detach(node_idx);
            cache.nodes[node_idx].bytes = bytes.clone();
            cache.append_to_tail(node_idx);
            return bytes;
        }

        if cache.map.len() >= MAX_CACHE_ENTRIES {
            let evict_idx = cache.head;
            let evict_key = cache.nodes[evict_idx].key;
            cache.map.remove(&evict_key);
            cache.detach(evict_idx);

            cache.nodes[evict_idx] = Node {
                key,
                bytes: bytes.clone(),
                prev: EMPTY_INDEX,
                next: EMPTY_INDEX,
            };
            cache.append_to_tail(evict_idx);
            cache.map.insert(key, evict_idx);
        } else {
            let node_idx = cache.nodes.len();
            cache.nodes.push(Node {
                key,
                bytes: bytes.clone(),
                prev: EMPTY_INDEX,
                next: EMPTY_INDEX,
            });
            cache.append_to_tail(node_idx);
            cache.map.insert(key, node_idx);
        }

        bytes
    })
}


/// Looks up a cached value WITHOUT generating on a miss. Returns None
/// if nothing is cached under this key yet. Used by stale-while-
/// revalidate patterns that want to show "whatever we have, even if
/// it's old" instantly, rather than blocking on a fresh render.
#[inline]
pub fn try_get<K: Hash>(key_data: K) -> Option<Arc<[u8]>> {
    let mut hasher = FxHasher::default();
    key_data.hash(&mut hasher);
    let key = hasher.finish();

    COMPONENT_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();
        let node_idx = *cache.map.get(&key)?;
        cache.detach(node_idx);
        cache.append_to_tail(node_idx);
        Some(cache.nodes[node_idx].bytes.clone())
    })
}

/// Clears the entire cache for the current thread.
pub fn clear_local_cache() {
    COMPONENT_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();
        cache.map.clear();
        cache.nodes.clear();
        cache.head = EMPTY_INDEX;
        cache.tail = EMPTY_INDEX;
    });
}

/// Current number of entries in the cache.
pub fn cache_len() -> usize {
    COMPONENT_CACHE.with(|cache_cell| cache_cell.borrow().map.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_hit_returns_same_bytes_without_rerunning_generator() {
        clear_local_cache();
        let mut calls = 0;
        let _ = component("key-a", || {
            calls += 1;
            vec![1, 2, 3]
        });
        let _ = component("key-a", || {
            calls += 1;
            vec![9, 9, 9]
        });
        assert_eq!(calls, 1);
    }

    #[test]
    fn cache_respects_capacity_ceiling() {
        clear_local_cache();
        for i in 0..(MAX_CACHE_ENTRIES + 100) {
            let _ = component(i, || vec![0u8; 4]);
        }
        assert_eq!(cache_len(), MAX_CACHE_ENTRIES);
    }

    #[test]
    fn clear_local_cache_actually_empties_it() {
        clear_local_cache();
        let _ = component("temp", || vec![1]);
        assert!(cache_len() > 0);
        clear_local_cache();
        assert_eq!(cache_len(), 0);
    }

    #[test]
    fn cache_evicts_lru_order_correctly() {
        clear_local_cache();

        // Fill cache completely with keys 0..2048
        for i in 0..MAX_CACHE_ENTRIES {
            let _ = component(i, || vec![i as u8]);
        }

        // Touch key 0 to make it the Most Recently Used item
        let _ = component(0, || vec![0]);

        // Insert a completely fresh key to trigger an eviction
        let _ = component(99999, || vec![255]);

        // Key 1 should have been evicted (oldest), but Key 0 must remain.
        let mut executed_generator = false;
        let _ = component(0, || {
            executed_generator = true;
            vec![0]
        });
        assert!(!executed_generator, "Key 0 was prematurely evicted!");
    }
    
    
    #[test]
    fn try_get_returns_none_on_miss_without_generating() {
        clear_local_cache();
        assert!(try_get("never-cached").is_none());
    }

    #[test]
    fn try_get_returns_some_after_a_component_call() {
        clear_local_cache();
        let _ = component("try-get-test", || b"hello".to_vec());
        let found = try_get("try-get-test");
        assert_eq!(found.as_deref(), Some(&b"hello"[..]));
    }
}
