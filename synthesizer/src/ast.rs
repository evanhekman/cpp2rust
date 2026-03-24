pub type Path = Vec<usize>;

#[derive(Clone, Debug, PartialEq)]
pub enum Child {
    Node(Box<Node>),
    Hole(String), // nonterminal name
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub kind: String,
    pub children: Vec<Child>,
    pub depth: usize,
}

impl Node {
    pub fn new(kind: impl Into<String>, children: Vec<Child>, depth: usize) -> Self {
        Self {
            kind: kind.into(),
            children,
            depth,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.children.iter().all(|c| match c {
            Child::Hole(_) => false,
            Child::Node(n) => n.is_complete(),
        })
    }

    pub fn first_hole_path(&self) -> Option<Path> {
        for (i, child) in self.children.iter().enumerate() {
            match child {
                Child::Hole(_) => return Some(vec![i]),
                Child::Node(n) => {
                    if let Some(mut sub) = n.first_hole_path() {
                        sub.insert(0, i);
                        return Some(sub);
                    }
                }
            }
        }
        None
    }

    pub fn hole_nt_at_path(&self, path: &[usize]) -> &str {
        let mut node = self;
        for &idx in &path[..path.len() - 1] {
            match &node.children[idx] {
                Child::Node(n) => node = n,
                Child::Hole(_) => panic!("hit hole before end of path"),
            }
        }
        match &node.children[*path.last().unwrap()] {
            Child::Hole(nt) => nt,
            Child::Node(_) => panic!("expected hole at path end"),
        }
    }


    pub fn node_at_path(&self, path: &[usize]) -> &Node {
        if path.is_empty() {
            return self;
        }
        let mut node = self;
        for &idx in path {
            match &node.children[idx] {
                Child::Node(n) => node = n,
                Child::Hole(_) => panic!("hit hole"),
            }
        }
        node
    }

    pub fn replace_at_path(&self, path: &[usize], replacement: Node) -> Node {
        if path.is_empty() {
            return replacement;
        }
        let i = path[0];
        let mut new_children = self.children.clone();
        if path.len() == 1 {
            new_children[i] = Child::Node(Box::new(replacement));
        } else {
            match &new_children[i] {
                Child::Node(child) => {
                    let new_child = child.replace_at_path(&path[1..], replacement);
                    new_children[i] = Child::Node(Box::new(new_child));
                }
                Child::Hole(_) => panic!("replace_at_path: encountered hole mid-path"),
            }
        }
        Node {
            kind: self.kind.clone(),
            children: new_children,
            depth: self.depth,
        }
    }
}
