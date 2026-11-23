use crate::types::TileId;
use std::cell::RefCell;

/// Maximum cost buckets for Dial's O(V + E) ring-bucket Dijkstra (max edge cost = 4 halves).
const NUM_BUCKETS: usize = 8;

/// Zero-allocation generation-stamped scratch arena stored per worker thread.
pub struct AiScratch {
    pub num_tiles: usize,
    pub generation: u32,
    pub stamp: Vec<u32>,
    pub dist: Vec<u16>,
    pub buckets: [Vec<TileId>; NUM_BUCKETS],
    pub bfs_queue: Vec<TileId>,
}

impl AiScratch {
    pub fn new() -> Self {
        Self {
            num_tiles: 0,
            generation: 0,
            stamp: Vec::new(),
            dist: Vec::new(),
            buckets: std::array::from_fn(|_| Vec::with_capacity(256)),
            bfs_queue: Vec::with_capacity(1024),
        }
    }

    #[inline]
    pub fn prepare(&mut self, num_tiles: usize) -> u32 {
        if self.num_tiles != num_tiles {
            self.num_tiles = num_tiles;
            self.stamp.clear();
            self.stamp.resize(num_tiles, 0);
            self.dist.clear();
            self.dist.resize(num_tiles, u16::MAX);
            self.generation = 0;
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.generation = 1;
        }
        for b in &mut self.buckets {
            b.clear();
        }
        self.bfs_queue.clear();
        self.generation
    }
}

thread_local! {
    static TLS_SCRATCH: RefCell<AiScratch> = RefCell::new(AiScratch::new());
}

#[inline]
pub fn with_tls_scratch<R>(f: impl FnOnce(&mut AiScratch) -> R) -> R {
    TLS_SCRATCH.with(|cell| f(&mut cell.borrow_mut()))
}
