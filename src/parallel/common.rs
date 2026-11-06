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
    pub const TERMINAL_0: Self = NodeId(uuid!("4d2e9633-9013-58b4-bacd-23fe7e6219b5"));
    pub const TERMINAL_1: Self = NodeId(uuid!("2f74ec4e-0603-5c74-b8c4-680f22547e65"));

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

    fn shorten(&self) -> String {
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

impl TaskId {
    pub const TERMINAL: Self = Self(NodeId::TERMINAL_0, NodeId::TERMINAL_1);
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl SuccessorResults {
    pub const TERMINAL: Self = Self(TaskId::TERMINAL, TaskId::TERMINAL);

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
        let zero = NodeId::new(
            NodeId(uuid!("00000000-0000-0000-0000-000000000000")),
            NodeId(uuid!("00000000-0000-0000-0000-000000000000")),
        );
        let computed = NodeId::new(zero, zero);

        assert_eq!(NodeId::TERMINAL_0, computed);
    }

    #[test]
    fn test_terminal_1_matches_runtime_computation() {
        let one = NodeId::new(
            NodeId(uuid!("00000000-0000-0000-0000-000000000001")),
            NodeId(uuid!("00000000-0000-0000-0000-000000000001")),
        );
        let computed = NodeId::new(one, one);

        assert_eq!(NodeId::TERMINAL_1, computed);
    }
}
