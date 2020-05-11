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

use id_tree::{self, InsertBehavior, Node, NodeId, NodeIdError, RemoveBehavior, Tree};

#[allow(unused)]
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

    #[allow(unused)]
    pub fn is_none(&self) -> bool {
        match self {
            Restriction::None => true,
            _ => false,
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
    ///
    /// NOTE: the `'_` in the returned subtree is needed to make this fail to compile:
    ///
    /// ```compile_fail
    /// let mut tree: Tree<i64> = id_tree::TreeBuilder::new().build();
    /// let node_r = Node::new(0);
    /// let root_node_id = tree.insert(node_r, InsertBehavior::AsRoot).unwrap();
    /// let child_node_id = tree.insert(Node::new(1), InsertBehavior::UnderNode(&root_node_id)).unwrap();
    /// let mut view = TreeView::new(&mut tree);
    /// let (_, mut copy1) = view.sub_tree(&root_node_id).unwrap();
    /// let (_, mut copy2) = view.sub_tree(&root_node_id).unwrap();
    /// let alias1 = copy1.get_mut(&child_node_id).unwrap();
    /// let alias2 = copy2.get_mut(&child_node_id).unwrap();
    /// drop((alias1, alias2));
    /// ```
    pub fn sub_tree(
        &mut self,
        node_id: &NodeId,
    ) -> Result<(&mut Node<T>, TreeView<'_, T>), Box<dyn Error>> {
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
    #[allow(unused)]
    pub fn children(&self) -> Option<id_tree::Children<T>> {
        let tree = unsafe { self.tree.as_ref() };
        if let Some(root_id) = self.restriction.root() {
            Some(tree.children(root_id).unwrap())
        } else {
            tree.root_node_id()
                .map(|root_id| tree.children(root_id).unwrap())
        }
    }

    /// Return a vector of the NodeIds of the root node's children if this is a view of the entire
    /// tree, or else a vector of the NodeIds of the children of whichever Node this view is
    /// restricted on.
    pub fn children_ids(&self) -> Vec<NodeId> {
        let tree = unsafe { self.tree.as_ref() };
        self.restriction
            .root()
            .or(tree.root_node_id())
            .map(|root_id| {
                tree.children_ids(root_id)
                    .unwrap() // unwrap OK because children_ids only errors if the passed in NodeId is invalid
                    .map(|id| id.clone())
                    .collect()
            })
            .or_else(|| Some(vec![])) // empty tree
            .unwrap() // unwrap OK because it will always be Some() due to the .or_else above
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
        if !self.can_access(node_id)? {
            return Err("the TreeView does not have access to the specified Node".into());
        }

        let tree = unsafe { self.tree.as_mut() };

        tree.get_mut(node_id).map_err(|e| e.into())
    }

    /// Inserts a node into the underlying Tree. See documentation for Tree.insert.
    ///
    /// # Errors
    ///
    /// In addition to errors from Tree.insert, an error is returned when the TreeView is
    /// sharing the Tree.
    pub fn insert(
        &mut self,
        node: Node<T>,
        behavior: InsertBehavior,
    ) -> Result<NodeId, Box<dyn Error>> {
        if !self.restriction.is_none() {
            return Err("the TreeView must have full access to the Tree to insert a node".into());
        }

        let tree = unsafe { self.tree.as_mut() };

        Ok(tree.insert(node, behavior)?)
    }

    /// Removes a node from the underlying Tree. See documentation for Tree.remove_node.
    ///
    /// # Errors
    ///
    /// In addition to errors from Tree.remove_node, an error is returned when the TreeView is
    /// sharing the Tree.
    pub fn remove(
        &mut self,
        node_id: NodeId,
        behavior: RemoveBehavior,
    ) -> Result<Node<T>, Box<dyn Error>> {
        if !self.restriction.is_none() {
            return Err("the TreeView must have full access to the Tree to remove a node".into());
        }

        let tree = unsafe { self.tree.as_mut() };

        Ok(tree.remove_node(node_id, behavior)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use id_tree::InsertBehavior;

    #[test]
    fn test_subtree_and_children_ids() {
        let mut tree: Tree<i64> = id_tree::TreeBuilder::new().build();
        let node_r = Node::new(0);
        let root_node_id = tree.insert(node_r, InsertBehavior::AsRoot).unwrap();

        let child_nodes = vec![Node::new(1), Node::new(2), Node::new(3)];

        for c in child_nodes.into_iter() {
            tree.insert(c, InsertBehavior::UnderNode(&root_node_id))
                .unwrap();
        }

        let mut view = TreeView::new(&mut tree);

        let (root_node_ref, mut sub_tree_ref) = view.sub_tree(&root_node_id).unwrap();
        for child_id in sub_tree_ref.children_ids().iter() {
            let child_ref = sub_tree_ref.get_mut(child_id).unwrap().data_mut();
            // root += child
            *root_node_ref.data_mut() += *child_ref;
            // zero out child
            *child_ref = 0;
        }
        drop(view);
        assert_eq!(*tree.get(&root_node_id).unwrap().data(), 1 + 2 + 3);
    }

    /*
    #[test]
    fn test_compile_fail_double_mut_borrow() {
        let mut tree: Tree<i64> = id_tree::TreeBuilder::new().build();
        let node_r = Node::new(0);
        let root_node_id = tree.insert(node_r, InsertBehavior::AsRoot).unwrap();
        let child_node_id = tree.insert(Node::new(1), InsertBehavior::UnderNode(&root_node_id)).unwrap();
        let mut view = TreeView::new(&mut tree);
        let (_, mut copy1) = view.sub_tree(&root_node_id).unwrap();
        let (_, mut copy2) = view.sub_tree(&root_node_id).unwrap();
        let alias1 = copy1.get_mut(&child_node_id).unwrap();
        let alias2 = copy2.get_mut(&child_node_id).unwrap();
        drop((alias1, alias2));
    }
    */
}
