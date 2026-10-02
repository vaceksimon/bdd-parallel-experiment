pub mod common;

pub mod parallel;
pub mod sequential;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct NodeId(usize);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Variable(u32);

#[derive(Copy, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug)]
pub struct Node {
    variable: Variable,
    low_child: NodeId,
    high_child: NodeId,
}
