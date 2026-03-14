use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use rasa_core::color::BlendMode;
use rasa_core::layer::Layer;

use crate::state::SessionState;

/// MCP tool definition for tools/list response.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Return all 5 MCP tool definitions.
pub fn list_tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "rasa_open_image".into(),
            description: "Open an image file or create a new blank document".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to open (PNG, JPEG, WebP, TIFF). Omit to create blank." },
                    "name": { "type": "string", "description": "Document name (for new documents)" },
                    "width": { "type": "integer", "description": "Width in pixels (for new documents)" },
                    "height": { "type": "integer", "description": "Height in pixels (for new documents)" }
                }
            }),
        },
        ToolDef {
            name: "rasa_edit_layer".into(),
            description: "Add, modify, or transform layers in a document".into(),
            input_schema: json!({
                "type": "object",
                "required": ["document_id", "action"],
                "properties": {
                    "document_id": { "type": "string", "description": "Document UUID" },
                    "action": {
                        "type": "string",
                        "enum": ["add", "remove", "rename", "set_opacity", "set_blend_mode", "set_visibility", "duplicate", "reorder", "merge_down"],
                        "description": "Layer operation to perform"
                    },
                    "layer_id": { "type": "string", "description": "Target layer UUID (for existing layers)" },
                    "name": { "type": "string", "description": "Layer name (for add/rename)" },
                    "opacity": { "type": "number", "description": "Opacity 0.0-1.0 (for set_opacity)" },
                    "blend_mode": { "type": "string", "description": "Blend mode name (for set_blend_mode)" },
                    "visible": { "type": "boolean", "description": "Visibility (for set_visibility)" },
                    "index": { "type": "integer", "description": "Target index (for reorder)" }
                }
            }),
        },
        ToolDef {
            name: "rasa_apply_filter".into(),
            description: "Apply filters or adjustments to a layer".into(),
            input_schema: json!({
                "type": "object",
                "required": ["document_id", "layer_id", "filter"],
                "properties": {
                    "document_id": { "type": "string", "description": "Document UUID" },
                    "layer_id": { "type": "string", "description": "Layer UUID" },
                    "filter": {
                        "type": "string",
                        "enum": ["brightness_contrast", "hue_saturation", "blur", "sharpen", "invert", "grayscale"],
                        "description": "Filter to apply"
                    },
                    "brightness": { "type": "number" },
                    "contrast": { "type": "number" },
                    "hue": { "type": "number" },
                    "saturation": { "type": "number" },
                    "lightness": { "type": "number" },
                    "radius": { "type": "integer" },
                    "amount": { "type": "number" }
                }
            }),
        },
        ToolDef {
            name: "rasa_get_document".into(),
            description: "Get document state: layers, dimensions, history".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "document_id": { "type": "string", "description": "Document UUID. Omit to list all open documents." }
                }
            }),
        },
        ToolDef {
            name: "rasa_export".into(),
            description: "Export document to an image file (PNG, JPEG, WebP, TIFF)".into(),
            input_schema: json!({
                "type": "object",
                "required": ["document_id", "path"],
                "properties": {
                    "document_id": { "type": "string", "description": "Document UUID" },
                    "path": { "type": "string", "description": "Output file path" },
                    "quality": { "type": "integer", "description": "JPEG quality 1-100 (default 90)" }
                }
            }),
        },
    ]
}

/// Call a tool by name with the given arguments.
pub fn call_tool(
    state: &SessionState,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    match name {
        "rasa_open_image" => tool_open_image(state, args),
        "rasa_edit_layer" => tool_edit_layer(state, args),
        "rasa_apply_filter" => tool_apply_filter(state, args),
        "rasa_get_document" => tool_get_document(state, args),
        "rasa_export" => tool_export(state, args),
        _ => Err(format!("unknown tool: {name}")),
    }
}

/// Maximum dimension allowed via MCP (16384 = 16K).
const MAX_MCP_DIMENSION: u32 = 16384;

fn tool_open_image(state: &SessionState, args: &Value) -> Result<Value, String> {
    if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
        let path = PathBuf::from(path);
        // Validate the path exists and is a file
        if !path.is_file() {
            return Err(format!("file not found: {}", path.display()));
        }
        let id = state
            .open_image(&path)
            .map_err(|e| e.to_string())?;
        let info = state
            .with_doc(id, |d| {
                json!({
                    "document_id": d.id.to_string(),
                    "name": d.name,
                    "width": d.size.width,
                    "height": d.size.height,
                    "layers": d.layers.len()
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(info)
    } else {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let width = (args
            .get("width")
            .and_then(|v| v.as_u64())
            .unwrap_or(1920) as u32)
            .clamp(1, MAX_MCP_DIMENSION);
        let height = (args
            .get("height")
            .and_then(|v| v.as_u64())
            .unwrap_or(1080) as u32)
            .clamp(1, MAX_MCP_DIMENSION);
        let id = state.create_document(name, width, height);
        Ok(json!({
            "document_id": id.to_string(),
            "name": name,
            "width": width,
            "height": height,
            "layers": 1
        }))
    }
}

fn tool_edit_layer(state: &SessionState, args: &Value) -> Result<Value, String> {
    let doc_id = parse_uuid(args, "document_id")?;
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or("missing action")?;

    match action {
        "add" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("New Layer");
            let layer_id = state
                .with_doc_mut(doc_id, |d| {
                    let (w, h) = (d.size.width, d.size.height);
                    let layer = Layer::new_raster(name, w, h);
                    let id = layer.id;
                    d.add_layer(layer);
                    id
                })
                .map_err(|e| e.to_string())?;
            Ok(json!({ "layer_id": layer_id.to_string(), "action": "added" }))
        }
        "remove" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            state
                .with_doc_mut(doc_id, |d| d.remove_layer(layer_id))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "removed" }))
        }
        "rename" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or("missing name")?;
            state
                .with_doc_mut(doc_id, |d| d.rename_layer(layer_id, name))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "renamed", "name": name }))
        }
        "set_opacity" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            let opacity = args
                .get("opacity")
                .and_then(|v| v.as_f64())
                .ok_or("missing opacity")? as f32;
            state
                .with_doc_mut(doc_id, |d| d.set_layer_opacity(layer_id, opacity))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "opacity_set", "opacity": opacity }))
        }
        "set_blend_mode" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            let mode_str = args
                .get("blend_mode")
                .and_then(|v| v.as_str())
                .ok_or("missing blend_mode")?;
            let mode = parse_blend_mode(mode_str)?;
            state
                .with_doc_mut(doc_id, |d| d.set_layer_blend_mode(layer_id, mode))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "blend_mode_set", "blend_mode": mode_str }))
        }
        "set_visibility" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            let visible = args
                .get("visible")
                .and_then(|v| v.as_bool())
                .ok_or("missing visible")?;
            state
                .with_doc_mut(doc_id, |d| d.set_layer_visibility(layer_id, visible))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "visibility_set", "visible": visible }))
        }
        "duplicate" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            let new_id = state
                .with_doc_mut(doc_id, |d| d.duplicate_layer(layer_id))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "duplicated", "new_layer_id": new_id.to_string() }))
        }
        "reorder" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            let index = args
                .get("index")
                .and_then(|v| v.as_u64())
                .ok_or("missing index")? as usize;
            state
                .with_doc_mut(doc_id, |d| d.reorder_layer(layer_id, index))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "reordered", "index": index }))
        }
        "merge_down" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            state
                .with_doc_mut(doc_id, |d| d.merge_down(layer_id))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({ "action": "merged_down" }))
        }
        _ => Err(format!("unknown action: {action}")),
    }
}

fn tool_apply_filter(state: &SessionState, args: &Value) -> Result<Value, String> {
    let doc_id = parse_uuid(args, "document_id")?;
    let layer_id = parse_uuid(args, "layer_id")?;
    let filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .ok_or("missing filter")?;

    state
        .with_doc_mut(doc_id, |d| {
            let buf = d
                .get_pixels_mut(layer_id)
                .ok_or_else(|| format!("no pixel data for layer {layer_id}"))?;

            match filter {
                "brightness_contrast" => {
                    let b = args.get("brightness").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let c = args.get("contrast").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    rasa_engine::filters::apply_adjustment(
                        buf,
                        &rasa_core::layer::Adjustment::BrightnessContrast {
                            brightness: b,
                            contrast: c,
                        },
                    );
                }
                "hue_saturation" => {
                    let h = args.get("hue").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let s = args.get("saturation").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let l = args.get("lightness").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    rasa_engine::filters::apply_adjustment(
                        buf,
                        &rasa_core::layer::Adjustment::HueSaturation {
                            hue: h,
                            saturation: s,
                            lightness: l,
                        },
                    );
                }
                "blur" => {
                    let r = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(3) as u32;
                    rasa_engine::filters::gaussian_blur(buf, r);
                }
                "sharpen" => {
                    let r = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                    let a = args.get("amount").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    rasa_engine::filters::sharpen(buf, r, a);
                }
                "invert" => {
                    rasa_engine::filters::invert(buf);
                }
                "grayscale" => {
                    rasa_engine::filters::grayscale(buf);
                }
                _ => return Err(format!("unknown filter: {filter}")),
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?
        .map_err(|e: String| e)?;

    Ok(json!({ "filter": filter, "applied": true }))
}

fn tool_get_document(state: &SessionState, args: &Value) -> Result<Value, String> {
    if let Some(id_str) = args.get("document_id").and_then(|v| v.as_str()) {
        let doc_id =
            Uuid::parse_str(id_str).map_err(|_| format!("invalid UUID: {id_str}"))?;
        state
            .with_doc(doc_id, |d| {
                let layers: Vec<Value> = d
                    .layers
                    .iter()
                    .map(|l| {
                        json!({
                            "id": l.id.to_string(),
                            "name": l.name,
                            "visible": l.visible,
                            "locked": l.locked,
                            "opacity": l.opacity,
                            "blend_mode": format!("{:?}", l.blend_mode),
                            "kind": format!("{:?}", l.kind),
                        })
                    })
                    .collect();
                json!({
                    "document_id": d.id.to_string(),
                    "name": d.name,
                    "width": d.size.width,
                    "height": d.size.height,
                    "dpi": d.dpi,
                    "layers": layers,
                    "active_layer": d.active_layer.map(|id| id.to_string()),
                    "can_undo": d.can_undo(),
                    "can_redo": d.can_redo(),
                })
            })
            .map_err(|e| e.to_string())
    } else {
        let docs = state.list_documents();
        let list: Vec<Value> = docs
            .iter()
            .map(|(id, name, w, h)| {
                json!({
                    "document_id": id.to_string(),
                    "name": name,
                    "width": w,
                    "height": h,
                })
            })
            .collect();
        Ok(json!({ "documents": list }))
    }
}

fn tool_export(state: &SessionState, args: &Value) -> Result<Value, String> {
    let doc_id = parse_uuid(args, "document_id")?;
    let path_str = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing path")?;
    let path = PathBuf::from(path_str);

    // Validate parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(format!("directory does not exist: {}", parent.display()));
        }
    }

    let format = rasa_storage::format::ImageFormat::from_path(&path)
        .ok_or_else(|| format!("unsupported format for: {path_str}"))?;

    let quality = args
        .get("quality")
        .and_then(|v| v.as_u64())
        .unwrap_or(90) as u8;

    let settings = match format {
        rasa_storage::format::ImageFormat::Jpeg => {
            rasa_storage::format::ExportSettings::Jpeg(rasa_storage::format::JpegQuality::new(
                quality,
            ))
        }
        _ => rasa_storage::format::ExportSettings::for_format(format),
    };

    // Composite the document and export
    state
        .with_doc(doc_id, |d| {
            let composited = rasa_engine::compositor::composite(d);
            rasa_storage::export::export_buffer(&composited, &path, &settings)
                .map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())?
        .map_err(|e| e)?;

    Ok(json!({
        "exported": true,
        "path": path_str,
        "format": format!("{:?}", format),
    }))
}

// ── Helpers ──

fn parse_uuid(args: &Value, key: &str) -> Result<Uuid, String> {
    let s = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing {key}"))?;
    Uuid::parse_str(s).map_err(|_| format!("invalid UUID for {key}: {s}"))
}

fn parse_blend_mode(s: &str) -> Result<BlendMode, String> {
    match s.to_lowercase().as_str() {
        "normal" => Ok(BlendMode::Normal),
        "multiply" => Ok(BlendMode::Multiply),
        "screen" => Ok(BlendMode::Screen),
        "overlay" => Ok(BlendMode::Overlay),
        "darken" => Ok(BlendMode::Darken),
        "lighten" => Ok(BlendMode::Lighten),
        "color_dodge" | "colordodge" => Ok(BlendMode::ColorDodge),
        "color_burn" | "colorburn" => Ok(BlendMode::ColorBurn),
        "soft_light" | "softlight" => Ok(BlendMode::SoftLight),
        "hard_light" | "hardlight" => Ok(BlendMode::HardLight),
        "difference" => Ok(BlendMode::Difference),
        "exclusion" => Ok(BlendMode::Exclusion),
        _ => Err(format!("unknown blend mode: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_tools_returns_five() {
        let tools = list_tools();
        assert_eq!(tools.len(), 5);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"rasa_open_image"));
        assert!(names.contains(&"rasa_edit_layer"));
        assert!(names.contains(&"rasa_apply_filter"));
        assert!(names.contains(&"rasa_get_document"));
        assert!(names.contains(&"rasa_export"));
    }

    #[test]
    fn tool_schemas_valid_json() {
        for tool in list_tools() {
            assert!(tool.input_schema.is_object());
        }
    }

    #[test]
    fn open_create_document() {
        let state = SessionState::new();
        let result = call_tool(&state, "rasa_open_image", &json!({
            "name": "Test Canvas",
            "width": 800,
            "height": 600,
        }))
        .unwrap();
        assert_eq!(result["name"], "Test Canvas");
        assert_eq!(result["width"], 800);
        assert_eq!(result["height"], 600);
        assert!(result["document_id"].is_string());
    }

    #[test]
    fn get_document_list() {
        let state = SessionState::new();
        state.create_document("A", 10, 10);
        state.create_document("B", 20, 20);
        let result = call_tool(&state, "rasa_get_document", &json!({})).unwrap();
        assert_eq!(result["documents"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn get_document_detail() {
        let state = SessionState::new();
        let id = state.create_document("Detail", 100, 100);
        let result = call_tool(
            &state,
            "rasa_get_document",
            &json!({ "document_id": id.to_string() }),
        )
        .unwrap();
        assert_eq!(result["name"], "Detail");
        assert_eq!(result["width"], 100);
        assert!(result["layers"].is_array());
        assert_eq!(result["layers"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn edit_layer_add() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add",
                "name": "Paint Layer",
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "added");
        assert!(result["layer_id"].is_string());
    }

    #[test]
    fn edit_layer_set_opacity() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "set_opacity",
                "layer_id": layer_id.to_string(),
                "opacity": 0.5,
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "opacity_set");
    }

    #[test]
    fn apply_filter_invert() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_apply_filter",
            &json!({
                "document_id": id.to_string(),
                "layer_id": layer_id.to_string(),
                "filter": "invert",
            }),
        )
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn unknown_tool_errors() {
        let state = SessionState::new();
        let result = call_tool(&state, "nonexistent", &json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn parse_blend_modes() {
        assert_eq!(parse_blend_mode("normal").unwrap(), BlendMode::Normal);
        assert_eq!(parse_blend_mode("Multiply").unwrap(), BlendMode::Multiply);
        assert_eq!(parse_blend_mode("soft_light").unwrap(), BlendMode::SoftLight);
        assert!(parse_blend_mode("bogus").is_err());
    }

    #[test]
    fn open_clamps_huge_dimensions() {
        let state = SessionState::new();
        let result = call_tool(&state, "rasa_open_image", &json!({
            "width": 99999,
            "height": 99999,
        }))
        .unwrap();
        assert!(result["width"].as_u64().unwrap() <= MAX_MCP_DIMENSION as u64);
        assert!(result["height"].as_u64().unwrap() <= MAX_MCP_DIMENSION as u64);
    }

    #[test]
    fn open_nonexistent_file_errors() {
        let state = SessionState::new();
        let result = call_tool(&state, "rasa_open_image", &json!({
            "path": "/nonexistent/fake_image.png",
        }));
        assert!(result.is_err());
    }

    #[test]
    fn export_bad_directory_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let result = call_tool(&state, "rasa_export", &json!({
            "document_id": id.to_string(),
            "path": "/nonexistent_dir/output.png",
        }));
        assert!(result.is_err());
    }

    #[test]
    fn edit_layer_duplicate() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "duplicate",
            "layer_id": layer_id.to_string(),
        }))
        .unwrap();
        assert_eq!(result["action"], "duplicated");
        let count = state.with_doc(id, |d| d.layers.len()).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn edit_layer_rename() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "rename",
            "layer_id": layer_id.to_string(),
            "name": "Renamed",
        }))
        .unwrap();
        assert_eq!(result["action"], "renamed");
    }

    #[test]
    fn apply_filter_blur() {
        let state = SessionState::new();
        let id = state.create_document("Test", 8, 8);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_apply_filter", &json!({
            "document_id": id.to_string(),
            "layer_id": layer_id.to_string(),
            "filter": "blur",
            "radius": 2,
        }))
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_brightness() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_apply_filter", &json!({
            "document_id": id.to_string(),
            "layer_id": layer_id.to_string(),
            "filter": "brightness_contrast",
            "brightness": 0.2,
            "contrast": 0.1,
        }))
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn edit_layer_set_visibility() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "set_visibility",
            "layer_id": layer_id.to_string(),
            "visible": false,
        }))
        .unwrap();
        assert_eq!(result["action"], "visibility_set");
    }

    #[test]
    fn edit_layer_set_blend_mode() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "set_blend_mode",
            "layer_id": layer_id.to_string(),
            "blend_mode": "multiply",
        }))
        .unwrap();
        assert_eq!(result["action"], "blend_mode_set");
    }

    #[test]
    fn edit_layer_reorder() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "add",
            "name": "Top",
        })).unwrap();
        let layer_id = state.with_doc(id, |d| d.layers[1].id).unwrap();
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "reorder",
            "layer_id": layer_id.to_string(),
            "index": 0,
        }))
        .unwrap();
        assert_eq!(result["action"], "reordered");
    }

    #[test]
    fn edit_layer_remove() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "add",
            "name": "Extra",
        })).unwrap();
        let extra_id = state.with_doc(id, |d| d.layers[1].id).unwrap();
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "remove",
            "layer_id": extra_id.to_string(),
        }))
        .unwrap();
        assert_eq!(result["action"], "removed");
    }

    #[test]
    fn edit_layer_merge_down() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "add",
            "name": "Upper",
        })).unwrap();
        let upper_id = state.with_doc(id, |d| d.layers[1].id).unwrap();
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "merge_down",
            "layer_id": upper_id.to_string(),
        }))
        .unwrap();
        assert_eq!(result["action"], "merged_down");
    }

    #[test]
    fn apply_filter_grayscale() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_apply_filter", &json!({
            "document_id": id.to_string(),
            "layer_id": layer_id.to_string(),
            "filter": "grayscale",
        }))
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_sharpen() {
        let state = SessionState::new();
        let id = state.create_document("Test", 8, 8);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_apply_filter", &json!({
            "document_id": id.to_string(),
            "layer_id": layer_id.to_string(),
            "filter": "sharpen",
            "radius": 1,
            "amount": 0.5,
        }))
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_hue_saturation() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_apply_filter", &json!({
            "document_id": id.to_string(),
            "layer_id": layer_id.to_string(),
            "filter": "hue_saturation",
            "hue": 30.0,
            "saturation": 0.1,
        }))
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_unknown_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(&state, "rasa_apply_filter", &json!({
            "document_id": id.to_string(),
            "layer_id": layer_id.to_string(),
            "filter": "bogus_filter",
        }));
        assert!(result.is_err());
    }

    #[test]
    fn edit_layer_unknown_action_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let result = call_tool(&state, "rasa_edit_layer", &json!({
            "document_id": id.to_string(),
            "action": "explode",
        }));
        assert!(result.is_err());
    }
}
