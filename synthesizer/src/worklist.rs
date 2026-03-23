use crate::ast::Node;
use std::collections::{BTreeMap, VecDeque};

/// Best-first worklist with optional capacity bounding.
/// Items are ordered by score (ascending); ties are FIFO.
/// When capacity is set and exceeded, the *worst* (highest-score) item is evicted.
pub struct Worklist {
    map: BTreeMap<i64, VecDeque<Node>>,
    total: usize,
    max_size: usize,
    evictions: usize,
}

impl Worklist {
    pub fn new() -> Self {
        Self::with_capacity(usize::MAX)
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            map: BTreeMap::new(),
            total: 0,
            max_size,
            evictions: 0,
        }
    }

    pub fn push(&mut self, node: Node, score: i64) {
        if self.total >= self.max_size {
            // Check whether new item is better than the current worst.
            if let Some((&worst_score, _)) = self.map.iter().next_back() {
                if score > worst_score {
                    // New item is strictly worse than everything we keep; discard it.
                    return;
                }
                // score <= worst_score: evict one worst item (FIFO rotation within same tier)
                // Evict one item from the worst bucket.
                let remove_bucket = self.map.get_mut(&worst_score).unwrap();
                remove_bucket.pop_back();
                if remove_bucket.is_empty() {
                    self.map.remove(&worst_score);
                }
                self.total -= 1;
                self.evictions += 1;
            }
        }
        self.map.entry(score).or_default().push_back(node);
        self.total += 1;
    }

    pub fn pop(&mut self) -> Option<Node> {
        let (&score, _) = self.map.iter().next()?;
        let bucket = self.map.get_mut(&score).unwrap();
        let node = bucket.pop_front().unwrap();
        if bucket.is_empty() {
            self.map.remove(&score);
        }
        self.total -= 1;
        Some(node)
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    pub fn len(&self) -> usize {
        self.total
    }

    pub fn evictions(&self) -> usize {
        self.evictions
    }
}
