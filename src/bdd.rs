use crate::{Bdd, Node};
use std::collections::HashMap;

const TERMINAL_0: u64 = u64::MAX - 1;
const TERMINAL_1: u64 = u64::MAX;

impl Node {
    fn new(variable: u64) -> Self {
        Self {
            variable,
            low_child: None,
            high_child: None,
        }
    }

    fn is_terminal(&self) -> bool {
        self.variable == TERMINAL_0 || self.variable == TERMINAL_1
    }
}

impl Bdd {
    fn apply_recursive_wrapper(left: Box<Node>, right: Box<Node>) -> Node {
        // finished
        let task_cache: HashMap<(Box<Node>, Box<Node>), Node> = HashMap::new();
        // existing
        let node_table: HashMap<(u64, u64, u64), Node> = HashMap::new();
        Self::apply_recursive(left, right, &task_cache, &node_table)
    }

    fn apply_recursive(
        left: Box<Node>,
        right: Box<Node>,
        task_cache: &HashMap<(Box<Node>, Box<Node>), Node>,
        node_table: &HashMap<(u64, u64, u64), Node>,
    ) -> Node {
        if left.is_terminal() && right.is_terminal() {
            let value = left.variable == TERMINAL_1 && right.variable == TERMINAL_1;
            return Node::new(if value { TERMINAL_1 } else { TERMINAL_0 });
        }

        if let Some(node) = task_cache.get(&(left.clone(), right.clone())) {
            return node.clone();
        }

        let v: u64;
        let (low_left, high_left): (Option<Box<Node>>, Option<Box<Node>>);
        let (low_right, high_right): (Option<Box<Node>>, Option<Box<Node>>);
        if left.variable < right.variable {
            v = left.variable;
            (low_left, high_left) = (left.low_child, left.high_child);
            (low_right, high_right) = (Some(right.clone()), Some(right.clone()));
        } else {
            v = right.variable;
            (low_left, high_left) = (Some(left.clone()), Some(left.clone()));
            (low_right, high_right) = (right.low_child, right.high_child);
        }

        let l = Self::apply_recursive(low_left, low_right, task_cache, node_table);
        let h = Self::apply_recursive(high_left, high_right, task_cache, node_table);

        let c: Node;
        if l != h {
            c = Self::ensure_node(node_table, v, l, h);
        } else {
            c = l;
        }

        task_cache.insert((left.clone(), right.clone()), c.clone());
    }

    fn ensure_node(
        node_table: &HashMap<(u64, u64, u64), Node>,
        variable: u64,
        low_child: Box<Node>,
        high_child: Box<Node>,
    ) -> Node {
        todo!()
    }
}
