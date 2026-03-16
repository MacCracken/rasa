use std::path::PathBuf;

use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use rasa_core::color::BlendMode;
use rasa_core::layer::{Adjustment, Layer};

use crate::state::SessionState;

/// MCP tool definition for tools/list response.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Return all 6 MCP tool definitions.
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
                        "enum": ["add", "remove", "rename", "set_opacity", "set_blend_mode", "set_visibility", "duplicate", "reorder", "merge_down", "add_adjustment", "set_adjustment"],
                        "description": "Layer operation to perform"
                    },
                    "layer_id": { "type": "string", "description": "Target layer UUID (for existing layers)" },
                    "name": { "type": "string", "description": "Layer name (for add/rename)" },
                    "opacity": { "type": "number", "description": "Opacity 0.0-1.0 (for set_opacity)" },
                    "blend_mode": { "type": "string", "description": "Blend mode name (for set_blend_mode)" },
                    "visible": { "type": "boolean", "description": "Visibility (for set_visibility)" },
                    "index": { "type": "integer", "description": "Target index (for reorder)" },
                    "adjustment_type": {
                        "type": "string",
                        "enum": ["brightness_contrast", "hue_saturation", "curves", "levels"],
                        "description": "Adjustment type (for add_adjustment/set_adjustment)"
                    },
                    "brightness": { "type": "number", "description": "Brightness -1.0 to 1.0" },
                    "contrast": { "type": "number", "description": "Contrast -1.0 to 1.0" },
                    "hue": { "type": "number", "description": "Hue shift -180 to 180 degrees" },
                    "saturation": { "type": "number", "description": "Saturation -1.0 to 1.0" },
                    "lightness": { "type": "number", "description": "Lightness -1.0 to 1.0" },
                    "black": { "type": "number", "description": "Black point 0.0-1.0 (for levels)" },
                    "white": { "type": "number", "description": "White point 0.0-1.0 (for levels)" },
                    "gamma": { "type": "number", "description": "Gamma 0.1-10.0 (for levels)" }
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
        ToolDef {
            name: "rasa_batch_export".into(),
            description: "Batch process multiple image files: import, apply filters, export".into(),
            input_schema: json!({
                "type": "object",
                "required": ["input_paths", "output_dir"],
                "properties": {
                    "input_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of input file paths"
                    },
                    "output_dir": { "type": "string", "description": "Output directory path" },
                    "format": {
                        "type": "string",
                        "enum": ["png", "jpeg", "webp", "tiff", "bmp", "gif", "psd"],
                        "description": "Output format (default: keep original)"
                    },
                    "quality": { "type": "integer", "description": "JPEG quality 1-100 (default 90)" },
                    "filters": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "enum": ["invert", "grayscale", "brightness_contrast", "hue_saturation", "blur", "sharpen"]
                                },
                                "brightness": { "type": "number" },
                                "contrast": { "type": "number" },
                                "hue": { "type": "number" },
                                "saturation": { "type": "number" },
                                "lightness": { "type": "number" },
                                "radius": { "type": "number" },
                                "amount": { "type": "number" }
                            }
                        },
                        "description": "Filters to apply to each image in order"
                    }
                }
            }),
        },
    ]
}

/// Call a tool by name with the given arguments.
pub fn call_tool(state: &SessionState, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "rasa_open_image" => tool_open_image(state, args),
        "rasa_edit_layer" => tool_edit_layer(state, args),
        "rasa_apply_filter" => tool_apply_filter(state, args),
        "rasa_get_document" => tool_get_document(state, args),
        "rasa_export" => tool_export(state, args),
        "rasa_batch_export" => tool_batch_export(args),
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
        let id = state.open_image(&path).map_err(|e| e.to_string())?;
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
        let width = (args.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32)
            .clamp(1, MAX_MCP_DIMENSION);
        let height = (args.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32)
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
        "add_adjustment" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Adjustment");
            let adjustment = parse_adjustment(args)?;
            let layer_id = state
                .with_doc_mut(doc_id, |d| d.add_adjustment_layer(name, adjustment))
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "layer_id": layer_id.to_string(),
                "action": "adjustment_added",
                "non_destructive": true,
            }))
        }
        "set_adjustment" => {
            let layer_id = parse_uuid(args, "layer_id")?;
            let adjustment = parse_adjustment(args)?;
            state
                .with_doc_mut(doc_id, |d| d.set_adjustment(layer_id, adjustment))
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "action": "adjustment_updated",
                "non_destructive": true,
            }))
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
                    let b = args
                        .get("brightness")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as f32;
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
                    let s = args
                        .get("saturation")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as f32;
                    let l = args
                        .get("lightness")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as f32;
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
        let doc_id = Uuid::parse_str(id_str).map_err(|_| format!("invalid UUID: {id_str}"))?;
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
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.is_dir()
    {
        return Err(format!("directory does not exist: {}", parent.display()));
    }

    let format = rasa_storage::format::ImageFormat::from_path(&path)
        .ok_or_else(|| format!("unsupported format for: {path_str}"))?;

    let quality = args.get("quality").and_then(|v| v.as_u64()).unwrap_or(90) as u8;

    let settings = match format {
        rasa_storage::format::ImageFormat::Jpeg => rasa_storage::format::ExportSettings::Jpeg(
            rasa_storage::format::JpegQuality::new(quality),
        ),
        _ => rasa_storage::format::ExportSettings::for_format(format),
    };

    // Composite the document and export
    state
        .with_doc(doc_id, |d| {
            let composited = rasa_engine::compositor::composite(d);
            rasa_storage::export::export_buffer(&composited, &path, &settings)
                .map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())??;

    Ok(json!({
        "exported": true,
        "path": path_str,
        "format": format!("{:?}", format),
    }))
}

fn tool_batch_export(args: &Value) -> Result<Value, String> {
    use rasa_storage::batch::BatchJob;
    use rasa_storage::format::ImageFormat;

    let input_paths: Vec<PathBuf> = args
        .get("input_paths")
        .and_then(|v| v.as_array())
        .ok_or("missing input_paths")?
        .iter()
        .filter_map(|v| v.as_str().map(PathBuf::from))
        .collect();

    if input_paths.is_empty() {
        return Err("input_paths must not be empty".into());
    }

    const MAX_BATCH_SIZE: usize = 1000;
    if input_paths.len() > MAX_BATCH_SIZE {
        return Err(format!(
            "batch size {} exceeds maximum of {MAX_BATCH_SIZE}",
            input_paths.len()
        ));
    }

    let output_dir = PathBuf::from(
        args.get("output_dir")
            .and_then(|v| v.as_str())
            .ok_or("missing output_dir")?,
    );

    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| {
            let ext = format!("file.{s}");
            ImageFormat::from_path(std::path::Path::new(&ext))
                .ok_or_else(|| format!("unknown format: {s}"))
        })
        .transpose()?;

    let jpeg_quality = args
        .get("quality")
        .and_then(|v| v.as_u64())
        .unwrap_or(90)
        .clamp(1, 100) as u8;

    let filters: Vec<_> = args
        .get("filters")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(parse_batch_filter)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    let job = BatchJob {
        input_paths,
        output_dir,
        format,
        jpeg_quality,
        filters,
        icc_profile: None,
    };

    let result = job.run().map_err(|e| e.to_string())?;

    let file_results: Vec<Value> = result
        .results
        .iter()
        .map(|r| {
            json!({
                "input": r.input.display().to_string(),
                "output": r.output.as_ref().map(|p| p.display().to_string()),
                "error": r.error,
            })
        })
        .collect();

    Ok(json!({
        "total": result.total,
        "succeeded": result.succeeded,
        "failed": result.failed,
        "results": file_results,
    }))
}

fn parse_batch_filter(v: &Value) -> Result<rasa_storage::batch::BatchFilter, String> {
    use rasa_storage::batch::BatchFilter;

    let name = v
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or("filter missing name")?;

    match name {
        "invert" => Ok(BatchFilter::Invert),
        "grayscale" => Ok(BatchFilter::Grayscale),
        "brightness_contrast" => Ok(BatchFilter::BrightnessContrast {
            brightness: v.get("brightness").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32,
            contrast: v.get("contrast").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32,
        }),
        "hue_saturation" => Ok(BatchFilter::HueSaturation {
            hue: v.get("hue").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32,
            saturation: v.get("saturation").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32,
            lightness: v.get("lightness").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32,
        }),
        "blur" => Ok(BatchFilter::GaussianBlur {
            radius: v
                .get("radius")
                .and_then(|n| n.as_u64())
                .unwrap_or(2)
                .clamp(1, 500) as u32,
        }),
        "sharpen" => Ok(BatchFilter::Sharpen {
            radius: v
                .get("radius")
                .and_then(|n| n.as_u64())
                .unwrap_or(1)
                .clamp(1, 500) as u32,
            amount: v.get("amount").and_then(|n| n.as_f64()).unwrap_or(0.5) as f32,
        }),
        _ => Err(format!("unknown filter: {name}")),
    }
}

// ── Helpers ──

fn parse_uuid(args: &Value, key: &str) -> Result<Uuid, String> {
    let s = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing {key}"))?;
    Uuid::parse_str(s).map_err(|_| format!("invalid UUID for {key}: {s}"))
}

fn parse_adjustment(args: &Value) -> Result<Adjustment, String> {
    let adj_type = args
        .get("adjustment_type")
        .and_then(|v| v.as_str())
        .ok_or("missing adjustment_type")?;
    match adj_type {
        "brightness_contrast" => {
            let brightness = args
                .get("brightness")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            let contrast = args.get("contrast").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            Ok(Adjustment::BrightnessContrast {
                brightness: brightness.clamp(-1.0, 1.0),
                contrast: contrast.clamp(-1.0, 1.0),
            })
        }
        "hue_saturation" => {
            let hue = args.get("hue").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let saturation = args
                .get("saturation")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            let lightness = args
                .get("lightness")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            Ok(Adjustment::HueSaturation {
                hue: hue.clamp(-180.0, 180.0),
                saturation: saturation.clamp(-1.0, 1.0),
                lightness: lightness.clamp(-1.0, 1.0),
            })
        }
        "curves" => {
            let points = args
                .get("points")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            let a = p.as_array()?;
                            Some((a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32))
                        })
                        .collect()
                })
                .unwrap_or_else(|| vec![(0.0, 0.0), (1.0, 1.0)]);
            Ok(Adjustment::Curves { points })
        }
        "levels" => {
            let black = args.get("black").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let white = args.get("white").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            let gamma = args.get("gamma").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            Ok(Adjustment::Levels {
                black: black.clamp(0.0, 1.0),
                white: white.clamp(0.0, 1.0),
                gamma: gamma.clamp(0.1, 10.0),
            })
        }
        _ => Err(format!("unknown adjustment_type: {adj_type}")),
    }
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
    fn list_tools_returns_six() {
        let tools = list_tools();
        assert_eq!(tools.len(), 6);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"rasa_open_image"));
        assert!(names.contains(&"rasa_edit_layer"));
        assert!(names.contains(&"rasa_apply_filter"));
        assert!(names.contains(&"rasa_get_document"));
        assert!(names.contains(&"rasa_export"));
        assert!(names.contains(&"rasa_batch_export"));
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
        let result = call_tool(
            &state,
            "rasa_open_image",
            &json!({
                "name": "Test Canvas",
                "width": 800,
                "height": 600,
            }),
        )
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
        assert_eq!(
            parse_blend_mode("soft_light").unwrap(),
            BlendMode::SoftLight
        );
        assert!(parse_blend_mode("bogus").is_err());
    }

    #[test]
    fn open_clamps_huge_dimensions() {
        let state = SessionState::new();
        let result = call_tool(
            &state,
            "rasa_open_image",
            &json!({
                "width": 99999,
                "height": 99999,
            }),
        )
        .unwrap();
        assert!(result["width"].as_u64().unwrap() <= MAX_MCP_DIMENSION as u64);
        assert!(result["height"].as_u64().unwrap() <= MAX_MCP_DIMENSION as u64);
    }

    #[test]
    fn open_nonexistent_file_errors() {
        let state = SessionState::new();
        let result = call_tool(
            &state,
            "rasa_open_image",
            &json!({
                "path": "/nonexistent/fake_image.png",
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn export_bad_directory_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let result = call_tool(
            &state,
            "rasa_export",
            &json!({
                "document_id": id.to_string(),
                "path": "/nonexistent_dir/output.png",
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn edit_layer_duplicate() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "duplicate",
                "layer_id": layer_id.to_string(),
            }),
        )
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
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "rename",
                "layer_id": layer_id.to_string(),
                "name": "Renamed",
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "renamed");
    }

    #[test]
    fn apply_filter_blur() {
        let state = SessionState::new();
        let id = state.create_document("Test", 8, 8);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_apply_filter",
            &json!({
                "document_id": id.to_string(),
                "layer_id": layer_id.to_string(),
                "filter": "blur",
                "radius": 2,
            }),
        )
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_brightness() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_apply_filter",
            &json!({
                "document_id": id.to_string(),
                "layer_id": layer_id.to_string(),
                "filter": "brightness_contrast",
                "brightness": 0.2,
                "contrast": 0.1,
            }),
        )
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn edit_layer_set_visibility() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "set_visibility",
                "layer_id": layer_id.to_string(),
                "visible": false,
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "visibility_set");
    }

    #[test]
    fn edit_layer_set_blend_mode() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "set_blend_mode",
                "layer_id": layer_id.to_string(),
                "blend_mode": "multiply",
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "blend_mode_set");
    }

    #[test]
    fn edit_layer_reorder() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add",
                "name": "Top",
            }),
        )
        .unwrap();
        let layer_id = state.with_doc(id, |d| d.layers[1].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "reorder",
                "layer_id": layer_id.to_string(),
                "index": 0,
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "reordered");
    }

    #[test]
    fn edit_layer_remove() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add",
                "name": "Extra",
            }),
        )
        .unwrap();
        let extra_id = state.with_doc(id, |d| d.layers[1].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "remove",
                "layer_id": extra_id.to_string(),
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "removed");
    }

    #[test]
    fn edit_layer_merge_down() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add",
                "name": "Upper",
            }),
        )
        .unwrap();
        let upper_id = state.with_doc(id, |d| d.layers[1].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "merge_down",
                "layer_id": upper_id.to_string(),
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "merged_down");
    }

    #[test]
    fn apply_filter_grayscale() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_apply_filter",
            &json!({
                "document_id": id.to_string(),
                "layer_id": layer_id.to_string(),
                "filter": "grayscale",
            }),
        )
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_sharpen() {
        let state = SessionState::new();
        let id = state.create_document("Test", 8, 8);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_apply_filter",
            &json!({
                "document_id": id.to_string(),
                "layer_id": layer_id.to_string(),
                "filter": "sharpen",
                "radius": 1,
                "amount": 0.5,
            }),
        )
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_hue_saturation() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_apply_filter",
            &json!({
                "document_id": id.to_string(),
                "layer_id": layer_id.to_string(),
                "filter": "hue_saturation",
                "hue": 30.0,
                "saturation": 0.1,
            }),
        )
        .unwrap();
        assert_eq!(result["applied"], true);
    }

    #[test]
    fn apply_filter_unknown_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 4, 4);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_apply_filter",
            &json!({
                "document_id": id.to_string(),
                "layer_id": layer_id.to_string(),
                "filter": "bogus_filter",
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn edit_layer_unknown_action_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "explode",
            }),
        );
        assert!(result.is_err());
    }

    // ── Non-destructive adjustment layer tests ──────────

    #[test]
    fn add_adjustment_brightness_contrast() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add_adjustment",
                "name": "Brighten",
                "adjustment_type": "brightness_contrast",
                "brightness": 0.3,
                "contrast": 0.1,
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "adjustment_added");
        assert_eq!(result["non_destructive"], true);
        assert!(result["layer_id"].is_string());
        let count = state.with_doc(id, |d| d.layers.len()).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn add_adjustment_hue_saturation() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add_adjustment",
                "adjustment_type": "hue_saturation",
                "hue": 45.0,
                "saturation": -0.3,
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "adjustment_added");
    }

    #[test]
    fn add_adjustment_levels() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add_adjustment",
                "adjustment_type": "levels",
                "black": 0.05,
                "white": 0.95,
                "gamma": 1.2,
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "adjustment_added");
    }

    #[test]
    fn add_adjustment_curves() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add_adjustment",
                "adjustment_type": "curves",
                "points": [[0.0, 0.0], [0.5, 0.7], [1.0, 1.0]],
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "adjustment_added");
    }

    #[test]
    fn set_adjustment_updates_existing() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let add_result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add_adjustment",
                "adjustment_type": "brightness_contrast",
                "brightness": 0.1,
            }),
        )
        .unwrap();
        let layer_id = add_result["layer_id"].as_str().unwrap();

        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "set_adjustment",
                "layer_id": layer_id,
                "adjustment_type": "brightness_contrast",
                "brightness": 0.8,
                "contrast": 0.5,
            }),
        )
        .unwrap();
        assert_eq!(result["action"], "adjustment_updated");
        assert_eq!(result["non_destructive"], true);
    }

    #[test]
    fn set_adjustment_on_raster_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let layer_id = state.with_doc(id, |d| d.layers[0].id).unwrap();
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "set_adjustment",
                "layer_id": layer_id.to_string(),
                "adjustment_type": "brightness_contrast",
                "brightness": 0.5,
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn batch_export_basic() {
        // Create test input files
        let dir = std::env::temp_dir().join("rasa_test_mcp_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input_path = dir.join("mcp_batch_input.png");
        let buf = rasa_core::pixel::PixelBuffer::filled(
            4,
            4,
            rasa_core::color::Color::new(1.0, 0.0, 0.0, 1.0),
        );
        rasa_storage::export::export_buffer(
            &buf,
            &input_path,
            &rasa_storage::format::ExportSettings::Png,
        )
        .unwrap();

        let output_dir = dir.join("mcp_batch_output");
        let state = SessionState::new();
        let result = call_tool(
            &state,
            "rasa_batch_export",
            &json!({
                "input_paths": [input_path.to_str().unwrap()],
                "output_dir": output_dir.to_str().unwrap(),
                "filters": [{ "name": "grayscale" }]
            }),
        )
        .unwrap();

        assert_eq!(result["total"], 1);
        assert_eq!(result["succeeded"], 1);
        assert_eq!(result["failed"], 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn batch_export_missing_inputs_errors() {
        let state = SessionState::new();
        let result = call_tool(
            &state,
            "rasa_batch_export",
            &json!({
                "output_dir": "/tmp/batch_test"
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn add_adjustment_missing_type_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add_adjustment",
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn add_adjustment_unknown_type_errors() {
        let state = SessionState::new();
        let id = state.create_document("Test", 10, 10);
        let result = call_tool(
            &state,
            "rasa_edit_layer",
            &json!({
                "document_id": id.to_string(),
                "action": "add_adjustment",
                "adjustment_type": "vignette",
            }),
        );
        assert!(result.is_err());
    }
}
