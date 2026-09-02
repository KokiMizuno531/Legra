use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "legra-{label}-{}-{unique}-{counter}",
        std::process::id()
    ))
}

struct McpProcess {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl McpProcess {
    fn spawn(setting_dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_legra_mcp"))
            .env("LEGRA_SETTING_DIR", setting_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input: Some(input),
            output,
        }
    }

    fn send(&mut self, message: Value) {
        let input = self.input.as_mut().unwrap();
        serde_json::to_writer(&mut *input, &message).unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
    }

    fn send_raw(&mut self, message: &[u8]) {
        let input = self.input.as_mut().unwrap();
        input.write_all(message).unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
    }

    fn response(&mut self) -> Value {
        let mut line = String::new();
        assert!(self.output.read_line(&mut line).unwrap() > 0);
        serde_json::from_str(&line).unwrap_or_else(|error| {
            panic!("MCP stdout contained a non-JSON-RPC line: {line:?}: {error}")
        })
    }

    fn request(&mut self, message: Value) -> Value {
        let expected_id = message.get("id").cloned().unwrap();
        self.send(message);
        let response = self.response();
        assert_eq!(response.get("jsonrpc").and_then(Value::as_str), Some("2.0"));
        assert_eq!(response.get("id"), Some(&expected_id));
        response
    }

    fn finish(mut self) -> String {
        drop(self.input.take());
        assert!(self.child.wait().unwrap().success());
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        stderr
    }
}

fn tool_call(id: u64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments
        }
    })
}

#[test]
fn stdio_mcp_initializes_lists_tools_and_enqueues_idempotently() {
    let setting_dir = unique_temp_dir("mcp-protocol");
    fs::create_dir_all(&setting_dir).unwrap();
    fs::write(
        setting_dir.join("app-data.json"),
        json!({
            "papers": [{
                "id": "paper-existing",
                "title": "Existing Quantum Paper",
                "authors": ["A. Researcher"],
                "year": 2024,
                "doi": "10.1234/existing",
                "tags": ["quantum"],
                "folder_category": "Physics/Quantum",
                "pdf_path": "Physics/Quantum/existing.pdf"
            }],
            "settings": {
                "managed_directory": null,
                "workspace_root": null
            }
        })
        .to_string(),
    )
    .unwrap();

    let mut mcp = McpProcess::spawn(&setting_dir);
    let initialized = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "Codex Integration Test", "version": "1" }
        }
    }));
    assert_eq!(
        initialized
            .pointer("/result/serverInfo/name")
            .and_then(Value::as_str),
        Some("legra-mcp")
    );
    assert!(initialized
        .pointer("/result/instructions")
        .and_then(Value::as_str)
        .is_some_and(|value| value.contains("DOI or arXiv")));

    // A notification must produce no stdout. The next line read below must belong
    // to request id 2, which also verifies stdout remains JSON-RPC-only.
    mcp.send(json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    }));
    let listed = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));
    let tools = listed
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .unwrap();
    assert_eq!(tools.len(), 5);
    for tool in tools {
        let name = tool.get("name").and_then(Value::as_str).unwrap();
        let expected_read_only = name != "import_paper";
        assert_eq!(
            tool.pointer("/annotations/readOnlyHint")
                .and_then(Value::as_bool),
            Some(expected_read_only)
        );
        assert_eq!(
            tool.pointer("/annotations/destructiveHint")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            tool.pointer("/annotations/idempotentHint")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            tool.pointer("/annotations/openWorldHint")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            tool.pointer("/inputSchema/type").and_then(Value::as_str),
            Some("object")
        );
    }

    let resolved = mcp.request(tool_call(
        3,
        "resolve_paper",
        json!({ "identifier": "https://doi.org/10.1234/EXISTING" }),
    ));
    assert_eq!(
        resolved
            .pointer("/result/structuredContent/found")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(resolved
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .is_some());

    let imported = mcp.request(tool_call(
        4,
        "import_paper",
        json!({
            "identifier": "https://arxiv.org/pdf/2401.12345v2.pdf",
            "category": "Physics/Quantum",
            "tags": ["codex-found", "quantum"],
            "download_policy": "open_access_only",
            "idempotency_key": "integration-retry-key"
        }),
    ));
    assert_eq!(
        imported.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );
    let job_id = imported
        .pointer("/result/structuredContent/request/job_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    assert_eq!(
        imported
            .pointer("/result/structuredContent/request/origin")
            .and_then(Value::as_str),
        Some("Codex Integration Test")
    );
    assert_eq!(
        imported
            .pointer("/result/structuredContent/request/identifier/value")
            .and_then(Value::as_str),
        Some("2401.12345")
    );

    let pending_path = setting_dir
        .join("agent-imports")
        .join("pending")
        .join(format!("{job_id}.json"));
    let pending: Value = serde_json::from_slice(&fs::read(&pending_path).unwrap()).unwrap();
    assert_eq!(
        pending.get("schema_version").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        pending.get("idempotency_key").and_then(Value::as_str),
        Some("integration-retry-key")
    );

    let retried = mcp.request(tool_call(
        5,
        "import_paper",
        json!({
            "identifier": "2401.12345",
            "category": "Physics/Quantum",
            "tags": ["codex-found", "quantum"],
            "download_policy": "open_access_only",
            "idempotency_key": "integration-retry-key"
        }),
    ));
    assert_eq!(
        retried
            .pointer("/result/structuredContent/request/job_id")
            .and_then(Value::as_str),
        Some(job_id.as_str())
    );
    assert_eq!(
        retried
            .pointer("/result/structuredContent/already_exists")
            .and_then(Value::as_bool),
        Some(true)
    );
    let pending_after_retry: Value =
        serde_json::from_slice(&fs::read(&pending_path).unwrap()).unwrap();
    assert_eq!(pending_after_retry, pending);

    let conflicting_retry = mcp.request(tool_call(
        51,
        "import_paper",
        json!({
            "identifier": "2401.12345",
            "category": "A different value must not be silently accepted",
            "idempotency_key": "integration-retry-key"
        }),
    ));
    assert_eq!(
        conflicting_retry
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(conflicting_retry
        .pointer("/result/structuredContent/error")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("different import arguments")));
    let pending_count = fs::read_dir(pending_path.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .count();
    assert_eq!(pending_count, 1);

    let status = mcp.request(tool_call(
        6,
        "get_import_status",
        json!({ "job_id": job_id }),
    ));
    assert_eq!(
        status
            .pointer("/result/structuredContent/state")
            .and_then(Value::as_str),
        Some("pending")
    );

    let rejected = mcp.request(tool_call(
        7,
        "import_paper",
        json!({
            "identifier": "https://example.com/untrusted-paper.pdf",
            "pdf_path": "/tmp/untrusted.pdf"
        }),
    ));
    assert_eq!(
        rejected.pointer("/result/isError").and_then(Value::as_bool),
        Some(true)
    );
    assert!(rejected
        .pointer("/result/structuredContent/error")
        .and_then(Value::as_str)
        .is_some());

    let excessive_search = mcp.request(tool_call(
        8,
        "search_library",
        json!({ "query": "quantum", "limit": 101 }),
    ));
    assert_eq!(
        excessive_search
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(true)
    );

    let excessive_identifier = mcp.request(tool_call(
        9,
        "resolve_paper",
        json!({ "identifier": "x".repeat(513) }),
    ));
    assert_eq!(
        excessive_identifier
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(true)
    );

    let unsafe_job_id = mcp.request(tool_call(
        10,
        "get_import_status",
        json!({ "job_id": "../../app-data" }),
    ));
    assert_eq!(
        unsafe_job_id
            .pointer("/result/isError")
            .and_then(Value::as_bool),
        Some(true)
    );

    let stderr = mcp.finish();
    assert!(stderr.is_empty(), "unexpected MCP stderr: {stderr}");
    fs::remove_dir_all(setting_dir).unwrap();
}

#[test]
fn version_mode_is_machine_checkable() {
    let output = Command::new(env!("CARGO_BIN_EXE_legra_mcp"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("legra-mcp {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn stdio_mcp_negotiates_protocol_and_rejects_oversized_messages() {
    let setting_dir = unique_temp_dir("mcp-limits");
    fs::create_dir_all(&setting_dir).unwrap();
    let mut mcp = McpProcess::spawn(&setting_dir);

    let initialized = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2099-01-01",
            "capabilities": {},
            "clientInfo": { "name": "Protocol Test", "version": "1" }
        }
    }));
    assert_eq!(
        initialized
            .pointer("/result/protocolVersion")
            .and_then(Value::as_str),
        Some("2025-06-18")
    );

    mcp.send_raw(&vec![b'x'; 1024 * 1024 + 1]);
    let oversized = mcp.response();
    assert_eq!(
        oversized.pointer("/error/code").and_then(Value::as_i64),
        Some(-32600)
    );
    assert!(oversized
        .pointer("/error/message")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("byte limit")));

    let ping = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    }));
    assert!(ping.pointer("/result").is_some());

    let stderr = mcp.finish();
    assert!(stderr.is_empty(), "unexpected MCP stderr: {stderr}");
    fs::remove_dir_all(setting_dir).unwrap();
}
