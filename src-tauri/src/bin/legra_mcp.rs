#[path = "../agent_bridge.rs"]
#[allow(dead_code)]
mod agent_bridge;

use agent_bridge::{
    enqueue_import, import_status, list_categories, resolve_paper, search_library, DownloadPolicy,
    EnqueueImportInput,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const SERVER_NAME: &str = "legra-mcp";
const PROTOCOL_VERSION: &str = "2025-06-18";
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Default)]
struct ServerState {
    client_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvePaperArguments {
    identifier: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchLibraryArguments {
    query: String,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyArguments {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportPaperArguments {
    identifier: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    download_policy: DownloadPolicy,
    #[serde(default)]
    idempotency_key: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportStatusArguments {
    job_id: String,
}

fn default_search_limit() -> usize {
    20
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message.into(),
    });
    if let Some(data) = data {
        error["data"] = data;
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error,
    })
}

fn tool_result(text: impl Into<String>, structured_content: Value, is_error: bool) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text.into(),
        }],
        "structuredContent": structured_content,
        "isError": is_error,
    })
}

fn tool_error(error: impl Into<String>) -> Value {
    let error = error.into();
    tool_result(error.clone(), json!({ "error": error }), true)
}

fn annotations(title: &str, read_only: bool) -> Value {
    json!({
        "title": title,
        "readOnlyHint": read_only,
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": false,
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "resolve_paper",
            "title": "Resolve paper identifier",
            "description": "Normalize a DOI, Nature article URL, or arXiv identifier and check whether it is already present in the local Legra library. This does not access the network.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 512,
                        "description": "A DOI, doi.org URL, nature.com/articles URL, arXiv ID, or arxiv.org abs/pdf URL."
                    }
                },
                "required": ["identifier"],
                "additionalProperties": false
            },
            "annotations": annotations("Resolve paper identifier", true)
        },
        {
            "name": "search_library",
            "title": "Search Legra library",
            "description": "Search bibliographic fields in the active local Legra library. Filesystem paths and private settings are not returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 300
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "default": 20
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            },
            "annotations": annotations("Search Legra library", true)
        },
        {
            "name": "list_categories",
            "title": "List Legra categories",
            "description": "List categories in the active local Legra library.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            },
            "annotations": annotations("List Legra categories", true)
        },
        {
            "name": "import_paper",
            "title": "Import paper into Legra",
            "description": "Queue a DOI, Nature article URL, or arXiv paper for Legra to import. Other publisher URLs and local PDF paths are deliberately rejected. open_access_only is the default; metadata_only never requests a PDF download.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 512,
                        "description": "A DOI, doi.org URL, nature.com/articles URL, arXiv ID, or arxiv.org abs/pdf URL."
                    },
                    "category": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 300
                    },
                    "tags": {
                        "type": "array",
                        "maxItems": 64,
                        "items": { "type": "string", "minLength": 1, "maxLength": 100 },
                        "default": []
                    },
                    "download_policy": {
                        "type": "string",
                        "enum": ["open_access_only", "metadata_only"],
                        "default": "open_access_only"
                    },
                    "idempotency_key": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 256,
                        "description": "Stable retry key. If omitted, Legra derives one from the canonical arguments and MCP client name."
                    }
                },
                "required": ["identifier"],
                "additionalProperties": false
            },
            "annotations": annotations("Import paper into Legra", false)
        },
        {
            "name": "get_import_status",
            "title": "Get Legra import status",
            "description": "Read the pending, processing, completed, or failed state and structured result of an agent import job.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job_id": {
                        "type": "string",
                        "pattern": "^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$"
                    }
                },
                "required": ["job_id"],
                "additionalProperties": false
            },
            "annotations": annotations("Get Legra import status", true)
        }
    ])
}

fn parse_arguments<T: for<'de> Deserialize<'de>>(arguments: Value) -> Result<T, String> {
    serde_json::from_value(arguments).map_err(|error| format!("Invalid tool arguments: {error}"))
}

fn call_tool(name: &str, arguments: Value, state: &ServerState) -> Value {
    let result = match name {
        "resolve_paper" => {
            let arguments: ResolvePaperArguments = match parse_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(error) => return tool_error(error),
            };
            resolve_paper(&arguments.identifier).map(|resolution| {
                let text = if resolution.found {
                    format!(
                        "{} {} is already present in Legra.",
                        match resolution.identifier.kind {
                            agent_bridge::IdentifierKind::Doi => "DOI",
                            agent_bridge::IdentifierKind::Arxiv => "arXiv",
                        },
                        resolution.identifier.value
                    )
                } else {
                    format!(
                        "{} is valid and is not currently registered in Legra.",
                        resolution.identifier.value
                    )
                };
                (
                    text,
                    serde_json::to_value(resolution).unwrap_or(Value::Null),
                )
            })
        }
        "search_library" => {
            let arguments: SearchLibraryArguments = match parse_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(error) => return tool_error(error),
            };
            if !(1..=100).contains(&arguments.limit) {
                return tool_error("Search limit must be between 1 and 100.");
            }
            search_library(&arguments.query, arguments.limit).map(|search| {
                let text = format!("Found {} matching paper(s) in Legra.", search.count);
                (text, serde_json::to_value(search).unwrap_or(Value::Null))
            })
        }
        "list_categories" => {
            if let Err(error) = parse_arguments::<EmptyArguments>(arguments) {
                return tool_error(error);
            }
            list_categories().map(|listing| {
                let text = format!("Found {} Legra categor(ies).", listing.count);
                (text, serde_json::to_value(listing).unwrap_or(Value::Null))
            })
        }
        "import_paper" => {
            let arguments: ImportPaperArguments = match parse_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(error) => return tool_error(error),
            };
            enqueue_import(EnqueueImportInput {
                identifier: arguments.identifier,
                category: arguments.category,
                tags: arguments.tags,
                download_policy: arguments.download_policy,
                idempotency_key: arguments.idempotency_key,
                origin: if state.client_name.is_empty() {
                    "unknown-mcp-client".to_string()
                } else {
                    state.client_name.clone()
                },
            })
            .map(|queued| {
                let text = if queued.already_exists {
                    format!(
                        "Legra import job {} already exists ({:?}).",
                        queued.request.job_id, queued.state
                    )
                } else {
                    format!("Queued Legra import job {}.", queued.request.job_id)
                };
                (text, serde_json::to_value(queued).unwrap_or(Value::Null))
            })
        }
        "get_import_status" => {
            let arguments: ImportStatusArguments = match parse_arguments(arguments) {
                Ok(arguments) => arguments,
                Err(error) => return tool_error(error),
            };
            import_status(&arguments.job_id).map(|status| {
                let text = format!("Legra import job {} is {:?}.", status.job_id, status.state);
                (text, serde_json::to_value(status).unwrap_or(Value::Null))
            })
        }
        _ => return tool_error(format!("Unknown Legra tool: {name}")),
    };

    match result {
        Ok((text, structured)) => tool_result(text, structured, false),
        Err(error) => tool_error(error),
    }
}

fn initialize_result(params: &Value, state: &mut ServerState) -> Value {
    state.client_name = params
        .get("clientInfo")
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown-mcp-client")
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect();
    if state.client_name.is_empty() {
        state.client_name = "unknown-mcp-client".to_string();
    }
    let requested_protocol_version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .filter(|version| !version.is_empty())
        .unwrap_or(PROTOCOL_VERSION);
    let protocol_version = if requested_protocol_version == PROTOCOL_VERSION {
        requested_protocol_version
    } else {
        PROTOCOL_VERSION
    };
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Legra imports papers by canonical DOI, Nature article URL, or arXiv identifier. Use resolve_paper or search_library to check for duplicates, list_categories when a destination is needed, import_paper to enqueue, and get_import_status to observe completion. Never pass arbitrary download URLs or local file paths. open_access_only is the default download policy."
    })
}

fn handle_message(message: Value, state: &mut ServerState) -> Option<Value> {
    let object = match message.as_object() {
        Some(object) => object,
        None => {
            return Some(jsonrpc_error(
                Value::Null,
                -32600,
                "Invalid JSON-RPC request.",
                None,
            ))
        }
    };
    let has_id = object.contains_key("id");
    let id = object.get("id").cloned().unwrap_or(Value::Null);
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return has_id.then(|| jsonrpc_error(id, -32600, "jsonrpc must be exactly '2.0'.", None));
    }
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return has_id.then(|| jsonrpc_error(id, -32600, "method is required.", None));
    };
    let params = object.get("params").cloned().unwrap_or_else(|| json!({}));

    if !has_id {
        // MCP notifications, including notifications/initialized and cancellation,
        // never receive a JSON-RPC response.
        return None;
    }

    match method {
        "initialize" => Some(jsonrpc_result(id, initialize_result(&params, state))),
        "ping" => Some(jsonrpc_result(id, json!({}))),
        "tools/list" => Some(jsonrpc_result(id, json!({ "tools": tools() }))),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Some(jsonrpc_error(
                    id,
                    -32602,
                    "tools/call params.name is required.",
                    None,
                ));
            };
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Some(jsonrpc_result(id, call_tool(name, arguments, state)))
        }
        _ => Some(jsonrpc_error(
            id,
            -32601,
            format!("Method not found: {method}"),
            None,
        )),
    }
}

fn write_json_line(output: &mut impl Write, value: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *output, value).map_err(|error| error.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|_| output.flush())
        .map_err(|error| error.to_string())
}

enum InputLine {
    End,
    Message(Vec<u8>),
    TooLarge,
}

fn read_input_line(input: &mut impl BufRead) -> io::Result<InputLine> {
    let mut message = Vec::new();
    let mut too_large = false;
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            if too_large {
                return Ok(InputLine::TooLarge);
            }
            return if message.is_empty() {
                Ok(InputLine::End)
            } else {
                Ok(InputLine::Message(message))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |position| position + 1);
        if !too_large {
            if message.len().saturating_add(consumed) > MAX_MESSAGE_BYTES {
                too_large = true;
                message.clear();
            } else {
                message.extend_from_slice(&available[..consumed]);
            }
        }
        input.consume(consumed);
        if newline.is_some() {
            if too_large {
                return Ok(InputLine::TooLarge);
            }
            while message
                .last()
                .is_some_and(|byte| matches!(*byte, b'\n' | b'\r'))
            {
                message.pop();
            }
            return Ok(InputLine::Message(message));
        }
    }
}

fn run_server() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut state = ServerState::default();
    loop {
        let line = read_input_line(&mut input)
            .map_err(|error| format!("Could not read MCP input: {error}"))?;
        let line = match line {
            InputLine::End => break,
            InputLine::TooLarge => {
                write_json_line(
                    &mut output,
                    &jsonrpc_error(
                        Value::Null,
                        -32600,
                        format!("MCP message exceeds the {MAX_MESSAGE_BYTES}-byte limit."),
                        None,
                    ),
                )?;
                continue;
            }
            InputLine::Message(line) => line,
        };
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let response = match serde_json::from_slice::<Value>(&line) {
            Ok(message) => handle_message(message, &mut state),
            Err(error) => Some(jsonrpc_error(
                Value::Null,
                -32700,
                "Parse error.",
                Some(json!({ "detail": error.to_string() })),
            )),
        };
        if let Some(response) = response {
            write_json_line(&mut output, &response)?;
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        "legra-mcp {}\n\nUsage: legra_mcp [--help | --version]\nWithout arguments, runs the Legra stdio MCP server.",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => {
            if let Err(error) = run_server() {
                eprintln!("legra-mcp: {error}");
                std::process::exit(1);
            }
        }
        [argument] if matches!(argument.as_str(), "--version" | "-V") => {
            println!("legra-mcp {}", env!("CARGO_PKG_VERSION"));
        }
        [argument] if matches!(argument.as_str(), "--help" | "-h") => print_help(),
        _ => {
            eprintln!("legra-mcp: unsupported arguments; use --help");
            std::process::exit(2);
        }
    }
}
