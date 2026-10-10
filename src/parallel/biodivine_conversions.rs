use crate::parallel::{Bdd, Node, NodeId, Variable};
use biodivine_lib_bdd::{BddNode, BddPointer, BddVariable};
use rand::RngExt;
use std::collections::HashMap;

impl From<biodivine_lib_bdd::Bdd> for Bdd {
    fn from(value: biodivine_lib_bdd::Bdd) -> Self {
        enum IdConversionResult {
            NewId(NodeId),
            ExistingId(NodeId),
        }
        impl From<IdConversionResult> for NodeId {
            fn from(value: IdConversionResult) -> Self {
                match value {
                    IdConversionResult::NewId(id) => id,
                    IdConversionResult::ExistingId(id) => id,
                }
            }
        }

        let mut converted_node_ids: HashMap<BddPointer, NodeId> = HashMap::new();

        let mut get_node_id = |node_id: BddPointer| -> IdConversionResult {
            if let Some(id) = converted_node_ids.get(&node_id) {
                IdConversionResult::ExistingId(*id)
            } else {
                let new_id = NodeId::new();
                converted_node_ids.insert(node_id, new_id);
                IdConversionResult::NewId(new_id)
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
            stack.push(biodivine_low_node_id);
            stack.push(biodivine_high_node_id);

            if let IdConversionResult::NewId(converted_node_id) = get_node_id(biodivine_node_id) {
                let converted_variable = biodivine_node.var.into();
                let converted_low_node_id = get_node_id(biodivine_low_node_id).into();
                let converted_high_node_id = get_node_id(biodivine_high_node_id).into();

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

impl From<Bdd> for biodivine_lib_bdd::Bdd {
    fn from(value: Bdd) -> Self {
        // Same as above; we are mostly just shuffling data around to move it into the right shape.

        // One special thing we do here is that we set num_vars to u16. We will do this for
        // all biodivine BDDs that we work with to make sure they are all compatible.
        let mut nodes: Vec<BddNode> = vec![BddNode::mk_zero(u16::MAX), BddNode::mk_one(u16::MAX)];
        for (_, node) in value.nodes.into_iter().skip(2) {
            nodes.push(node.into());
        }
        biodivine_lib_bdd::Bdd::from_nodes(&nodes)
            .unwrap_or_else(|message| panic!("Correctness violation: {message}"))
    }
}

impl From<Node> for BddNode {
    fn from(value: Node) -> Self {
        BddNode::mk_node(
            value.variable.into(),
            value.low_child.into(),
            value.high_child.into(),
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

impl From<NodeId> for BddPointer {
    fn from(_value: NodeId) -> Self {
        let id: usize = rand::rng().random_range(0..usize::MAX);
        Self::from_index(id)
    }
}
