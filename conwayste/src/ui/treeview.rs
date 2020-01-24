/*  Copyright 2020 the Conwayste Developers.
 *
 *  This file is part of conwayste.
 *
 *  conwayste is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  conwayste is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with conwayste.  If not, see
 *  <http://www.gnu.org/licenses/>. */

// KNOWN ISSUES: this code is too magical and will give the reader a headache. At least 3 espresso
// shots are recommended.

use std::cell::UnsafeCell;
use std::error::Error;
use std::rc::Rc;

use id_tree::{self, Node, NodeId, Tree, NodeIdError};

#[derive(PartialEq, Debug)]
enum Restriction {
    None,                     // TreeView has full access to the Tree
    SubTree(NodeId),          // TreeView has access to this Node and all Nodes under it
    ExclusiveSubTree(NodeId), // TreeView has access to all Nodes under this Node, but not this Node
}

impl Restriction {
    /// Returns the NodeId of the root of the TreeView if it is part of the restriction. If
    /// unrestricted, it has to be obtained from the Tree, and it may be absent.
    pub fn root(&self) -> Option<&NodeId> {
        match self {
            Restriction::None => None,
            Restriction::ExclusiveSubTree(ref root_node_id) => Some(root_node_id),
            Restriction::SubTree(ref root_node_id) => Some(root_node_id),
        }
    }
}

/// Allows getting separate mutable references to a Node and to all of the Nodes under that Node,
/// without any Node having more than one mutable reference to it at once.
pub struct TreeView<'a, T> {
    tree: Rc<UnsafeCell<&'a mut Tree<T>>>,
    restriction: Restriction,
}

impl<'a, T> TreeView<'a, T> {
    pub fn new(tree: &'a mut Tree<T>) -> Self {
        return TreeView::<'a> {
            tree: Rc::new(UnsafeCell::new(tree)),
            restriction: Restriction::None,
        };
    }

    /// Indicates whether the specified Node is accessible in this TreeView.
    ///
    /// # Errors
    ///
    /// * `NodeIdError` if the underlying Tree reports that this NodeId is invalid.
    pub fn can_access(&self, node_id: &NodeId) -> Result<bool, NodeIdError> {
        let tree = unsafe { &*self.tree.get() };

        match self.restriction {
            Restriction::None => {
                tree.get(node_id)?; // return error if the node_id is invalid
                // unrestricted, so any valid node is accessible
                return Ok(true);
            }
            Restriction::ExclusiveSubTree(ref root_node_id) => {
                if root_node_id == node_id {
                    // can't access the root node of its subtree if this is an _exclusive_ subtree
                    // restriction.
                    return Ok(false);
                }
                // If the root node is an ancestor of the referenced node in question, then it's
                // accessible.
                Ok(tree.ancestor_ids(node_id)?.any(|n| n == root_node_id))
            }
            Restriction::SubTree(ref root_node_id) => {
                if root_node_id == node_id {
                    // it's totally fine to access the root node of a non-exclusive subtree.
                    return Ok(true);
                }
                // If the root node is an ancestor of the referenced node in question, then it's
                // accessible.
                Ok(tree.ancestor_ids(node_id)?.any(|n| n == root_node_id))
            }
        }
    }

    /// Takes a mutable reference to a TreeView and returns a new TreeView that is restricted to
    /// all Nodes below the specified Node. It takes a mutable reference to prevent Node aliasing.
    ///
    /// # Errors
    ///
    /// * NodeId is invalid for the underlying Tree.
    /// * NodeId refers to a Node that is outside of this TreeView.
    pub fn sub_tree(
        &mut self,
        node_id: &NodeId,
    ) -> Result<(&mut Node<T>, TreeView<'a, T>), Box<dyn Error>> {
        if !self.can_access(node_id)? {
            return Err("the TreeView does not have access to the specified Node".into());
        }

        let tree_mut_ref = unsafe { &mut *self.tree.get() };
        let node_mut_ref = tree_mut_ref.get_mut(node_id)?;
        let subtree = TreeView {
            tree: self.tree.clone(),
            restriction: Restriction::ExclusiveSubTree(node_id.clone()),
        };
        Ok((node_mut_ref, subtree))
    }

    /// If this tree has any Nodes at all, this will return (as Some) an iterator over the root
    /// node's children.
    pub fn children(&self) -> Option<id_tree::Children<T>> {
        let tree = unsafe { &*self.tree.get() };
        if let Some(root_id) = self.restriction.root() {
            Some(tree.children(root_id).unwrap())
        } else {
            tree.root_node_id()
                .map(|root_id| {
                    tree.children(root_id).unwrap()
                })
        }
    }

    //XXX children_ids

    /////// following only ok if .can_access
    //XXX get
    //XXX get_mut


    /////// following only ok if restriction is None
    //XXX insert
    //XXX remove
}
