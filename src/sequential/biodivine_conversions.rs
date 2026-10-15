use crate::sequential::Bdd;
use crate::sequential::{Node, NodeId, Variable};
use biodivine_lib_bdd::{
    Bdd as BiodivineBdd, BddNode as BiodivineNode, BddPointer as BiodivineNodeId,
    BddVariable as BiodivineVariable,
};

impl From<BiodivineBdd> for Bdd {
    fn from(value: BiodivineBdd) -> Self {
        // The two BDD implementations are very similar, meaning the translation is mostly
        // just moving data into the right structs:

        let mut result = Bdd::new();
        for biodivine_node in value.to_nodes() {
            if biodivine_node.is_terminal() {
                continue; // Skip terminal nodes (these are already created for us).
            }
            let node: Node = biodivine_node.into();
            let node_id = NodeId(result.nodes.len());
            result.nodes.push(node);
            result.node_table.insert(node, node_id);
        }

        result
    }
}

impl From<Bdd> for BiodivineBdd {
    fn from(value: Bdd) -> Self {
        // Same as above; we are mostly just shuffling data around to move it into the right shape.

        // One special thing we do here is that we set num_vars to u16. We will do this for
        // all biodivine BDDs that we work with to make sure they are all compatible.
        let mut nodes: Vec<BiodivineNode> = vec![
            BiodivineNode::mk_zero(u16::MAX),
            BiodivineNode::mk_one(u16::MAX),
        ];
        for node in value.nodes.into_iter().skip(2) {
            nodes.push(node.into());
        }
        BiodivineBdd::from_nodes(&nodes)
            .unwrap_or_else(|message| panic!("Correctness violation: {message}"))
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

impl From<Node> for BiodivineNode {
    fn from(value: Node) -> Self {
        BiodivineNode::mk_node(
            value.variable.into(),
            value.low_child.into(),
            value.high_child.into(),
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

impl From<BiodivineNodeId> for NodeId {
    fn from(value: BiodivineNodeId) -> Self {
        NodeId(value.to_index())
    }
}

impl From<NodeId> for BiodivineNodeId {
    fn from(value: NodeId) -> Self {
        BiodivineNodeId::from_index(value.0)
    }
}
