use crate::{Bdd, Node, NodeId, Variable};
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

        let v: Variable;
        let left_node = self.nodes[left.as_usize()];
        let right_node = self.nodes[right.as_usize()];
        let (low_left, high_left): (NodeId, NodeId);
        let (low_right, high_right): (NodeId, NodeId);
        if left_node.variable < right_node.variable {
            v = left_node.variable;
            (low_left, high_left) = (left_node.low_child, left_node.high_child);
            (low_right, high_right) = (right, right);
        } else {
            v = right_node.variable;
            (low_left, high_left) = (left, left);
            (low_right, high_right) = (right_node.low_child, right_node.high_child);
        }

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
