use std::net::SocketAddr;

use enum_tree::{EnumTree, EnumTreeNode};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Endpoint(pub SocketAddr);

impl EnumTree for Endpoint {
    fn enum_tree() -> EnumTreeNode {
        EnumTreeNode {
            name:     "Endpoint".to_owned(),
            subnodes: vec![],
        }
    }
}
