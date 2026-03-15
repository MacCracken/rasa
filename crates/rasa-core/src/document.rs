use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::color::BlendMode;
use crate::command::{Command, History};
use crate::error::RasaError;
use crate::geometry::Size;
use crate::layer::{Adjustment, Layer, LayerKind};
use crate::pixel::PixelBuffer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub name: String,
    pub size: Size,
    pub dpi: f32,
    pub layers: Vec<Layer>,
    pub active_layer: Option<Uuid>,
    #[serde(skip)]
    pub pixel_data: Vec<(Uuid, PixelBuffer)>,
    #[serde(skip)]
    history: Option<History>,
}

impl Document {
    /// Maximum dimension (width or height) for a document.
    pub const MAX_DIMENSION: u32 = 65536;

    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        let width = width.clamp(1, Self::MAX_DIMENSION);
        let height = height.clamp(1, Self::MAX_DIMENSION);
        let bg = Layer::new_raster("Background", width, height);
        let bg_id = bg.id;
        let pixel_data = vec![(
            bg_id,
            PixelBuffer::filled(width, height, crate::color::Color::WHITE),
        )];
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            size: Size { width, height },
            dpi: 72.0,
            layers: vec![bg],
            active_layer: Some(bg_id),
            pixel_data,
            history: Some(History::new(200)),
        }
    }

    // ── Layer lookup ───────────────────────────────────

    pub fn find_layer(&self, id: Uuid) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn find_layer_mut(&mut self, id: Uuid) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn layer_index(&self, id: Uuid) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    pub fn get_pixels(&self, layer_id: Uuid) -> Option<&PixelBuffer> {
        self.pixel_data
            .iter()
            .find(|(id, _)| *id == layer_id)
            .map(|(_, buf)| buf)
    }

    pub fn get_pixels_mut(&mut self, layer_id: Uuid) -> Option<&mut PixelBuffer> {
        self.pixel_data
            .iter_mut()
            .find(|(id, _)| *id == layer_id)
            .map(|(_, buf)| buf)
    }

    // ── Layer operations ───────────────────────────────

    pub fn add_layer(&mut self, layer: Layer) {
        let index = self.layers.len();
        self.add_layer_at(layer, index);
    }

    pub fn add_layer_at(&mut self, layer: Layer, index: usize) {
        let index = index.min(self.layers.len());
        if let LayerKind::Raster { width, height } = layer.kind {
            self.pixel_data
                .push((layer.id, PixelBuffer::new(width, height)));
        }
        let cmd = Command::AddLayer {
            layer: layer.clone(),
            index,
        };
        self.layers.insert(index, layer);
        self.active_layer = Some(self.layers[index].id);
        self.push_command(cmd);
    }

    pub fn remove_layer(&mut self, id: Uuid) -> Result<Layer, RasaError> {
        if self.layers.len() <= 1 {
            return Err(RasaError::CannotRemoveLastLayer);
        }
        let index = self.layer_index(id).ok_or(RasaError::LayerNotFound(id))?;
        let layer = self.layers.remove(index);
        self.pixel_data.retain(|(lid, _)| *lid != id);
        let cmd = Command::RemoveLayer {
            layer: layer.clone(),
            index,
        };
        if self.active_layer == Some(id) {
            self.active_layer = self.layers.last().map(|l| l.id);
        }
        self.push_command(cmd);
        Ok(layer)
    }

    pub fn reorder_layer(&mut self, id: Uuid, new_index: usize) -> Result<(), RasaError> {
        let from = self.layer_index(id).ok_or(RasaError::LayerNotFound(id))?;
        let to = new_index.min(self.layers.len().saturating_sub(1));
        if from == to {
            return Ok(());
        }
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
        self.push_command(Command::ReorderLayer {
            layer_id: id,
            from_index: from,
            to_index: to,
        });
        Ok(())
    }

    pub fn duplicate_layer(&mut self, id: Uuid) -> Result<Uuid, RasaError> {
        let index = self.layer_index(id).ok_or(RasaError::LayerNotFound(id))?;
        let original = &self.layers[index];
        let mut new_layer = original.clone();
        new_layer.id = Uuid::new_v4();
        new_layer.name = format!("{} copy", original.name);
        let new_id = new_layer.id;
        let insert_at = index + 1;

        // Duplicate pixel data if raster
        if let Some(src_buf) = self.get_pixels(id) {
            self.pixel_data.push((new_id, src_buf.clone()));
        }

        let cmd = Command::DuplicateLayer {
            original_id: id,
            new_layer: new_layer.clone(),
            index: insert_at,
        };
        self.layers.insert(insert_at, new_layer);
        self.active_layer = Some(new_id);
        self.push_command(cmd);
        Ok(new_id)
    }

    pub fn set_layer_visibility(&mut self, id: Uuid, visible: bool) -> Result<(), RasaError> {
        let layer = self
            .find_layer_mut(id)
            .ok_or(RasaError::LayerNotFound(id))?;
        let old = layer.visible;
        layer.visible = visible;
        self.push_command(Command::SetLayerVisibility {
            layer_id: id,
            old_visible: old,
            new_visible: visible,
        });
        Ok(())
    }

    pub fn set_layer_locked(&mut self, id: Uuid, locked: bool) -> Result<(), RasaError> {
        let layer = self
            .find_layer_mut(id)
            .ok_or(RasaError::LayerNotFound(id))?;
        let old = layer.locked;
        layer.locked = locked;
        self.push_command(Command::SetLayerLocked {
            layer_id: id,
            old_locked: old,
            new_locked: locked,
        });
        Ok(())
    }

    pub fn set_layer_opacity(&mut self, id: Uuid, opacity: f32) -> Result<(), RasaError> {
        let layer = self
            .find_layer_mut(id)
            .ok_or(RasaError::LayerNotFound(id))?;
        let old = layer.opacity;
        layer.opacity = opacity.clamp(0.0, 1.0);
        self.push_command(Command::SetLayerOpacity {
            layer_id: id,
            old_opacity: old,
            new_opacity: opacity.clamp(0.0, 1.0),
        });
        Ok(())
    }

    pub fn set_layer_blend_mode(&mut self, id: Uuid, mode: BlendMode) -> Result<(), RasaError> {
        let layer = self
            .find_layer_mut(id)
            .ok_or(RasaError::LayerNotFound(id))?;
        let old = layer.blend_mode;
        layer.blend_mode = mode;
        self.push_command(Command::SetLayerBlendMode {
            layer_id: id,
            old_mode: old,
            new_mode: mode,
        });
        Ok(())
    }

    pub fn rename_layer(&mut self, id: Uuid, name: impl Into<String>) -> Result<(), RasaError> {
        let name = name.into();
        let layer = self
            .find_layer_mut(id)
            .ok_or(RasaError::LayerNotFound(id))?;
        let old = layer.name.clone();
        layer.name = name.clone();
        self.push_command(Command::RenameLayer {
            layer_id: id,
            old_name: old,
            new_name: name,
        });
        Ok(())
    }

    /// Add a non-destructive adjustment layer above the current stack.
    pub fn add_adjustment_layer(
        &mut self,
        name: impl Into<String>,
        adjustment: Adjustment,
    ) -> Uuid {
        let layer = Layer::new_adjustment(name, adjustment, (self.size.width, self.size.height));
        let id = layer.id;
        let index = self.layers.len();
        let cmd = Command::AddLayer {
            layer: layer.clone(),
            index,
        };
        self.layers.insert(index, layer);
        self.active_layer = Some(id);
        self.push_command(cmd);
        id
    }

    /// Update the parameters of an existing adjustment layer (non-destructive).
    pub fn set_adjustment(&mut self, id: Uuid, adjustment: Adjustment) -> Result<(), RasaError> {
        let layer = self
            .find_layer_mut(id)
            .ok_or(RasaError::LayerNotFound(id))?;
        let old = match &layer.kind {
            LayerKind::Adjustment(adj) => adj.clone(),
            _ => return Err(RasaError::NotAnAdjustmentLayer(id)),
        };
        layer.kind = LayerKind::Adjustment(adjustment.clone());
        self.push_command(Command::SetAdjustment {
            layer_id: id,
            old_adjustment: old,
            new_adjustment: adjustment,
        });
        Ok(())
    }

    pub fn flatten_visible(&self) -> Vec<(&Layer, Option<&PixelBuffer>)> {
        self.layers
            .iter()
            .filter(|l| l.visible)
            .map(|l| (l, self.get_pixels(l.id)))
            .collect()
    }

    // ── Merge / Group / Flatten ─────────────────────────

    /// Merge a layer down into the layer below it. Both must be raster layers.
    /// The upper layer is removed and its pixels are composited onto the lower layer.
    pub fn merge_down(&mut self, upper_id: Uuid) -> Result<(), RasaError> {
        let upper_idx = self
            .layer_index(upper_id)
            .ok_or(RasaError::LayerNotFound(upper_id))?;
        if upper_idx == 0 {
            return Err(RasaError::Other("cannot merge down: no layer below".into()));
        }
        let lower_idx = upper_idx - 1;

        let upper_layer = self.layers[upper_idx].clone();
        let lower_layer = self.layers[lower_idx].clone();

        // Composite upper onto lower
        let (lower_w, lower_h) = match lower_layer.kind {
            LayerKind::Raster { width, height } => (width, height),
            _ => {
                return Err(RasaError::Other(
                    "cannot merge: lower layer is not raster".into(),
                ));
            }
        };

        let upper_buf = self
            .get_pixels(upper_layer.id)
            .ok_or_else(|| RasaError::Other("upper layer has no pixel data".into()))?
            .clone();

        let lower_buf = self
            .get_pixels_mut(lower_layer.id)
            .ok_or_else(|| RasaError::Other("lower layer has no pixel data".into()))?;

        let w = lower_w.min(upper_buf.width);
        let h = lower_h.min(upper_buf.height);
        for y in 0..h {
            for x in 0..w {
                let base = lower_buf.get(x, y).unwrap();
                let top = upper_buf.get(x, y).unwrap();
                let result =
                    crate::blend::blend(base, top, upper_layer.blend_mode, upper_layer.opacity);
                lower_buf.set(x, y, result);
            }
        }

        // Create the merged layer snapshot for undo
        let merged = self.layers[lower_idx].clone();

        // Remove upper layer
        self.layers.remove(upper_idx);
        self.pixel_data.retain(|(id, _)| *id != upper_layer.id);

        self.active_layer = Some(lower_layer.id);
        self.push_command(Command::MergeLayers {
            upper_layer: Box::new(upper_layer),
            upper_index: upper_idx,
            lower_layer: Box::new(lower_layer),
            lower_index: lower_idx,
            merged: Box::new(merged),
        });
        Ok(())
    }

    /// Group the specified layers into a new group layer.
    /// Layers must be contiguous in the stack.
    pub fn group_layers(&mut self, layer_ids: &[Uuid]) -> Result<Uuid, RasaError> {
        if layer_ids.is_empty() {
            return Err(RasaError::Other("no layers to group".into()));
        }

        // Collect indices and verify they exist
        let mut indices: Vec<usize> = layer_ids
            .iter()
            .map(|id| self.layer_index(*id).ok_or(RasaError::LayerNotFound(*id)))
            .collect::<Result<Vec<_>, _>>()?;
        indices.sort();

        // Verify contiguous
        for i in 1..indices.len() {
            if indices[i] != indices[i - 1] + 1 {
                return Err(RasaError::Other(
                    "layers must be contiguous to group".into(),
                ));
            }
        }

        let group_index = indices[0];

        // Collect layers with their indices (for undo)
        let layers_with_indices: Vec<(Layer, usize)> = indices
            .iter()
            .rev()
            .map(|&idx| (self.layers[idx].clone(), idx))
            .collect();

        // Extract the children (remove from highest index first)
        let mut children = Vec::new();
        for &idx in indices.iter().rev() {
            children.push(self.layers.remove(idx));
        }
        children.reverse(); // restore original order

        // Create group layer
        let group = Layer {
            id: Uuid::new_v4(),
            name: "Group".into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::default(),
            bounds: crate::geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: self.size.width as f64,
                height: self.size.height as f64,
            },
            kind: LayerKind::Group { children },
        };
        let group_id = group.id;

        let cmd = Command::GroupLayers {
            layer_ids: layer_ids.to_vec(),
            layers: layers_with_indices,
            group: Box::new(group.clone()),
            group_index,
        };
        self.layers.insert(group_index, group);
        self.active_layer = Some(group_id);
        self.push_command(cmd);
        Ok(group_id)
    }

    /// Ungroup a group layer, replacing it with its children in the stack.
    pub fn ungroup_layer(&mut self, group_id: Uuid) -> Result<Vec<Uuid>, RasaError> {
        let group_idx = self
            .layer_index(group_id)
            .ok_or(RasaError::LayerNotFound(group_id))?;

        let group_layer = self.layers[group_idx].clone();
        let children = match &group_layer.kind {
            LayerKind::Group { children } => children.clone(),
            _ => {
                return Err(RasaError::Other("layer is not a group".into()));
            }
        };

        // Remove the group
        self.layers.remove(group_idx);

        // Collect children info for undo
        let children_with_indices: Vec<(Layer, usize)> = children
            .iter()
            .enumerate()
            .map(|(i, l)| (l.clone(), group_idx + i))
            .collect();

        // Insert children at the group's position
        let child_ids: Vec<Uuid> = children.iter().map(|l| l.id).collect();
        for (i, child) in children.into_iter().enumerate() {
            self.layers.insert(group_idx + i, child);
        }

        self.active_layer = child_ids.first().copied();
        self.push_command(Command::UngroupLayer {
            group: Box::new(group_layer),
            group_index: group_idx,
            children: children_with_indices,
        });
        Ok(child_ids)
    }

    // ── Undo / Redo ────────────────────────────────────

    fn history_mut(&mut self) -> &mut History {
        self.history.get_or_insert_with(|| History::new(200))
    }

    fn push_command(&mut self, cmd: Command) {
        self.history_mut().push(cmd);
    }

    pub fn can_undo(&self) -> bool {
        self.history.as_ref().is_some_and(|h| h.can_undo())
    }

    pub fn can_redo(&self) -> bool {
        self.history.as_ref().is_some_and(|h| h.can_redo())
    }

    pub fn undo(&mut self) -> Result<(), RasaError> {
        let cmd = self
            .history_mut()
            .undo()
            .ok_or(RasaError::NothingToUndo)?
            .clone();
        self.apply_inverse(&cmd);
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), RasaError> {
        let cmd = self
            .history_mut()
            .redo()
            .ok_or(RasaError::NothingToRedo)?
            .clone();
        self.apply_forward(&cmd);
        Ok(())
    }

    fn apply_inverse(&mut self, cmd: &Command) {
        match cmd {
            Command::AddLayer { layer, index } => {
                self.layers.remove(*index);
                self.pixel_data.retain(|(id, _)| *id != layer.id);
            }
            Command::RemoveLayer { layer, index } => {
                if let LayerKind::Raster { width, height } = layer.kind {
                    self.pixel_data
                        .push((layer.id, PixelBuffer::new(width, height)));
                }
                self.layers.insert(*index, layer.clone());
            }
            Command::ReorderLayer {
                from_index,
                to_index,
                ..
            } => {
                let layer = self.layers.remove(*to_index);
                self.layers.insert(*from_index, layer);
            }
            Command::RenameLayer {
                layer_id, old_name, ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.name = old_name.clone();
                }
            }
            Command::SetLayerVisibility {
                layer_id,
                old_visible,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.visible = *old_visible;
                }
            }
            Command::SetLayerLocked {
                layer_id,
                old_locked,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.locked = *old_locked;
                }
            }
            Command::SetLayerOpacity {
                layer_id,
                old_opacity,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.opacity = *old_opacity;
                }
            }
            Command::SetLayerBlendMode {
                layer_id, old_mode, ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.blend_mode = *old_mode;
                }
            }
            Command::DuplicateLayer {
                new_layer, index, ..
            } => {
                self.layers.remove(*index);
                self.pixel_data.retain(|(id, _)| *id != new_layer.id);
            }
            Command::MergeLayers {
                upper_layer,
                upper_index,
                lower_layer,
                lower_index,
                ..
            } => {
                // Undo merge: restore the original lower layer and re-insert upper
                self.layers[*lower_index] = *lower_layer.clone();
                // Restore lower layer's pixel data
                if let LayerKind::Raster { width, height } = lower_layer.kind {
                    // Replace pixel data with a fresh buffer (original pixels lost in merge)
                    self.pixel_data.retain(|(id, _)| *id != lower_layer.id);
                    self.pixel_data
                        .push((lower_layer.id, PixelBuffer::new(width, height)));
                }
                // Re-insert upper layer
                if let LayerKind::Raster { width, height } = upper_layer.kind {
                    self.pixel_data
                        .push((upper_layer.id, PixelBuffer::new(width, height)));
                }
                self.layers.insert(*upper_index, *upper_layer.clone());
            }
            Command::GroupLayers {
                layers,
                group_index,
                ..
            } => {
                // Undo group: remove the group, re-insert original layers
                self.layers.remove(*group_index);
                let mut sorted = layers.clone();
                sorted.sort_by_key(|(_, idx)| *idx);
                for (layer, idx) in sorted {
                    self.layers.insert(idx, layer);
                }
            }
            Command::UngroupLayer {
                group,
                group_index,
                children,
                ..
            } => {
                // Undo ungroup: remove children, re-insert group
                for _ in 0..children.len() {
                    self.layers.remove(*group_index);
                }
                self.layers.insert(*group_index, *group.clone());
            }
            Command::SetAdjustment {
                layer_id,
                old_adjustment,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.kind = LayerKind::Adjustment(old_adjustment.clone());
                }
            }
        }
    }

    fn apply_forward(&mut self, cmd: &Command) {
        match cmd {
            Command::AddLayer { layer, index } => {
                if let LayerKind::Raster { width, height } = layer.kind {
                    self.pixel_data
                        .push((layer.id, PixelBuffer::new(width, height)));
                }
                self.layers.insert(*index, layer.clone());
            }
            Command::RemoveLayer { layer, index } => {
                self.layers.remove(*index);
                self.pixel_data.retain(|(id, _)| *id != layer.id);
            }
            Command::ReorderLayer {
                from_index,
                to_index,
                ..
            } => {
                let layer = self.layers.remove(*from_index);
                self.layers.insert(*to_index, layer);
            }
            Command::RenameLayer {
                layer_id, new_name, ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.name = new_name.clone();
                }
            }
            Command::SetLayerVisibility {
                layer_id,
                new_visible,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.visible = *new_visible;
                }
            }
            Command::SetLayerLocked {
                layer_id,
                new_locked,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.locked = *new_locked;
                }
            }
            Command::SetLayerOpacity {
                layer_id,
                new_opacity,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.opacity = *new_opacity;
                }
            }
            Command::SetLayerBlendMode {
                layer_id, new_mode, ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.blend_mode = *new_mode;
                }
            }
            Command::DuplicateLayer {
                original_id,
                new_layer,
                index,
            } => {
                if let Some(src_buf) = self.get_pixels(*original_id) {
                    self.pixel_data.push((new_layer.id, src_buf.clone()));
                }
                self.layers.insert(*index, new_layer.clone());
            }
            Command::MergeLayers {
                upper_layer,
                upper_index: _,
                merged,
                lower_index,
                ..
            } => {
                // Redo merge: replace lower with merged, remove upper
                self.layers[*lower_index] = *merged.clone();
                // Find and remove the upper layer
                if let Some(idx) = self.layers.iter().position(|l| l.id == upper_layer.id) {
                    self.layers.remove(idx);
                    self.pixel_data.retain(|(id, _)| *id != upper_layer.id);
                }
            }
            Command::GroupLayers {
                group,
                group_index,
                layers,
                ..
            } => {
                // Redo group: remove original layers (highest index first), insert group
                let mut indices: Vec<usize> = layers.iter().map(|(_, idx)| *idx).collect();
                indices.sort();
                for &idx in indices.iter().rev() {
                    self.layers.remove(idx);
                }
                self.layers.insert(*group_index, *group.clone());
            }
            Command::UngroupLayer {
                group_index,
                children,
                ..
            } => {
                // Redo ungroup: remove group, insert children
                self.layers.remove(*group_index);
                let mut sorted = children.clone();
                sorted.sort_by_key(|(_, idx)| *idx);
                for (layer, idx) in sorted {
                    self.layers.insert(idx, layer);
                }
            }
            Command::SetAdjustment {
                layer_id,
                new_adjustment,
                ..
            } => {
                if let Some(l) = self.find_layer_mut(*layer_id) {
                    l.kind = LayerKind::Adjustment(new_adjustment.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_has_background() {
        let doc = Document::new("Test", 100, 100);
        assert_eq!(doc.layers.len(), 1);
        assert_eq!(doc.layers[0].name, "Background");
        assert!(doc.active_layer.is_some());
    }

    #[test]
    fn add_layer() {
        let mut doc = Document::new("Test", 100, 100);
        doc.add_layer(Layer::new_raster("Layer 1", 100, 100));
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers[1].name, "Layer 1");
    }

    #[test]
    fn add_layer_at_index() {
        let mut doc = Document::new("Test", 100, 100);
        doc.add_layer(Layer::new_raster("Top", 100, 100));
        doc.add_layer_at(Layer::new_raster("Middle", 100, 100), 1);
        assert_eq!(doc.layers[0].name, "Background");
        assert_eq!(doc.layers[1].name, "Middle");
        assert_eq!(doc.layers[2].name, "Top");
    }

    #[test]
    fn remove_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let l = Layer::new_raster("Layer 1", 100, 100);
        let id = l.id;
        doc.add_layer(l);
        assert_eq!(doc.layers.len(), 2);
        doc.remove_layer(id).unwrap();
        assert_eq!(doc.layers.len(), 1);
    }

    #[test]
    fn remove_nonexistent_layer_errors() {
        let mut doc = Document::new("Test", 100, 100);
        let result = doc.remove_layer(Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let bg_id = doc.layers[0].id;
        let new_id = doc.duplicate_layer(bg_id).unwrap();
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers[1].name, "Background copy");
        assert_ne!(new_id, bg_id);
        assert!(doc.get_pixels(new_id).is_some());
    }

    #[test]
    fn reorder_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let l = Layer::new_raster("Top", 100, 100);
        let top_id = l.id;
        doc.add_layer(l);
        assert_eq!(doc.layers[1].name, "Top");
        doc.reorder_layer(top_id, 0).unwrap();
        assert_eq!(doc.layers[0].name, "Top");
        assert_eq!(doc.layers[1].name, "Background");
    }

    #[test]
    fn set_opacity() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        doc.set_layer_opacity(id, 0.5).unwrap();
        assert_eq!(doc.layers[0].opacity, 0.5);
    }

    #[test]
    fn set_opacity_clamps() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        doc.set_layer_opacity(id, 1.5).unwrap();
        assert_eq!(doc.layers[0].opacity, 1.0);
        doc.set_layer_opacity(id, -0.5).unwrap();
        assert_eq!(doc.layers[0].opacity, 0.0);
    }

    #[test]
    fn set_blend_mode() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        doc.set_layer_blend_mode(id, BlendMode::Multiply).unwrap();
        assert_eq!(doc.layers[0].blend_mode, BlendMode::Multiply);
    }

    #[test]
    fn rename_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        doc.rename_layer(id, "New Name").unwrap();
        assert_eq!(doc.layers[0].name, "New Name");
    }

    #[test]
    fn visibility_toggle() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        assert!(doc.layers[0].visible);
        doc.set_layer_visibility(id, false).unwrap();
        assert!(!doc.layers[0].visible);
    }

    #[test]
    fn flatten_visible_skips_hidden() {
        let mut doc = Document::new("Test", 100, 100);
        let bg_id = doc.layers[0].id;
        let l = Layer::new_raster("Hidden", 100, 100);
        let hid = l.id;
        doc.add_layer(l);
        doc.set_layer_visibility(hid, false).unwrap();
        let visible = doc.flatten_visible();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].0.id, bg_id);
    }

    #[test]
    fn undo_add_layer() {
        let mut doc = Document::new("Test", 100, 100);
        doc.add_layer(Layer::new_raster("Layer 1", 100, 100));
        assert_eq!(doc.layers.len(), 2);
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 1);
    }

    #[test]
    fn redo_add_layer() {
        let mut doc = Document::new("Test", 100, 100);
        doc.add_layer(Layer::new_raster("Layer 1", 100, 100));
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 1);
        doc.redo().unwrap();
        assert_eq!(doc.layers.len(), 2);
    }

    #[test]
    fn undo_remove_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let l = Layer::new_raster("Layer 1", 100, 100);
        let id = l.id;
        doc.add_layer(l);
        doc.remove_layer(id).unwrap();
        assert_eq!(doc.layers.len(), 1);
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 2);
    }

    #[test]
    fn undo_reorder() {
        let mut doc = Document::new("Test", 100, 100);
        let l = Layer::new_raster("Top", 100, 100);
        let top_id = l.id;
        doc.add_layer(l);
        doc.reorder_layer(top_id, 0).unwrap();
        assert_eq!(doc.layers[0].name, "Top");
        doc.undo().unwrap();
        assert_eq!(doc.layers[0].name, "Background");
        assert_eq!(doc.layers[1].name, "Top");
    }

    #[test]
    fn undo_rename() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        doc.rename_layer(id, "Renamed").unwrap();
        assert_eq!(doc.layers[0].name, "Renamed");
        doc.undo().unwrap();
        assert_eq!(doc.layers[0].name, "Background");
    }

    #[test]
    fn undo_opacity() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        doc.set_layer_opacity(id, 0.3).unwrap();
        doc.undo().unwrap();
        assert_eq!(doc.layers[0].opacity, 1.0);
    }

    #[test]
    fn undo_blend_mode() {
        let mut doc = Document::new("Test", 100, 100);
        let id = doc.layers[0].id;
        doc.set_layer_blend_mode(id, BlendMode::Screen).unwrap();
        doc.undo().unwrap();
        assert_eq!(doc.layers[0].blend_mode, BlendMode::Normal);
    }

    #[test]
    fn undo_duplicate() {
        let mut doc = Document::new("Test", 100, 100);
        let bg_id = doc.layers[0].id;
        doc.duplicate_layer(bg_id).unwrap();
        assert_eq!(doc.layers.len(), 2);
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 1);
    }

    #[test]
    fn pixel_data_for_new_layers() {
        let doc = Document::new("Test", 100, 100);
        let bg_id = doc.layers[0].id;
        let pixels = doc.get_pixels(bg_id).unwrap();
        assert_eq!(pixels.dimensions(), (100, 100));
    }

    #[test]
    fn nothing_to_undo_errors() {
        let mut doc = Document::new("Test", 100, 100);
        // undo the initial add (background) — history has no entries since
        // new() doesn't record the initial background as a command
        let result = doc.undo();
        assert!(result.is_err());
    }

    // ── Merge tests ─────────────────────────────────────

    #[test]
    fn merge_down_combines_layers() {
        let mut doc = Document::new("Test", 4, 4);
        let l = Layer::new_raster("Top", 4, 4);
        let top_id = l.id;
        doc.add_layer(l);
        // Fill top layer with red
        if let Some(buf) = doc.get_pixels_mut(top_id) {
            for y in 0..4 {
                for x in 0..4 {
                    buf.set(x, y, crate::color::Color::new(1.0, 0.0, 0.0, 1.0));
                }
            }
        }
        assert_eq!(doc.layers.len(), 2);
        doc.merge_down(top_id).unwrap();
        assert_eq!(doc.layers.len(), 1);
        assert_eq!(doc.layers[0].name, "Background");
    }

    #[test]
    fn merge_down_bottom_layer_errors() {
        let doc_bg_id = {
            let doc = Document::new("Test", 4, 4);
            doc.layers[0].id
        };
        let mut doc = Document::new("Test", 4, 4);
        let bg_id = doc.layers[0].id;
        let result = doc.merge_down(bg_id);
        assert!(result.is_err());
        let _ = doc_bg_id; // just to suppress warning
    }

    #[test]
    fn merge_down_nonexistent_errors() {
        let mut doc = Document::new("Test", 4, 4);
        let result = doc.merge_down(Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn undo_merge_down() {
        let mut doc = Document::new("Test", 4, 4);
        let l = Layer::new_raster("Top", 4, 4);
        let top_id = l.id;
        doc.add_layer(l);
        assert_eq!(doc.layers.len(), 2);
        doc.merge_down(top_id).unwrap();
        assert_eq!(doc.layers.len(), 1);
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers[1].name, "Top");
    }

    // ── Group tests ─────────────────────────────────────

    #[test]
    fn group_layers_creates_group() {
        let mut doc = Document::new("Test", 10, 10);
        let l1 = Layer::new_raster("Layer 1", 10, 10);
        let l2 = Layer::new_raster("Layer 2", 10, 10);
        let id1 = l1.id;
        let id2 = l2.id;
        doc.add_layer(l1);
        doc.add_layer(l2);
        assert_eq!(doc.layers.len(), 3);

        let group_id = doc.group_layers(&[id1, id2]).unwrap();
        assert_eq!(doc.layers.len(), 2); // Background + Group
        let group = doc.find_layer(group_id).unwrap();
        assert_eq!(group.name, "Group");
        if let LayerKind::Group { children } = &group.kind {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].name, "Layer 1");
            assert_eq!(children[1].name, "Layer 2");
        } else {
            panic!("expected Group");
        }
    }

    #[test]
    fn group_empty_errors() {
        let mut doc = Document::new("Test", 10, 10);
        let result = doc.group_layers(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn group_noncontiguous_errors() {
        let mut doc = Document::new("Test", 10, 10);
        let l1 = Layer::new_raster("L1", 10, 10);
        let l2 = Layer::new_raster("L2", 10, 10);
        let l3 = Layer::new_raster("L3", 10, 10);
        let id1 = l1.id;
        let id3 = l3.id;
        doc.add_layer(l1);
        doc.add_layer(l2);
        doc.add_layer(l3);
        // L1 is at index 1, L3 is at index 3 — not contiguous
        let result = doc.group_layers(&[id1, id3]);
        assert!(result.is_err());
    }

    #[test]
    fn undo_group_layers() {
        let mut doc = Document::new("Test", 10, 10);
        let l1 = Layer::new_raster("Layer 1", 10, 10);
        let l2 = Layer::new_raster("Layer 2", 10, 10);
        let id1 = l1.id;
        let id2 = l2.id;
        doc.add_layer(l1);
        doc.add_layer(l2);
        doc.group_layers(&[id1, id2]).unwrap();
        assert_eq!(doc.layers.len(), 2);
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 3);
        assert_eq!(doc.layers[1].name, "Layer 1");
        assert_eq!(doc.layers[2].name, "Layer 2");
    }

    // ── Ungroup tests ───────────────────────────────────

    #[test]
    fn ungroup_layer_restores_children() {
        let mut doc = Document::new("Test", 10, 10);
        let l1 = Layer::new_raster("Layer 1", 10, 10);
        let l2 = Layer::new_raster("Layer 2", 10, 10);
        let id1 = l1.id;
        let id2 = l2.id;
        doc.add_layer(l1);
        doc.add_layer(l2);
        let group_id = doc.group_layers(&[id1, id2]).unwrap();
        assert_eq!(doc.layers.len(), 2);

        let child_ids = doc.ungroup_layer(group_id).unwrap();
        assert_eq!(doc.layers.len(), 3);
        assert_eq!(child_ids.len(), 2);
        assert_eq!(doc.layers[1].name, "Layer 1");
        assert_eq!(doc.layers[2].name, "Layer 2");
    }

    #[test]
    fn ungroup_non_group_errors() {
        let mut doc = Document::new("Test", 10, 10);
        let bg_id = doc.layers[0].id;
        let result = doc.ungroup_layer(bg_id);
        assert!(result.is_err());
    }

    #[test]
    fn undo_ungroup() {
        let mut doc = Document::new("Test", 10, 10);
        let l1 = Layer::new_raster("Layer 1", 10, 10);
        let l2 = Layer::new_raster("Layer 2", 10, 10);
        let id1 = l1.id;
        let id2 = l2.id;
        doc.add_layer(l1);
        doc.add_layer(l2);
        let group_id = doc.group_layers(&[id1, id2]).unwrap();
        doc.ungroup_layer(group_id).unwrap();
        assert_eq!(doc.layers.len(), 3);
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 2);
        // The group should be back
        if let LayerKind::Group { children } = &doc.layers[1].kind {
            assert_eq!(children.len(), 2);
        } else {
            panic!("expected Group after undo");
        }
    }

    // ── Validation tests ────────────────────────────────

    #[test]
    fn new_document_clamps_zero_dimensions() {
        let doc = Document::new("Tiny", 0, 0);
        assert_eq!(doc.size.width, 1);
        assert_eq!(doc.size.height, 1);
    }

    #[test]
    fn new_document_clamps_huge_dimensions() {
        let doc = Document::new("Huge", 100000, 100000);
        assert_eq!(doc.size.width, Document::MAX_DIMENSION);
        assert_eq!(doc.size.height, Document::MAX_DIMENSION);
    }

    #[test]
    fn cannot_remove_last_layer() {
        let mut doc = Document::new("Test", 10, 10);
        let bg_id = doc.layers[0].id;
        let result = doc.remove_layer(bg_id);
        assert!(result.is_err());
        assert_eq!(doc.layers.len(), 1); // still there
    }

    #[test]
    fn remove_layer_works_with_multiple() {
        let mut doc = Document::new("Test", 10, 10);
        let l = Layer::new_raster("Extra", 10, 10);
        let lid = l.id;
        doc.add_layer(l);
        assert_eq!(doc.layers.len(), 2);
        doc.remove_layer(lid).unwrap();
        assert_eq!(doc.layers.len(), 1);
    }

    // ── Adjustment layer tests ──────────────────────────

    #[test]
    fn add_adjustment_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let adj_id = doc.add_adjustment_layer(
            "Brightness",
            crate::layer::Adjustment::BrightnessContrast {
                brightness: 0.2,
                contrast: 0.0,
            },
        );
        assert_eq!(doc.layers.len(), 2);
        let layer = doc.find_layer(adj_id).unwrap();
        assert_eq!(layer.name, "Brightness");
        assert!(matches!(layer.kind, LayerKind::Adjustment(_)));
        assert!(doc.get_pixels(adj_id).is_none()); // no pixel data
    }

    #[test]
    fn set_adjustment_updates_params() {
        let mut doc = Document::new("Test", 100, 100);
        let adj_id = doc.add_adjustment_layer(
            "BC",
            crate::layer::Adjustment::BrightnessContrast {
                brightness: 0.1,
                contrast: 0.0,
            },
        );
        doc.set_adjustment(
            adj_id,
            crate::layer::Adjustment::BrightnessContrast {
                brightness: 0.5,
                contrast: 0.3,
            },
        )
        .unwrap();
        let layer = doc.find_layer(adj_id).unwrap();
        if let LayerKind::Adjustment(crate::layer::Adjustment::BrightnessContrast {
            brightness,
            contrast,
        }) = &layer.kind
        {
            assert_eq!(*brightness, 0.5);
            assert_eq!(*contrast, 0.3);
        } else {
            panic!("expected BrightnessContrast");
        }
    }

    #[test]
    fn set_adjustment_on_raster_errors() {
        let mut doc = Document::new("Test", 10, 10);
        let bg_id = doc.layers[0].id;
        let result = doc.set_adjustment(
            bg_id,
            crate::layer::Adjustment::BrightnessContrast {
                brightness: 0.0,
                contrast: 0.0,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn undo_set_adjustment() {
        let mut doc = Document::new("Test", 100, 100);
        let adj_id = doc.add_adjustment_layer(
            "Levels",
            crate::layer::Adjustment::Levels {
                black: 0.0,
                white: 1.0,
                gamma: 1.0,
            },
        );
        doc.set_adjustment(
            adj_id,
            crate::layer::Adjustment::Levels {
                black: 0.1,
                white: 0.9,
                gamma: 2.0,
            },
        )
        .unwrap();
        doc.undo().unwrap();
        let layer = doc.find_layer(adj_id).unwrap();
        if let LayerKind::Adjustment(crate::layer::Adjustment::Levels { gamma, .. }) = &layer.kind {
            assert_eq!(*gamma, 1.0);
        } else {
            panic!("expected Levels after undo");
        }
    }

    #[test]
    fn redo_set_adjustment() {
        let mut doc = Document::new("Test", 100, 100);
        let adj_id = doc.add_adjustment_layer(
            "Levels",
            crate::layer::Adjustment::Levels {
                black: 0.0,
                white: 1.0,
                gamma: 1.0,
            },
        );
        doc.set_adjustment(
            adj_id,
            crate::layer::Adjustment::Levels {
                black: 0.1,
                white: 0.9,
                gamma: 2.0,
            },
        )
        .unwrap();
        doc.undo().unwrap();
        doc.redo().unwrap();
        let layer = doc.find_layer(adj_id).unwrap();
        if let LayerKind::Adjustment(crate::layer::Adjustment::Levels { gamma, .. }) = &layer.kind {
            assert_eq!(*gamma, 2.0);
        } else {
            panic!("expected Levels after redo");
        }
    }

    #[test]
    fn undo_add_adjustment_layer() {
        let mut doc = Document::new("Test", 100, 100);
        doc.add_adjustment_layer(
            "HS",
            crate::layer::Adjustment::HueSaturation {
                hue: 0.0,
                saturation: 0.5,
                lightness: 0.0,
            },
        );
        assert_eq!(doc.layers.len(), 2);
        doc.undo().unwrap();
        assert_eq!(doc.layers.len(), 1);
    }

    #[test]
    fn duplicate_adjustment_layer() {
        let mut doc = Document::new("Test", 100, 100);
        let adj_id = doc.add_adjustment_layer(
            "Curves",
            crate::layer::Adjustment::Curves {
                points: vec![(0.0, 0.0), (0.5, 0.7), (1.0, 1.0)],
            },
        );
        let dup_id = doc.duplicate_layer(adj_id).unwrap();
        assert_eq!(doc.layers.len(), 3);
        assert_ne!(adj_id, dup_id);
        let dup = doc.find_layer(dup_id).unwrap();
        assert!(matches!(dup.kind, LayerKind::Adjustment(_)));
    }
}
