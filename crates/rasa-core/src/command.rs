use std::collections::VecDeque;
use std::convert::Infallible;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::color::BlendMode;
use crate::layer::{Adjustment, Layer, LayerKind};
use crate::pixel::PixelBuffer;

/// Re-export muharrir's command primitives for external consumers who want
/// the full execute-pattern workflow.
pub use muharrir::command::{Command as CommandTrait, CommandHistory, CompoundCommand};

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
    SetAdjustment {
        layer_id: Uuid,
        old_adjustment: Adjustment,
        new_adjustment: Adjustment,
    },
}

// ── muharrir::command::Command trait implementation ─────────────
//
// This allows our Command enum to participate in muharrir's command
// infrastructure (CompoundCommand, CommandHistory) when desired.

impl muharrir::command::Command for Command {
    type Target = crate::document::Document;
    type Error = Infallible;

    fn apply(&mut self, doc: &mut Self::Target) -> Result<(), Self::Error> {
        apply_forward(self, doc);
        Ok(())
    }

    fn reverse(&mut self, doc: &mut Self::Target) -> Result<(), Self::Error> {
        apply_inverse(self, doc);
        Ok(())
    }

    fn description(&self) -> &str {
        match self {
            Command::AddLayer { .. } => "Add Layer",
            Command::RemoveLayer { .. } => "Remove Layer",
            Command::ReorderLayer { .. } => "Reorder Layer",
            Command::RenameLayer { .. } => "Rename Layer",
            Command::SetLayerVisibility { .. } => "Toggle Visibility",
            Command::SetLayerLocked { .. } => "Toggle Lock",
            Command::SetLayerOpacity { .. } => "Set Opacity",
            Command::SetLayerBlendMode { .. } => "Set Blend Mode",
            Command::DuplicateLayer { .. } => "Duplicate Layer",
            Command::MergeLayers { .. } => "Merge Layers",
            Command::GroupLayers { .. } => "Group Layers",
            Command::UngroupLayer { .. } => "Ungroup Layer",
            Command::SetAdjustment { .. } => "Set Adjustment",
        }
    }
}

// ── Apply / Reverse logic ──────────────────────────────────────

pub(crate) fn apply_forward(cmd: &Command, doc: &mut crate::document::Document) {
    match cmd {
        Command::AddLayer { layer, index } => {
            if let LayerKind::Raster { width, height } = layer.kind {
                doc.pixel_data
                    .push((layer.id, PixelBuffer::new(width, height)));
            }
            doc.layers.insert(*index, layer.clone());
        }
        Command::RemoveLayer { layer, index } => {
            doc.layers.remove(*index);
            doc.pixel_data.retain(|(id, _)| *id != layer.id);
        }
        Command::ReorderLayer {
            from_index,
            to_index,
            ..
        } => {
            let layer = doc.layers.remove(*from_index);
            doc.layers.insert(*to_index, layer);
        }
        Command::RenameLayer {
            layer_id, new_name, ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.name = new_name.clone();
            }
        }
        Command::SetLayerVisibility {
            layer_id,
            new_visible,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.visible = *new_visible;
            }
        }
        Command::SetLayerLocked {
            layer_id,
            new_locked,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.locked = *new_locked;
            }
        }
        Command::SetLayerOpacity {
            layer_id,
            new_opacity,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.opacity = *new_opacity;
            }
        }
        Command::SetLayerBlendMode {
            layer_id, new_mode, ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.blend_mode = *new_mode;
            }
        }
        Command::DuplicateLayer {
            original_id,
            new_layer,
            index,
        } => {
            if let Some(src_buf) = doc.get_pixels(*original_id) {
                doc.pixel_data.push((new_layer.id, src_buf.clone()));
            }
            doc.layers.insert(*index, new_layer.clone());
        }
        Command::MergeLayers {
            upper_layer,
            upper_index: _,
            merged,
            lower_index,
            ..
        } => {
            doc.layers[*lower_index] = *merged.clone();
            if let Some(idx) = doc.layers.iter().position(|l| l.id == upper_layer.id) {
                doc.layers.remove(idx);
                doc.pixel_data.retain(|(id, _)| *id != upper_layer.id);
            }
        }
        Command::GroupLayers {
            group,
            group_index,
            layers,
            ..
        } => {
            let mut indices: Vec<usize> = layers.iter().map(|(_, idx)| *idx).collect();
            indices.sort();
            for &idx in indices.iter().rev() {
                doc.layers.remove(idx);
            }
            doc.layers.insert(*group_index, *group.clone());
        }
        Command::UngroupLayer {
            group_index,
            children,
            ..
        } => {
            doc.layers.remove(*group_index);
            let mut sorted = children.clone();
            sorted.sort_by_key(|(_, idx)| *idx);
            for (layer, idx) in sorted {
                doc.layers.insert(idx, layer);
            }
        }
        Command::SetAdjustment {
            layer_id,
            new_adjustment,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.kind = LayerKind::Adjustment(new_adjustment.clone());
            }
        }
    }
}

pub(crate) fn apply_inverse(cmd: &Command, doc: &mut crate::document::Document) {
    match cmd {
        Command::AddLayer { layer, index } => {
            doc.layers.remove(*index);
            doc.pixel_data.retain(|(id, _)| *id != layer.id);
        }
        Command::RemoveLayer { layer, index } => {
            if let LayerKind::Raster { width, height } = layer.kind {
                doc.pixel_data
                    .push((layer.id, PixelBuffer::new(width, height)));
            }
            doc.layers.insert(*index, layer.clone());
        }
        Command::ReorderLayer {
            from_index,
            to_index,
            ..
        } => {
            let layer = doc.layers.remove(*to_index);
            doc.layers.insert(*from_index, layer);
        }
        Command::RenameLayer {
            layer_id, old_name, ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.name = old_name.clone();
            }
        }
        Command::SetLayerVisibility {
            layer_id,
            old_visible,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.visible = *old_visible;
            }
        }
        Command::SetLayerLocked {
            layer_id,
            old_locked,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.locked = *old_locked;
            }
        }
        Command::SetLayerOpacity {
            layer_id,
            old_opacity,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.opacity = *old_opacity;
            }
        }
        Command::SetLayerBlendMode {
            layer_id, old_mode, ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.blend_mode = *old_mode;
            }
        }
        Command::DuplicateLayer {
            new_layer, index, ..
        } => {
            doc.layers.remove(*index);
            doc.pixel_data.retain(|(id, _)| *id != new_layer.id);
        }
        Command::MergeLayers {
            upper_layer,
            upper_index,
            lower_layer,
            lower_index,
            ..
        } => {
            doc.layers[*lower_index] = *lower_layer.clone();
            if let LayerKind::Raster { width, height } = lower_layer.kind {
                doc.pixel_data.retain(|(id, _)| *id != lower_layer.id);
                doc.pixel_data
                    .push((lower_layer.id, PixelBuffer::new(width, height)));
            }
            if let LayerKind::Raster { width, height } = upper_layer.kind {
                doc.pixel_data
                    .push((upper_layer.id, PixelBuffer::new(width, height)));
            }
            doc.layers.insert(*upper_index, *upper_layer.clone());
        }
        Command::GroupLayers {
            layers,
            group_index,
            ..
        } => {
            doc.layers.remove(*group_index);
            let mut sorted = layers.clone();
            sorted.sort_by_key(|(_, idx)| *idx);
            for (layer, idx) in sorted {
                doc.layers.insert(idx, layer);
            }
        }
        Command::UngroupLayer {
            group,
            group_index,
            children,
            ..
        } => {
            for _ in 0..children.len() {
                doc.layers.remove(*group_index);
            }
            doc.layers.insert(*group_index, *group.clone());
        }
        Command::SetAdjustment {
            layer_id,
            old_adjustment,
            ..
        } => {
            if let Some(l) = doc.find_layer_mut(*layer_id) {
                l.kind = LayerKind::Adjustment(old_adjustment.clone());
            }
        }
    }
}

// ── History (record-then-undo stack) ───────────────────────────
//
// We keep our own history stack because our Document methods apply
// changes inline and then record the command. muharrir's CommandHistory
// uses the execute-pattern (apply via execute) which doesn't match
// our architecture. The Command trait is still implemented above for
// interop and for undo/redo dispatch.

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

    pub fn undo(&mut self) -> Option<&Command> {
        let cmd = self.undo_stack.pop_back()?;
        self.redo_stack.push(cmd);
        self.redo_stack.last()
    }

    pub fn redo(&mut self) -> Option<&Command> {
        let cmd = self.redo_stack.pop()?;
        self.undo_stack.push_back(cmd);
        self.undo_stack.back()
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

    #[test]
    fn command_trait_description() {
        use muharrir::command::Command as _;
        let cmd = dummy_add_cmd();
        assert_eq!(cmd.description(), "Add Layer");
    }

    #[test]
    fn command_trait_apply_and_reverse() {
        use muharrir::command::Command as _;
        let mut doc = crate::document::Document::new("T", 10, 10);
        let layer = Layer::new_raster("Added", 10, 10);
        let mut cmd = Command::AddLayer {
            layer: layer.clone(),
            index: 1,
        };
        cmd.apply(&mut doc).unwrap();
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers[1].name, "Added");
        cmd.reverse(&mut doc).unwrap();
        assert_eq!(doc.layers.len(), 1);
    }
}
