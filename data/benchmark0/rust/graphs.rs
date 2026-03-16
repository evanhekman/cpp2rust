// Naive Rust Translation
/*
struct Node {
    name: String,
    neighbors: Vec<&Node>,
}
*/

use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Node {
    name: String,
    neighbors: Vec<Weak<RefCell<Node>>>,
}