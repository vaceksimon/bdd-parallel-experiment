use crate::parallel::{Bdd, Node, NodeId, Variable};
use biodivine_lib_bdd::{BddNode, BddPointer, BddVariable};
use std::collections::HashMap;

impl From<biodivine_lib_bdd::Bdd> for Bdd {
    fn from(value: biodivine_lib_bdd::Bdd) -> Self {
        let mut converted_node_ids: HashMap<BddPointer, NodeId> = HashMap::new();

        let mut get_node_id = |node_id: BddPointer| -> NodeId {
            if let Some(id) = converted_node_ids.get(&node_id) {
                *id
            } else {
                let new_id = if node_id.is_terminal() {
                    if node_id.is_one() {
                        NodeId::TERMINAL_1
                    } else {
                        NodeId::TERMINAL_0
                    }
                } else {
                    NodeId::new()
                };

                converted_node_ids.insert(node_id, new_id);
                new_id
            }
        };

        let result = Bdd::new();

        let mut stack: Vec<BddPointer> = Vec::with_capacity(value.size());
        stack.push(value.root_pointer());
        let biodivine_nodes = value.to_nodes();

        while let Some(biodivine_node_id) = stack.pop() {
            let biodivine_node = biodivine_nodes[biodivine_node_id.to_index()];

            let biodivine_low_node_id = biodivine_node.low_link;
            let biodivine_high_node_id = biodivine_node.high_link;
            if !biodivine_high_node_id.is_terminal() {
                stack.push(biodivine_high_node_id);
            }
            if !biodivine_low_node_id.is_terminal() {
                stack.push(biodivine_low_node_id);
            }

            let converted_node_id = get_node_id(biodivine_node_id);
            if result.nodes.get(&converted_node_id).is_none() {
                let converted_variable = biodivine_node.var.into();
                let converted_low_node_id = get_node_id(biodivine_low_node_id);
                let converted_high_node_id = get_node_id(biodivine_high_node_id);

                let converted_node = Node::new(
                    converted_variable,
                    converted_low_node_id,
                    converted_high_node_id,
                );
                result.nodes.insert(converted_node_id, converted_node);
                result.node_table.insert(converted_node, converted_node_id);
            } else {
                continue;
            }
        }
        result
    }
}

impl Bdd {
    pub(crate) fn into_biodivine_bdd(self, root: NodeId) -> biodivine_lib_bdd::Bdd {
        let mut converted_node_ids: HashMap<NodeId, BddPointer> = HashMap::new();
        let mut nodes: Vec<BddNode> = vec![BddNode::mk_zero(u16::MAX), BddNode::mk_one(u16::MAX)];
        let mut current_id: usize = 2;

        let mut get_node_id = |node_id: NodeId, current_id: &mut usize| -> BddPointer {
            if let Some(id) = converted_node_ids.get(&node_id) {
                *id
            } else {
                let new_id = if node_id.is_terminal() {
                    if node_id.is_one() {
                        BddPointer::one()
                    } else {
                        BddPointer::zero()
                    }
                } else {
                    let new_id = BddPointer::from_index(*current_id);
                    *current_id += 1;
                    new_id
                };

                converted_node_ids.insert(node_id, new_id);
                new_id
            }
        };

        let mut stack: Vec<NodeId> = Vec::with_capacity(self.nodes.len());
        stack.push(root);

        while let Some(node_id) = stack.pop() {
            let node = *self.nodes.get(&node_id).unwrap().value();

            let low_node_id = node.low_child;
            let high_node_id = node.high_child;
            stack.push(high_node_id);
            stack.push(low_node_id);

            let converted_node_id = get_node_id(node_id, &mut current_id);
            if converted_node_id.to_index() > current_id {
                let converted_variable = node.variable.into();
                let converted_low_node_id = get_node_id(node.low_child, &mut current_id);
                let converted_high_node_id = get_node_id(node.high_child, &mut current_id);
                nodes.insert(
                    converted_node_id.to_index(),
                    BddNode::mk_node(
                        converted_variable,
                        converted_low_node_id,
                        converted_high_node_id,
                    ),
                );
            } else {
                continue;
            }
        }

        biodivine_lib_bdd::Bdd::from_nodes(&nodes)
            .unwrap_or_else(|message| panic!("Correctness violation: {message}"))
    }
}

impl From<BddVariable> for Variable {
    fn from(value: BddVariable) -> Self {
        Variable(
            u32::try_from(value.to_index())
                .expect("Correctness violation: Biodivine variables are 16-bit."),
        )
    }
}

impl From<Variable> for BddVariable {
    fn from(value: Variable) -> Self {
        // Technically this conversion can fail, but we don't expect that to happen in tests.
        BddVariable::from_index(
            usize::try_from(value.0).expect("Invariant violation: usize is smaller than 32 bits."),
        )
    }
}
