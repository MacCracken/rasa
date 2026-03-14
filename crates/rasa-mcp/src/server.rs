use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::state::SessionState;
use crate::tools;

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

/// MCP server info returned during initialization.
pub fn server_info() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "rasa-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

/// Handle a single JSON-RPC request and return a response.
pub fn handle_request(state: &SessionState, request: &JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(server_info()),
            error: None,
        },
        "notifications/initialized" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({})),
            error: None,
        },
        "tools/list" => {
            let tool_defs = tools::list_tools();
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: Some(json!({ "tools": tool_defs })),
                error: None,
            }
        }
        "tools/call" => {
            let tool_name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));

            match tools::call_tool(state, tool_name, &arguments) {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                        }]
                    })),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": e
                        }],
                        "isError": true
                    })),
                    error: None,
                },
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("method not found: {}", request.method),
            }),
        },
    }
}

/// Run the MCP server on stdio (blocking).
pub fn run_stdio() {
    let state = SessionState::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("parse error: {e}")
                    }
                });
                let _ = writeln!(stdout, "{}", serde_json::to_string(&err_response).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        // JSON-RPC notifications (no id) must not receive a response
        if request.id.is_none() {
            // Still process the request for side effects
            let _ = handle_request(&state, &request);
            continue;
        }

        let response = handle_request(&state, &request);
        let response_json = serde_json::to_string(&response).unwrap();
        let _ = writeln!(stdout, "{response_json}");
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_initialize() {
        let state = SessionState::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: json!({}),
        };
        let resp = handle_request(&state, &req);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert!(
            result["serverInfo"]["name"]
                .as_str()
                .unwrap()
                .contains("rasa")
        );
    }

    #[test]
    fn handle_tools_list() {
        let state = SessionState::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "tools/list".into(),
            params: json!({}),
        };
        let resp = handle_request(&state, &req);
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().len();
        assert_eq!(tools, 5);
    }

    #[test]
    fn handle_tools_call() {
        let state = SessionState::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "tools/call".into(),
            params: json!({
                "name": "rasa_open_image",
                "arguments": {
                    "name": "MCP Test",
                    "width": 64,
                    "height": 64,
                }
            }),
        };
        let resp = handle_request(&state, &req);
        assert!(resp.error.is_none());
        let content = &resp.result.unwrap()["content"][0]["text"];
        let text = content.as_str().unwrap();
        assert!(text.contains("MCP Test"));
    }

    #[test]
    fn handle_unknown_method() {
        let state = SessionState::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "bogus/method".into(),
            params: json!({}),
        };
        let resp = handle_request(&state, &req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[test]
    fn handle_tool_error() {
        let state = SessionState::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "tools/call".into(),
            params: json!({
                "name": "rasa_get_document",
                "arguments": {
                    "document_id": "00000000-0000-0000-0000-000000000000"
                }
            }),
        };
        let resp = handle_request(&state, &req);
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], true);
    }

    #[test]
    fn handle_notifications_initialized() {
        let state = SessionState::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "notifications/initialized".into(),
            params: json!({}),
        };
        let resp = handle_request(&state, &req);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn server_info_has_version() {
        let info = server_info();
        assert!(
            info["serverInfo"]["name"]
                .as_str()
                .unwrap()
                .contains("rasa")
        );
        assert!(info["protocolVersion"].is_string());
        assert!(info["capabilities"]["tools"].is_object());
    }

    #[test]
    fn response_serializes_without_null_fields() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: json!(1),
            result: Some(json!({"ok": true})),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn error_response_serializes_without_result() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: json!(1),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "bad".into(),
            }),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"result\""));
        assert!(json.contains("-32600"));
    }
}
