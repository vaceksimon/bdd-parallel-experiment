use crate::parallel::{Bdd, Node, NodeId, Variable};
use biodivine_lib_bdd::{
    Bdd as BiodivineBdd, BddNode as BiodivineNode, BddPointer as BiodivineNodeId,
    BddVariable as BiodivineVariable,
};
use std::collections::HashMap;
use uuid::Uuid;

impl Bdd {
    /// Manages the mapping of node IDs to BDD pointers for conversion Bdd -> BiodivineBdd
    fn convert_node_id(
        node_id: NodeId,
        id_map: &mut HashMap<NodeId, BiodivineNodeId>,
        current_id: &mut usize,
    ) -> BiodivineNodeId {
        if let Some(biodivine_node_id) = id_map.get(&node_id) {
            *biodivine_node_id
        } else {
            let new_id: BiodivineNodeId = if node_id.is_terminal() {
                if node_id.is_one() {
                    BiodivineNodeId::one()
                } else {
                    BiodivineNodeId::zero()
                }
            } else {
                let new_id: BiodivineNodeId = BiodivineNodeId::from_index(*current_id);
                *current_id += 1;
                new_id
            };

            id_map.insert(node_id, new_id);
            new_id
        }
    }

    pub(super) fn into_biodivine_bdd(self, root: NodeId) -> BiodivineBdd {
        let mut id_map: HashMap<NodeId, BiodivineNodeId> = HashMap::new();
        let mut nodes: Vec<BiodivineNode> = vec![
            BiodivineNode::mk_zero(u16::MAX),
            BiodivineNode::mk_one(u16::MAX),
        ];
        let mut current_id: usize = 2;

        let mut stack: Vec<NodeId> = Vec::with_capacity(self.nodes.len());
        stack.push(root);

        while let Some(node_id) = stack.pop() {
            let converted_node_id: BiodivineNodeId =
                Self::convert_node_id(node_id, &mut id_map, &mut current_id);

            if converted_node_id.to_index() > current_id {
                let node = *self.nodes.get(&node_id).unwrap().value();

                let low_node_id = node.low_child;
                let high_node_id = node.high_child;
                stack.push(high_node_id);
                stack.push(low_node_id);

                let converted_variable: BiodivineVariable = node.variable.into();
                let converted_low_node_id: BiodivineNodeId =
                    Self::convert_node_id(node.low_child, &mut id_map, &mut current_id);
                let converted_high_node_id: BiodivineNodeId =
                    Self::convert_node_id(node.high_child, &mut id_map, &mut current_id);
                nodes.insert(
                    converted_node_id.to_index(),
                    BiodivineNode::mk_node(
                        converted_variable,
                        converted_low_node_id,
                        converted_high_node_id,
                    ),
                );
            } else {
                continue;
            }
        }

        BiodivineBdd::from_nodes(&nodes)
            .unwrap_or_else(|message| panic!("Correctness violation: {message}"))
    }
}

impl From<BiodivineBdd> for Bdd {
    fn from(value: BiodivineBdd) -> Self {
        // The two BDD implementations are very similar, meaning the translation is mostly
        // just moving data into the right structs:

        let result = Bdd::new();
        for biodivine_node in value.to_nodes() {
            if biodivine_node.is_terminal() {
                continue; // Skip terminal nodes (these are already created for us).
            }
            let node: Node = biodivine_node.into();
            let node_id: NodeId = result.nodes.len().into();
            result.nodes.insert(node_id, node);
            result.node_table.insert(node, node_id);
        }

        result
    }
}

impl From<BiodivineNode> for Node {
    fn from(value: BiodivineNode) -> Self {
        Node::new(
            value.var.into(),
            value.low_link.into(),
            value.high_link.into(),
        )
    }
}

impl From<BiodivineVariable> for Variable {
    fn from(value: BiodivineVariable) -> Self {
        Variable(
            u32::try_from(value.to_index())
                .expect("Correctness violation: Biodivine variables are 16-bit."),
        )
    }
}

impl From<Variable> for BiodivineVariable {
    fn from(value: Variable) -> Self {
        // Technically this conversion can fail, but we don't expect that to happen in tests.
        BiodivineVariable::from_index(
            usize::try_from(value.0).expect("Invariant violation: usize is smaller than 32 bits."),
        )
    }
}

impl From<usize> for NodeId {
    fn from(value: usize) -> Self {
        NodeId(Uuid::from_u128(value as u128))
    }
}

impl From<BiodivineNodeId> for NodeId {
    fn from(value: BiodivineNodeId) -> Self {
        value.to_index().into()
    }
}
