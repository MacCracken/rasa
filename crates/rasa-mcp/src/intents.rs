use serde::Serialize;

/// An agnoshi voice command intent for the AGNOS platform.
#[derive(Debug, Clone, Serialize)]
pub struct AgnosIntent {
    pub name: String,
    pub description: String,
    pub utterances: Vec<String>,
    pub parameters: Vec<IntentParam>,
}

/// A parameter slot within an intent.
#[derive(Debug, Clone, Serialize)]
pub struct IntentParam {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

/// Return all 5 agnoshi intents for Rasa.
pub fn list_intents() -> Vec<AgnosIntent> {
    vec![
        AgnosIntent {
            name: "rasa.open".into(),
            description: "Open an image or create a new canvas".into(),
            utterances: vec![
                "open {file}".into(),
                "create a new {width} by {height} canvas".into(),
                "new image {width} by {height}".into(),
                "open image {file}".into(),
                "load {file}".into(),
            ],
            parameters: vec![
                IntentParam {
                    name: "file".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "File path to open".into(),
                },
                IntentParam {
                    name: "width".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Canvas width".into(),
                },
                IntentParam {
                    name: "height".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Canvas height".into(),
                },
            ],
        },
        AgnosIntent {
            name: "rasa.filter".into(),
            description: "Apply a filter or adjustment to the current layer".into(),
            utterances: vec![
                "apply {filter}".into(),
                "add a {filter} filter".into(),
                "blur this layer".into(),
                "sharpen the image".into(),
                "invert the colors".into(),
                "make it grayscale".into(),
                "increase brightness by {amount}".into(),
                "adjust contrast to {amount}".into(),
            ],
            parameters: vec![
                IntentParam {
                    name: "filter".into(),
                    param_type: "string".into(),
                    required: true,
                    description:
                        "Filter name: blur, sharpen, invert, grayscale, brightness, contrast".into(),
                },
                IntentParam {
                    name: "amount".into(),
                    param_type: "number".into(),
                    required: false,
                    description: "Filter intensity or amount".into(),
                },
            ],
        },
        AgnosIntent {
            name: "rasa.layer".into(),
            description: "Manage layers: add, remove, rename, adjust".into(),
            utterances: vec![
                "add a new layer".into(),
                "add layer called {name}".into(),
                "delete this layer".into(),
                "remove layer {name}".into(),
                "rename layer to {name}".into(),
                "set opacity to {opacity}".into(),
                "hide this layer".into(),
                "show this layer".into(),
                "duplicate this layer".into(),
                "merge down".into(),
            ],
            parameters: vec![
                IntentParam {
                    name: "action".into(),
                    param_type: "string".into(),
                    required: true,
                    description:
                        "Layer action: add, remove, rename, opacity, hide, show, duplicate, merge"
                            .into(),
                },
                IntentParam {
                    name: "name".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Layer name".into(),
                },
                IntentParam {
                    name: "opacity".into(),
                    param_type: "number".into(),
                    required: false,
                    description: "Opacity value 0-100".into(),
                },
            ],
        },
        AgnosIntent {
            name: "rasa.export".into(),
            description: "Export or save the current document".into(),
            utterances: vec![
                "export as {format}".into(),
                "save as {file}".into(),
                "export to {file}".into(),
                "save this image".into(),
                "export as PNG".into(),
                "save as JPEG with quality {quality}".into(),
            ],
            parameters: vec![
                IntentParam {
                    name: "file".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Output file path".into(),
                },
                IntentParam {
                    name: "format".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Export format: png, jpeg, webp, tiff".into(),
                },
                IntentParam {
                    name: "quality".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "JPEG quality 1-100".into(),
                },
            ],
        },
        AgnosIntent {
            name: "rasa.ai".into(),
            description: "Apply AI operations: inpaint, upscale, remove background, generate"
                .into(),
            utterances: vec![
                "remove the background".into(),
                "upscale this image".into(),
                "upscale {scale} times".into(),
                "inpaint the selection".into(),
                "fill selection with {prompt}".into(),
                "generate an image of {prompt}".into(),
                "AI select the subject".into(),
            ],
            parameters: vec![
                IntentParam {
                    name: "operation".into(),
                    param_type: "string".into(),
                    required: true,
                    description:
                        "AI operation: inpaint, upscale, remove_background, generate, select".into(),
                },
                IntentParam {
                    name: "prompt".into(),
                    param_type: "string".into(),
                    required: false,
                    description: "Text prompt for generation or inpainting".into(),
                },
                IntentParam {
                    name: "scale".into(),
                    param_type: "integer".into(),
                    required: false,
                    description: "Upscale factor (2 or 4)".into(),
                },
            ],
        },
    ]
}

/// Generate the `.agnos-agent` bundle as a JSON value.
pub fn agnos_agent_bundle() -> serde_json::Value {
    serde_json::json!({
        "name": "rasa",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "AI-powered image editor with voice control via AGNOS",
        "intents": list_intents(),
        "mcp": {
            "transport": "stdio",
            "command": "rasa-mcp"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_intents() {
        let intents = list_intents();
        assert_eq!(intents.len(), 5);
    }

    #[test]
    fn intent_names() {
        let names: Vec<String> = list_intents().iter().map(|i| i.name.clone()).collect();
        assert!(names.contains(&"rasa.open".to_string()));
        assert!(names.contains(&"rasa.filter".to_string()));
        assert!(names.contains(&"rasa.layer".to_string()));
        assert!(names.contains(&"rasa.export".to_string()));
        assert!(names.contains(&"rasa.ai".to_string()));
    }

    #[test]
    fn intents_have_utterances() {
        for intent in list_intents() {
            assert!(
                !intent.utterances.is_empty(),
                "{} has no utterances",
                intent.name
            );
        }
    }

    #[test]
    fn intents_have_parameters() {
        for intent in list_intents() {
            assert!(
                !intent.parameters.is_empty(),
                "{} has no params",
                intent.name
            );
        }
    }

    #[test]
    fn agnos_bundle_valid() {
        let bundle = agnos_agent_bundle();
        assert_eq!(bundle["name"], "rasa");
        assert_eq!(bundle["intents"].as_array().unwrap().len(), 5);
        assert_eq!(bundle["mcp"]["transport"], "stdio");
    }

    #[test]
    fn intents_serialize() {
        let intents = list_intents();
        let json = serde_json::to_string(&intents).unwrap();
        assert!(json.contains("rasa.open"));
        assert!(json.contains("utterances"));
    }
}
