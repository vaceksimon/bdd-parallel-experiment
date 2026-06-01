use crate::{Bdd, Node, NodeId, Variable};
use std::cmp::min;
use std::collections::HashMap;

impl Variable {
    const TERMINAL_VARIABLE: Variable = Variable(u32::MAX);
    const UNDEFINED_VARIABLE: Variable = Variable(u32::MAX - 1);

    fn is_undefined(&self) -> bool {
        self == &Self::UNDEFINED_VARIABLE
    }
}

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
        Self::new(
            Variable::TERMINAL_VARIABLE,
            NodeId::TERMINAL_1,
            NodeId::TERMINAL_1,
        )
    }

    fn zero() -> Self {
        Self::new(
            Variable::TERMINAL_VARIABLE,
            NodeId::TERMINAL_0,
            NodeId::TERMINAL_0,
        )
    }
}

impl Default for Bdd {
    fn default() -> Self {
        Self::new()
    }
}

impl Bdd {
    pub fn new() -> Self {
        let terminal_0 = Node::zero();
        let terminal_1 = Node::one();

        Bdd {
            nodes: Vec::from([terminal_0, terminal_1]),
            node_table: HashMap::new(),
            task_cache: HashMap::new(),
        }
    }

    pub fn apply_recursive(&mut self, a_id: NodeId, b_id: NodeId) -> (NodeId, Node) {
        if a_id.is_terminal() && b_id.is_terminal() {
            return if a_id.is_one() && b_id.is_one() {
                (
                    NodeId::TERMINAL_1,
                    self.nodes[NodeId::TERMINAL_1.as_usize()],
                )
            } else {
                (
                    NodeId::TERMINAL_0,
                    self.nodes[NodeId::TERMINAL_0.as_usize()],
                )
            };
        }

        if let Some(found_node_id) = self.task_cache.get(&(a_id, b_id)) {
            return (*found_node_id, self.nodes[found_node_id.as_usize()]);
        }

        let a = self.nodes[a_id.as_usize()];
        let b = self.nodes[b_id.as_usize()];
        let v = min(a.variable, b.variable);

        let (low_a, high_a) = if a.variable == v {
            (a.low_child, a.high_child)
        } else {
            (a_id, a_id)
        };

        let (low_b, high_b) = if b.variable == v {
            (b.low_child, b.high_child)
        } else {
            (b_id, b_id)
        };

        let l = self.apply_recursive(low_a, low_b);
        let h = self.apply_recursive(high_a, high_b);

        let (c_node_id, c) = if l != h {
            self.ensure_node(v, l.0, h.0)
        } else {
            l
        };

        self.task_cache.insert((a_id, b_id), c_node_id);
        (c_node_id, c)
    }

    pub fn apply_iterative(&mut self, a_id: NodeId, b_id: NodeId) -> (NodeId, Node) {
        // stack contains tasks that need to be done
        let mut stack: Vec<(NodeId, NodeId, Variable)> =
            vec![(a_id, b_id, Variable::UNDEFINED_VARIABLE)];
        // results  contains results of tasks
        let mut results: Vec<(NodeId, Node)> = vec![];

        while let Some((a_id, b_id, variable)) = stack.pop() {
            if a_id.is_terminal() && b_id.is_terminal() {
                if a_id.is_one() && b_id.is_one() {
                    results.push((
                        NodeId::TERMINAL_1,
                        self.nodes[NodeId::TERMINAL_1.as_usize()],
                    ));
                } else {
                    results.push((
                        NodeId::TERMINAL_0,
                        self.nodes[NodeId::TERMINAL_0.as_usize()],
                    ));
                };
                continue;
            }

            if variable.is_undefined() {
                if let Some(found_node_id) = self.task_cache.get(&(a_id, b_id)) {
                    results.push((*found_node_id, self.nodes[found_node_id.as_usize()]));
                    continue;
                }

                let a = self.nodes[a_id.as_usize()];
                let b = self.nodes[b_id.as_usize()];
                let v = min(a.variable, b.variable);

                let (low_a, high_a) = if a.variable == v {
                    (a.low_child, a.high_child)
                } else {
                    (a_id, a_id)
                };

                let (low_b, high_b) = if b.variable == v {
                    (b.low_child, b.high_child)
                } else {
                    (b_id, b_id)
                };

                stack.push((a_id, b_id, v));
                stack.push((high_a, high_b, Variable::UNDEFINED_VARIABLE));
                stack.push((low_a, low_b, Variable::UNDEFINED_VARIABLE));

                continue;
            }

            let h = results.pop().expect("low result present in result stack");
            let l = results.pop().expect("high result present in result stack");

            let (c_node_id, c) = if l != h {
                self.ensure_node(variable, l.0, h.0)
            } else {
                l
            };

            self.task_cache.insert((a_id, b_id), c_node_id);
            results.push((c_node_id, c));
        }

        let (root_id, root) = results.pop().expect("only one result expected");
        assert!(results.is_empty());
        (root_id, root)
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
    fn apply_recursion_thesis_example() {
        let mut bdd = Bdd::new();

        let (a4_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (a3_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_1, a4_id);
        let (a2_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_0, a4_id);
        let (a1_id, _) = bdd.ensure_node(Variable(1), a2_id, a3_id);

        let (b3_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let (b2_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b1_id, _) = bdd.ensure_node(Variable(2), b2_id, b3_id);

        let (_, c1) = bdd.apply_recursive(a1_id, b1_id);
        assert_eq!(c1.variable, Variable(1));
        assert_eq!(c1.low_child.as_usize(), 0);

        let c2 = bdd.nodes[c1.high_child.as_usize()];
        assert_eq!(c2.variable, Variable(2));
        assert_eq!(c2.high_child.as_usize(), 0);

        let c3 = bdd.nodes[c2.low_child.as_usize()];
        assert_eq!(c3.variable, Variable(3));
        assert_eq!(c3.low_child.as_usize(), 0);
        assert_eq!(c3.high_child.as_usize(), 1);
    }

    #[test]
    fn apply_iterative_manual_thesis_example() {
        let mut bdd = Bdd::new();

        let (a4_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (a3_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_1, a4_id);
        let (a2_id, _) = bdd.ensure_node(Variable(2), NodeId::TERMINAL_0, a4_id);
        let (a1_id, _) = bdd.ensure_node(Variable(1), a2_id, a3_id);

        let (b3_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_1, NodeId::TERMINAL_0);
        let (b2_id, _) = bdd.ensure_node(Variable(3), NodeId::TERMINAL_0, NodeId::TERMINAL_1);
        let (b1_id, _) = bdd.ensure_node(Variable(2), b2_id, b3_id);

        let (_, c1) = bdd.apply_iterative(a1_id, b1_id);
        assert_eq!(c1.variable, Variable(1));
        assert_eq!(c1.low_child.as_usize(), 0);

        let c2 = bdd.nodes[c1.high_child.as_usize()];
        assert_eq!(c2.variable, Variable(2));
        assert_eq!(c2.high_child.as_usize(), 0);

        let c3 = bdd.nodes[c2.low_child.as_usize()];
        assert_eq!(c3.variable, Variable(3));
        assert_eq!(c3.low_child.as_usize(), 0);
        assert_eq!(c3.high_child.as_usize(), 1);
    }
}
