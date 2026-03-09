use std::collections::BinaryHeap;
use std::cmp::Ordering;
use crate::ast::Node;

struct Entry {
    score: i64,
    counter: u64,
    node: Node,
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        // min-heap by score, then by insertion order (lower counter = earlier)
        other.score.cmp(&self.score)
            .then(other.counter.cmp(&self.counter))
    }
}
impl PartialOrd for Entry { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) } }
impl PartialEq for Entry { fn eq(&self, other: &Self) -> bool { self.cmp(other) == Ordering::Equal } }
impl Eq for Entry {}

pub struct Worklist {
    heap: BinaryHeap<Entry>,
    counter: u64,
}

impl Worklist {
    pub fn new() -> Self { Self { heap: BinaryHeap::new(), counter: 0 } }
    pub fn push(&mut self, node: Node, score: i64) {
        self.heap.push(Entry { score, counter: self.counter, node });
        self.counter += 1;
    }
    pub fn pop(&mut self) -> Option<Node> { self.heap.pop().map(|e| e.node) }
    pub fn is_empty(&self) -> bool { self.heap.is_empty() }
    pub fn len(&self) -> usize { self.heap.len() }
}
