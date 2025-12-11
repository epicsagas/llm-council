use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub struct McpServer;

impl McpServer {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        let mut buffer = String::new();

        loop {
            buffer.clear();
            let bytes_read = reader.read_line(&mut buffer).await?;

            if bytes_read == 0 {
                break; // EOF
            }

            let line = buffer.trim();
            if line.is_empty() {
                continue;
            }

            match self.handle_request(line).await {
                Ok(Some(response)) => {
                    let response_json = serde_json::to_string(&response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
                Ok(None) => {
                    // Notification (no id) or intentionally suppressed response
                }
                Err(e) => {
                    // For malformed input (e.g., non-JSON lines), log and skip without emitting a JSON response
                    eprintln!("Error handling request (ignored): {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_request(&self, line: &str) -> Result<Option<McpResponse>> {
        let request: McpRequest = serde_json::from_str(line)
            .context("Failed to parse JSON-RPC request")?;

        let mut request_id = request.id.clone();
        let is_notification = match request_id.as_ref() {
            None => true,
            Some(v) if v.is_null() => true,
            Some(v) if v.is_boolean() => true,
            Some(v) if v.is_array() => true,
            Some(v) if v.is_object() => true,
            _ => false,
        };
        if is_notification && request_id.is_some() {
            eprintln!(
                "Invalid JSON-RPC id (ignored, treated as notification): {:?}",
                request_id
            );
            request_id = None;
        }
        let response_id = if is_notification { None } else { request_id.clone() };

        let result = match request.method.as_str() {
            "initialize" => {
                Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "mcp-council",
                        "version": "0.1.0"
                    }
                }))
            }
            "tools/list" => {
                Some(json!({
                    "tools": [
                        {
                            "name": "council.peer_review",
                            "description": "Stage2: Read Stage1 JSON files and generate peer review using local LLM CLI",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "title": {
                                        "type": "string",
                                        "description": "Conversation title/directory name"
                                    },
                                    "engine": {
                                        "type": "string",
                                        "description": "LLM model/engine (examples: sonnet, gemini, gpt, grok)",
                                        "default": "claude"
                                    },
                                    "self_model": {
                                        "type": "string",
                                        "description": "Model name to exclude from peer review (its own response)"
                                    }
                                },
                                "required": ["title"]
                            }
                        },
                        {
                            "name": "council.finalize",
                            "description": "Stage3: Read Stage1 and Stage2 JSON files and generate final answer using local LLM CLI",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "title": {
                                        "type": "string",
                                        "description": "Conversation title/directory name"
                                    },
                                    "engine": {
                                        "type": "string",
                                        "description": "LLM model/engine (examples: sonnet, gemini, gpt, grok)",
                                        "default": "claude"
                                    }
                                },
                                "required": ["title"]
                            }
                        }
                    ]
                }))
            }
            "tools/call" => {
                let params = request.params.context("Missing params")?;
                let tool_name = params["name"]
                    .as_str()
                    .context("Missing tool name")?;
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                match tool_name {
                    "council.peer_review" => {
                        match crate::tools::peer_review::handle_peer_review(arguments).await {
                            Ok(result) => Some(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string(&result)?
                                    }
                                ]
                            })),
                            Err(e) => {
                                if is_notification {
                                    eprintln!("Peer review failed for notification: {}", e);
                                    return Ok(None);
                                }
                                return Ok(Some(McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: response_id.clone(),
                                    result: None,
                                    error: Some(McpError {
                                        code: -32603,
                                        message: format!("Peer review failed: {}", e),
                                        data: None,
                                    }),
                                }));
                            }
                        }
                    }
                    "council.finalize" => {
                        match crate::tools::finalize::handle_finalize(arguments).await {
                            Ok(result) => Some(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string(&result)?
                                    }
                                ]
                            })),
                            Err(e) => {
                                if is_notification {
                                    eprintln!("Finalize failed for notification: {}", e);
                                    return Ok(None);
                                }
                                return Ok(Some(McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: response_id.clone(),
                                    result: None,
                                    error: Some(McpError {
                                        code: -32603,
                                        message: format!("Finalize failed: {}", e),
                                        data: None,
                                    }),
                                }));
                            }
                        }
                    }
                    _ => {
                        if is_notification {
                            return Ok(None);
                        }
                        return Ok(Some(McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: response_id.clone(),
                            result: None,
                            error: Some(McpError {
                                code: -32601,
                                message: format!("Unknown tool: {}", tool_name),
                                data: None,
                            }),
                        }));
                    }
                }
            }
            _ => {
                if is_notification {
                    return Ok(None);
                }
                return Ok(Some(McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: response_id.clone(),
                    result: None,
                    error: Some(McpError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                }));
            }
        };

        if is_notification {
            Ok(None)
        } else {
            Ok(Some(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: response_id,
                result,
                error: None,
            }))
        }
    }
}

