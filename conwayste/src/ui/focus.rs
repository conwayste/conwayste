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

use id_tree::NodeId;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CycleType {
    TopLevel,
    NonTopLevel,
}

/// Represents keyboard focus changes for a cycle of widgets. Focus would typically be changed by
/// clicking on a widget, or pressing Tab, Ctrl-Tab, or Esc.
#[derive(Clone, Debug)]
pub struct FocusCycle {
    index: Option<usize>,
    ids: Vec<NodeId>,
    pub cycle_type: CycleType,
}

// TODO: add top_level bool (Layering will have it be true, Pane will have it be false)
impl FocusCycle {
    pub fn new(cycle_type: CycleType) -> Self {
        FocusCycle {
            index: None,
            ids: vec![],
            cycle_type,
        }
    }

    /// Return an Option containing the ID of the currently focused widget, if one exists.
    pub fn focused_widget_id(&self) -> Option<&NodeId> {
        self.index.map(|idx| &self.ids[idx])
    }

    /// Sets the currently focused widget ID. The return value is (was_successful, old_focused)
    /// where old_focused is the previously focused widget ID, and was_successful indicates
    /// whether the focus setting succeeded. It can only fail if the widget ID is not found in
    /// this FocusCycle.
    pub fn set_focused(&mut self, node_id: &NodeId) -> (bool, Option<NodeId>) {
        let old_focused_widget = self.focused_widget_id().map(|id| id.clone());
        let mut was_successful = false;
        if let Some(found_idx) = self.find(node_id) {
            was_successful = true;
            self.index = Some(found_idx);
        }
        (was_successful, old_focused_widget)
    }

    /// Clears the focus.
    pub fn clear_focus(&mut self) {
        self.index = None;
    }

    /// Focus the next widget in the focus cycle. If none were focused, focus the first. Does
    /// nothing if there are no IDs.
    /// Returns true if focus became lost due to reaching the end.
    pub fn focus_next(&mut self) -> bool {
        // TODO: cycle type
        if self.ids.len() == 0 {
            return false;
        }
        if let Some(idx) = self.index {
            if self.cycle_type == CycleType::TopLevel {
                self.index = Some((idx + 1) % self.ids.len());
            } else {
                // non-top level -- no looping
                let next_idx = idx + 1;
                if idx == self.ids.len() {
                    self.index = None;
                    return true;
                }
                self.index = Some(next_idx);
            }
        } else {
            self.index = Some(0);
        }
        false
    }

    /// Focus the previous widget in the focus cycle. If none were focused, focus the last. Does
    /// nothing if there are no IDs.
    /// Returns true if focus became lost due to reaching the beginning.
    pub fn focus_previous(&mut self) -> bool {
        if self.ids.len() == 0 {
            return false;
        }
        if let Some(idx) = self.index {
            if self.cycle_type == CycleType::TopLevel {
                self.index = Some((idx + self.ids.len() - 1) % self.ids.len());
            } else {
                // non-top level -- no looping
                if idx == 0 {
                    self.index = None;
                    return true;
                }
                self.index = Some(idx - 1);
            }
        } else {
            self.index = Some(self.ids.len() - 1);
        }
        false
    }

    /// Append a widget ID to the focus cycle.
    pub fn push(&mut self, node_id: NodeId) {
        self.ids.push(node_id);
    }

    /// Find the index of the specified widget ID.
    pub fn find(&self, node_id: &NodeId) -> Option<usize> {
        for i in 0..self.ids.len() {
            if self.ids[i] == *node_id {
                return Some(i);
            }
        }
        None
    }

    /// Remove a widget ID from the focus cycle. If the widget ID does not exist in the focus
    /// cycle, nothing happens. If the widget ID is the one that currently has focus, nothing is
    /// focused.
    pub fn remove(&mut self, node_id: &NodeId) {
        // remove
        if let Some(remove_idx) = self.find(node_id) {
            self.ids.remove(remove_idx);

            if let Some(focused_idx) = self.index {
                if focused_idx > remove_idx {
                    self.index = Some(focused_idx - 1);
                } else if focused_idx == remove_idx {
                    self.index = None;
                }
            }
        }
    }

    /// Return an immutable slice of all NodeIds in the focus cycle.
    pub fn as_slice(&self) -> &[NodeId] {
        self.ids.as_slice()
    }
}
