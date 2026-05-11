use crate::{Bdd, Node, NodeId, Variable};
use std::cmp::min;
use std::collections::HashMap;

const TERMINAL_VARIABLE: Variable = Variable::MAX;

impl NodeId {
    const TERMINAL_0: Self = NodeId(0);
    const TERMINAL_1: Self = NodeId(1);

    fn as_usize(self) -> usize {
        self.0
    }

    fn is_terminal(&self) -> bool {
        self == &Self::TERMINAL_0 || self == &Self::TERMINAL_1
    }

    fn is_one(self) -> bool {
        self == Self::TERMINAL_1
    }

    fn is_zero(self) -> bool {
        self == Self::TERMINAL_0
    }
}

impl Node {
    fn new(variable: Variable, low_child: NodeId, high_child: NodeId) -> Self {
        Self {
            variable,
            low_child,
            high_child,
        }
    }

    fn one() -> Self {
        Self::new(TERMINAL_VARIABLE, NodeId::TERMINAL_1, NodeId::TERMINAL_1)
    }

    fn zero() -> Self {
        Self::new(TERMINAL_VARIABLE, NodeId::TERMINAL_0, NodeId::TERMINAL_0)
    }

    fn is_one(&self) -> bool {
        self.low_child == NodeId::TERMINAL_1
    }

    fn is_zero(&self) -> bool {
        self.low_child == NodeId::TERMINAL_0
    }

    fn is_terminal(&self) -> bool {
        self.variable == TERMINAL_VARIABLE
    }
}

impl Bdd {
    fn new() -> Self {
        let terminal_0 = Node::zero();
        let terminal_1 = Node::one();

        Bdd {
            nodes: Vec::from([terminal_0, terminal_1]),
            node_table: HashMap::new(),
            task_cache: HashMap::new(),
        }
    }

    fn apply_recursive(&mut self, left: NodeId, right: NodeId) -> (NodeId, Node) {
        if left.is_terminal() && right.is_terminal() {
            return if left.is_one() && right.is_one() {
                (NodeId::TERMINAL_1, self.nodes[1])
            } else {
                (NodeId::TERMINAL_0, self.nodes[0])
            };
        }

        if let Some(found_node_id) = self.task_cache.get(&(left, right)) {
            return (*found_node_id, self.nodes[found_node_id.as_usize()]);
        }

        let left_node = self.nodes[left.as_usize()];
        let right_node = self.nodes[right.as_usize()];
        let v = min(left_node.variable, right_node.variable);

        let (low_left, high_left) = if left_node.variable == v {
            (left_node.low_child, left_node.high_child)
        } else {
            (left, left)
        };

        let (low_right, high_right) = if right_node.variable == v {
            (right_node.low_child, right_node.high_child)
        } else {
            (right, right)
        };

        let l = self.apply_recursive(low_left, low_right);
        let h = self.apply_recursive(high_left, high_right);

        let (c_node_id, c) = if l != h {
            self.ensure_node(v, l.0, h.0)
        } else {
            l
        };

        self.task_cache.insert((left, right), c_node_id);
        (c_node_id, c)
    }

    fn ensure_node(
        &mut self,
        variable: Variable,
        low_child: NodeId,
        high_child: NodeId,
    ) -> (NodeId, Node) {
        let needle = Node::new(variable, low_child, high_child);
        if let Some(found) = self.node_table.get(&needle) {
            (*found, needle)
        } else {
            let node_id = NodeId(self.nodes.len());
            self.nodes.push(needle);
            self.node_table.insert(needle, node_id);
            (node_id, needle)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_recursion_manual_bdd_construction() {
        let mut nodes: Vec<Node> = Vec::with_capacity(8);

        let zero = Node::zero();
        let zero_id = NodeId::TERMINAL_0;
        nodes.insert(zero_id.as_usize(), zero);
        let one = Node::one();
        let one_id = NodeId::TERMINAL_1;
        nodes.insert(one_id.as_usize(), one);

        let a4 = Node::new(3, NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let a4_id = NodeId(2);
        nodes.insert(a4_id.as_usize(), a4);

        let a3 = Node::new(2, NodeId::TERMINAL_1, a4_id);
        let a3_id = NodeId(3);
        nodes.insert(a3_id.as_usize(), a3);
        let a2 = Node::new(2, NodeId::TERMINAL_0, a4_id);
        let a2_id = NodeId(4);
        nodes.insert(a2_id.as_usize(), a2);

        let a1 = Node::new(1, a2_id, a3_id);
        let a1_id = NodeId(5);
        nodes.insert(a1_id.as_usize(), a1);

        let b3 = Node::new(3, NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let b3_id = NodeId(6);
        nodes.insert(b3_id.as_usize(), b3);

        let b2 = a4;
        let b2_id = a4_id;
        // nodes.insert(b2_id.as_usize(), b2); // avoid duplicities - node is identical to a4

        let b1 = Node::new(2, b2_id, b3_id);
        let b1_id = NodeId(7);
        nodes.insert(b1_id.as_usize(), b1);

        let mut bdd = Bdd::new();
        bdd.nodes = nodes;

        let mut node_table: HashMap<Node, NodeId> = HashMap::with_capacity(8);
        node_table.insert(zero, zero_id);
        node_table.insert(one, one_id);
        node_table.insert(a1, a1_id);
        node_table.insert(a2, a2_id);
        node_table.insert(a3, a3_id);
        node_table.insert(a4, a4_id);
        node_table.insert(b1, b1_id);
        // node_table.insert(b2, b2_id); // avoid duplicities - node is identical to a4
        node_table.insert(b3, b3_id);
        bdd.node_table = node_table;

        let (_, c1) = bdd.apply_recursive(a1_id, b1_id);
        assert_eq!(c1.variable, 1);
        assert_eq!(c1.low_child.as_usize(), 0);

        let c2 = bdd.nodes[c1.high_child.as_usize()];
        assert_eq!(c2.variable, 2);
        assert_eq!(c2.high_child.as_usize(), 0);

        let c3 = bdd.nodes[c2.low_child.as_usize()];
        assert_eq!(c3.variable, 3);
        assert_eq!(c3.low_child.as_usize(), 0);
        assert_eq!(c3.high_child.as_usize(), 1);
    }
}
