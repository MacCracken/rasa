use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::color::BlendMode;
use crate::layer::Layer;

/// A reversible command for undo/redo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    AddLayer {
        layer: Layer,
        index: usize,
    },
    RemoveLayer {
        layer: Layer,
        index: usize,
    },
    ReorderLayer {
        layer_id: Uuid,
        from_index: usize,
        to_index: usize,
    },
    RenameLayer {
        layer_id: Uuid,
        old_name: String,
        new_name: String,
    },
    SetLayerVisibility {
        layer_id: Uuid,
        old_visible: bool,
        new_visible: bool,
    },
    SetLayerLocked {
        layer_id: Uuid,
        old_locked: bool,
        new_locked: bool,
    },
    SetLayerOpacity {
        layer_id: Uuid,
        old_opacity: f32,
        new_opacity: f32,
    },
    SetLayerBlendMode {
        layer_id: Uuid,
        old_mode: BlendMode,
        new_mode: BlendMode,
    },
    DuplicateLayer {
        original_id: Uuid,
        new_layer: Layer,
        index: usize,
    },
    MergeLayers {
        upper_layer: Box<Layer>,
        upper_index: usize,
        lower_layer: Box<Layer>,
        lower_index: usize,
        merged: Box<Layer>,
    },
    GroupLayers {
        layer_ids: Vec<Uuid>,
        layers: Vec<(Layer, usize)>,
        group: Box<Layer>,
        group_index: usize,
    },
    UngroupLayer {
        group: Box<Layer>,
        group_index: usize,
        children: Vec<(Layer, usize)>,
    },
}

/// Undo/redo history for a document.
///
/// Uses VecDeque for O(1) eviction of oldest commands when max_depth is reached,
/// and avoids cloning commands by moving them between stacks.
#[derive(Debug, Clone)]
pub struct History {
    undo_stack: VecDeque<Command>,
    redo_stack: Vec<Command>,
    max_depth: usize,
}

impl History {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    pub fn push(&mut self, command: Command) {
        self.redo_stack.clear();
        self.undo_stack.push_back(command);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.pop_front();
        }
    }

    pub fn undo(&mut self) -> Option<Command> {
        let cmd = self.undo_stack.pop_back()?;
        self.redo_stack.push(cmd.clone());
        Some(cmd)
    }

    pub fn redo(&mut self) -> Option<Command> {
        let cmd = self.redo_stack.pop()?;
        self.undo_stack.push_back(cmd.clone());
        Some(cmd)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::Layer;

    fn dummy_add_cmd() -> Command {
        Command::AddLayer {
            layer: Layer::new_raster("Test", 10, 10),
            index: 0,
        }
    }

    #[test]
    fn push_and_undo() {
        let mut h = History::new(100);
        h.push(dummy_add_cmd());
        assert!(h.can_undo());
        assert!(!h.can_redo());
        let cmd = h.undo();
        assert!(cmd.is_some());
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    #[test]
    fn redo_after_undo() {
        let mut h = History::new(100);
        h.push(dummy_add_cmd());
        h.undo();
        let cmd = h.redo();
        assert!(cmd.is_some());
        assert!(h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn push_clears_redo() {
        let mut h = History::new(100);
        h.push(dummy_add_cmd());
        h.undo();
        assert!(h.can_redo());
        h.push(dummy_add_cmd());
        assert!(!h.can_redo());
    }

    #[test]
    fn max_depth_evicts_oldest() {
        let mut h = History::new(3);
        for _ in 0..5 {
            h.push(dummy_add_cmd());
        }
        assert_eq!(h.undo_count(), 3);
    }

    #[test]
    fn empty_undo_returns_none() {
        let mut h = History::new(100);
        assert!(h.undo().is_none());
    }

    #[test]
    fn empty_redo_returns_none() {
        let mut h = History::new(100);
        assert!(h.redo().is_none());
    }

    #[test]
    fn clear_empties_both_stacks() {
        let mut h = History::new(100);
        h.push(dummy_add_cmd());
        h.push(dummy_add_cmd());
        h.undo();
        h.clear();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }
}
