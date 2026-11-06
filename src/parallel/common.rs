use crate::parallel::{Bdd, Node, NodeId, SuccessorResults, TaskId, Variable};
use std::fmt::Display;
use uuid::{Uuid, uuid};

impl Variable {
    pub const TERMINAL_VARIABLE: Variable = Variable(u32::MAX);
    pub const UNDEFINED_VARIABLE: Variable = Variable(u32::MAX - 1);

    pub fn is_terminal(&self) -> bool {
        self == &Self::TERMINAL_VARIABLE
    }

    pub fn is_undefined(&self) -> bool {
        self == &Self::UNDEFINED_VARIABLE
    }
}

impl NodeId {
    pub const TERMINAL_0: Self = NodeId(uuid!("f56f7e63-aa4f-590f-90d6-bc9c11beb92d"));
    pub const TERMINAL_1: Self = NodeId(uuid!("957cf0cd-0ea4-5a8e-b9da-b04d52ea19b7"));

    pub fn new(x: NodeId, y: NodeId) -> Self {
        let mut combined_bytes = [0u8; 32];
        combined_bytes[..16].copy_from_slice(x.0.as_bytes());
        combined_bytes[16..].copy_from_slice(y.0.as_bytes());
        Self(Uuid::new_v5(&Uuid::NAMESPACE_DNS, &combined_bytes))
    }

    pub fn is_terminal(&self) -> bool {
        self == &Self::TERMINAL_0 || self == &Self::TERMINAL_1
    }

    #[cfg(test)]
    pub fn is_zero(self) -> bool {
        self == Self::TERMINAL_0
    }

    pub fn is_one(self) -> bool {
        self == Self::TERMINAL_1
    }

    pub(crate) fn shorten(&self) -> String {
        let s = self.0.to_string();
        let value = &s[s.len() - 12..];
        value.to_string()
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.shorten())
    }
}

impl Node {
    pub fn new(variable: Variable, low_child: NodeId, high_child: NodeId) -> Self {
        Self {
            variable,
            low_child,
            high_child,
        }
    }

    pub fn one() -> Self {
        Self::new(
            Variable::TERMINAL_VARIABLE,
            NodeId::TERMINAL_1,
            NodeId::TERMINAL_1,
        )
    }

    pub fn zero() -> Self {
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

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl SuccessorResults {
    pub fn new(a: TaskId, b: TaskId) -> Self {
        Self(a, b)
    }
}

impl Display for SuccessorResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl From<SuccessorResults> for (TaskId, TaskId) {
    fn from(value: SuccessorResults) -> Self {
        (value.0, value.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortened_node_id() {
        let zero = NodeId(uuid!("00000000-0000-0000-0000-000000000000"));
        assert_eq!("000000000000", zero.shorten());

        let one = NodeId(uuid!("00000000-0000-0000-0000-000000000001"));
        assert_eq!("000000000001", one.shorten());

        let random = NodeId(uuid!("99c3282d-3297-5c9c-bb99-f25529d406ad"));
        assert_eq!("f25529d406ad", random.shorten());
    }

    #[test]
    fn test_terminal_0_matches_runtime_computation() {
        let zero_uuid = uuid!("00000000-0000-0000-0000-000000000000");

        let zero_node_id = NodeId::new(NodeId(zero_uuid), NodeId(zero_uuid));
        assert_eq!(NodeId::TERMINAL_0, zero_node_id);
    }

    #[test]
    fn test_terminal_1_matches_runtime_computation() {
        let one_uuid = uuid!("00000000-0000-0000-0000-000000000001");

        let one_node_id = NodeId::new(NodeId(one_uuid), NodeId(one_uuid));
        assert_eq!(NodeId::TERMINAL_1, one_node_id);
    }
}
