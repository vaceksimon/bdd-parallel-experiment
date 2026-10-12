use crate::parallel::{Bdd, Node, NodeId, Variable};
use biodivine_lib_bdd::{BddNode, BddPointer, BddVariable};
use std::collections::HashMap;
use uuid::Uuid;

impl Bdd {
    /// Manages the mapping of node IDs to BDD pointers for conversion Bdd -> BiodivineBdd
    fn convert_node_id(
        node_id: NodeId,
        id_map: &mut HashMap<NodeId, BddPointer>,
        current_id: &mut usize,
    ) -> BddPointer {
        if let Some(id) = id_map.get(&node_id) {
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

            id_map.insert(node_id, new_id);
            new_id
        }
    }

    pub(super) fn into_biodivine_bdd(self, root: NodeId) -> biodivine_lib_bdd::Bdd {
        let mut id_map: HashMap<NodeId, BddPointer> = HashMap::new();
        let mut nodes: Vec<BddNode> = vec![BddNode::mk_zero(u16::MAX), BddNode::mk_one(u16::MAX)];
        let mut current_id: usize = 2;

        let mut stack: Vec<NodeId> = Vec::with_capacity(self.nodes.len());
        stack.push(root);

        while let Some(node_id) = stack.pop() {
            let converted_node_id = Self::convert_node_id(node_id, &mut id_map, &mut current_id);

            if converted_node_id.to_index() > current_id {
                let node = *self.nodes.get(&node_id).unwrap().value();

                let low_node_id = node.low_child;
                let high_node_id = node.high_child;
                stack.push(high_node_id);
                stack.push(low_node_id);

                let converted_variable = node.variable.into();
                let converted_low_node_id =
                    Self::convert_node_id(node.low_child, &mut id_map, &mut current_id);
                let converted_high_node_id =
                    Self::convert_node_id(node.high_child, &mut id_map, &mut current_id);
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

impl From<biodivine_lib_bdd::Bdd> for Bdd {
    fn from(value: biodivine_lib_bdd::Bdd) -> Self {
        // The two BDD implementations are very similar, meaning the translation is mostly
        // just moving data into the right structs:

        let result = Bdd::new();
        for biodivine_node in value.to_nodes() {
            if biodivine_node.is_terminal() {
                continue; // Skip terminal nodes (these are already created for us).
            }
            let node: Node = biodivine_node.into();
            let node_id = result.nodes.len().into();
            result.nodes.insert(node_id, node);
            result.node_table.insert(node, node_id);
        }

        result
    }
}

impl From<BddNode> for Node {
    fn from(value: BddNode) -> Self {
        Node::new(
            value.var.into(),
            value.low_link.into(),
            value.high_link.into(),
        )
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

impl From<usize> for NodeId {
    fn from(value: usize) -> Self {
        NodeId(Uuid::from_u128(value as u128))
    }
}

impl From<BddPointer> for NodeId {
    fn from(value: BddPointer) -> Self {
        value.to_index().into()
    }
}
