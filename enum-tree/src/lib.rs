#[macro_use]
extern crate enum_tree_derive;

pub use enum_tree_derive::EnumTree;

pub trait EnumTree {
    fn enum_tree() -> EnumTreeNode;
}

#[derive(Debug)]
pub struct EnumTreeNode {
    pub name: String,
    pub subnodes: Vec<EnumTreeNode>,

    // XXX does it make sense to encode the type into the node for input validation?
}