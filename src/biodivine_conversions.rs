use biodivine_lib_bdd::{BddNode, BddPointer, BddVariable};
use crate::{Bdd, Node, NodeId, Variable};

impl From<biodivine_lib_bdd::Bdd> for Bdd {
    fn from(value: biodivine_lib_bdd::Bdd) -> Self {
        // The two BDD implementations are very similar, meaning the translation is mostly
        // just moving data into the right structs:

        let mut result = Bdd::new();
        for biodivine_node in value.to_nodes() {
            if biodivine_node.is_terminal() {
                continue;   // Skip terminal nodes (these are already created for us).
            }
            let node: Node = biodivine_node.into();
            let node_id = NodeId(result.nodes.len());
            result.nodes.push(node);
            result.node_table.insert(node, node_id);
        }

        result
    }
}

impl From<Bdd> for biodivine_lib_bdd::Bdd {
    fn from(value: Bdd) -> Self {
        // Same as above; we are mostly just shuffling data around to move it into the right shape.

        // One special thing we do here is that we set num_vars to u16. We will do this for
        // all biodivine BDDs that we work with to make sure they are all compatible.
        let mut nodes: Vec<BddNode> = vec![
            BddNode::mk_zero(u16::MAX),
            BddNode::mk_one(u16::MAX),
        ];
        for node in value.nodes.into_iter().skip(2) {
            nodes.push(node.into());
        }
        biodivine_lib_bdd::Bdd::from_nodes(&nodes)
            .unwrap_or_else(|message| {
                panic!("Correctness violation: {message}")
            })
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

impl From<Node> for BddNode {
    fn from(value: Node) -> Self {
        BddNode::mk_node(
            value.variable.into(),
            value.low_child.into(),
            value.high_child.into()
        )
    }
}

impl From<BddVariable> for Variable {
    fn from(value: BddVariable) -> Self {
        Variable(u32::try_from(value.to_index()).expect("Correctness violation: Biodivine variables are 16-bit."))
    }
}

impl From<Variable> for BddVariable {
    fn from(value: Variable) -> Self {
        // Technically this conversion can fail, but we don't expect that to happen in tests.
        BddVariable::from_index(usize::try_from(value.0).expect("Invariant violation: usize is smaller than 32 bits."))
    }
}

impl From<BddPointer> for NodeId {
    fn from(value: BddPointer) -> Self {
        NodeId(value.to_index())
    }
}

impl From<NodeId> for BddPointer {
    fn from(value: NodeId) -> Self {
        BddPointer::from_index(value.0)
    }
}
