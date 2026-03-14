use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::color::BlendMode;
use crate::command::{Command, History};
use crate::error::RasaError;
use crate::geometry::Size;
use crate::layer::{Layer, LayerKind};
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
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
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
        let to = new_index.min(self.layers.len() - 1);
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

    pub fn flatten_visible(&self) -> Vec<(&Layer, Option<&PixelBuffer>)> {
        self.layers
            .iter()
            .filter(|l| l.visible)
            .map(|l| (l, self.get_pixels(l.id)))
            .collect()
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
            .ok_or_else(|| RasaError::Other("nothing to undo".into()))?;
        self.apply_inverse(&cmd);
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), RasaError> {
        let cmd = self
            .history_mut()
            .redo()
            .ok_or_else(|| RasaError::Other("nothing to redo".into()))?;
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
            _ => {}
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
            _ => {}
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
}
