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

///! This module implements a `TreeView` over an `id_tree::Tree`. It uses `unsafe` to do what the
///! Rust aliasing rules would usually not allow, but in a (hopefully) sound way.
///!
///! It supports operations that modify the `Tree` structure, but only when there are no other
///! references to parts of the `Tree`.
///!
///! # Known issues
///!
///! * The assumption here is that a `.get_mut` on an `id_tree::Tree` does not modify anything in
///!   the `Tree` (for example, it doesn't alter any bookkeeping counters or whatever) that is read
///!   by common immutable operations on the `Tree`. If this assumption holds, it is sound to
///!   have one or more `&Node` references while there is one or more `&mut Node` (as long as the
///!   Nodes are different, which the `TreeView` checks). Undefined behavior will result if this
///!   assumption does not hold!
///!
///! * This code is too magical and will give the reader a headache. At least 3 espresso
///!   shots are recommended.


use std::error::Error;
use std::marker::PhantomData;
use std::ptr::NonNull;

use id_tree::{self, Node, NodeId, NodeIdError, Tree};

#[derive(PartialEq, Debug)]
enum Restriction {
    None,                     // TreeView has full access to the Tree
    SubTree(NodeId),          // TreeView has access to all Nodes under this Node, but not this Node
    InclusiveSubTree(NodeId), // TreeView has access to this Node and all Nodes under it // TODO: need method for this?
}

impl Restriction {
    /// Returns the NodeId of the root of the TreeView if it is part of the restriction. If
    /// unrestricted, it has to be obtained from the Tree, and it may be absent.
    pub fn root(&self) -> Option<&NodeId> {
        match self {
            Restriction::None => None,
            Restriction::SubTree(ref root_node_id) => Some(root_node_id),
            Restriction::InclusiveSubTree(ref root_node_id) => Some(root_node_id),
        }
    }
}

/// A view onto a Tree, either in whole or a subtree thereof. A TreeView can be split up into a
/// mutable Node reference and a TreeView on the Nodes under that Node; in other words, it supports
/// multiple non-overlapping mutable references to Nodes in a Tree.
pub struct TreeView<'a, T: 'a> {
    tree: NonNull<Tree<T>>,
    lifetime: PhantomData<&'a mut T>,
    restriction: Restriction,
}

impl<'a, T> TreeView<'a, T> {
    pub fn new(tree: &'a mut Tree<T>) -> Self {
        return TreeView::<'a> {
            tree: NonNull::new(tree).unwrap(), // unwrap OK because the tree reference will never be null
            lifetime: PhantomData,
            restriction: Restriction::None,
        };
    }

    /// Indicates whether the specified Node is accessible in this TreeView.
    ///
    /// # Errors
    ///
    /// * `NodeIdError` if the underlying Tree reports that this NodeId is invalid.
    pub fn can_access(&self, node_id: &NodeId) -> Result<bool, NodeIdError> {
        let tree = unsafe { self.tree.as_ref() };

        match self.restriction {
            Restriction::None => {
                tree.get(node_id)?; // return error if the node_id is invalid
                                    // unrestricted, so any valid node is accessible
                return Ok(true);
            }
            Restriction::SubTree(ref root_node_id) => {
                if root_node_id == node_id {
                    // can't access the root node of its subtree if this is an _exclusive_ subtree
                    // restriction.
                    return Ok(false);
                }
                // If the root node is an ancestor of the referenced node in question, then it's
                // accessible.
                Ok(tree.ancestor_ids(node_id)?.any(|n| n == root_node_id))
            }
            Restriction::InclusiveSubTree(ref root_node_id) => {
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
    /// It is analogous to the .split_at_mut method of a slice. Like that method, it
    /// (unfortunately) has to use `unsafe`.
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

        let subtree = TreeView {
            tree: self.tree,
            lifetime: PhantomData,
            restriction: Restriction::SubTree(node_id.clone()),
        };
        let tree_mut_ref = unsafe { self.tree.as_mut() };
        let node_mut_ref = tree_mut_ref.get_mut(node_id)?;
        Ok((node_mut_ref, subtree))
    }

    /// If this tree has any Nodes at all, this will return (as Some) an iterator over the root
    /// node's children.
    pub fn children(&self) -> Option<id_tree::Children<T>> {
        let tree = unsafe { self.tree.as_ref() };
        if let Some(root_id) = self.restriction.root() {
            Some(tree.children(root_id).unwrap())
        } else {
            tree.root_node_id()
                .map(|root_id| tree.children(root_id).unwrap())
        }
    }

    /// If this tree has any Nodes at all, this will return (as Some) an iterator over the NodeIds
    /// of the root node's children.
    pub fn children_ids(&self) -> Option<id_tree::ChildrenIds> {
        let tree = unsafe { self.tree.as_ref() };
        if let Some(root_id) = self.restriction.root() {
            Some(tree.children_ids(root_id).unwrap())
        } else {
            tree.root_node_id()
                .map(|root_id| tree.children_ids(root_id).unwrap())
        }
    }

    /// Get an immutable reference to a Node. The specified Node must be accessible.
    pub fn get(&self, node_id: &NodeId) -> Result<&Node<T>, Box<dyn Error>> {
        let tree = unsafe { self.tree.as_ref() };

        if !self.can_access(node_id)? {
            return Err("the TreeView does not have access to the specified Node".into());
        }

        tree.get(node_id).map_err(|e| e.into())
    }

    /// Get a mutable reference to a Node. The specified Node must be accessible.
    pub fn get_mut(&mut self, node_id: &NodeId) -> Result<&mut Node<T>, Box<dyn Error>> {
        let tree = unsafe { self.tree.as_mut() };

        if !self.can_access(node_id)? {
            return Err("the TreeView does not have access to the specified Node".into());
        }

        tree.get_mut(node_id).map_err(|e| e.into())
    }

    /////// XXX following only ok if restriction is None

    //XXX insert
    //XXX remove
}
