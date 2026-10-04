use crate::sequential::{Node, NodeId, Variable};

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
    pub const TERMINAL_0: Self = NodeId(0);
    pub const TERMINAL_1: Self = NodeId(1);

    pub fn as_usize(self) -> usize {
        self.0
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
