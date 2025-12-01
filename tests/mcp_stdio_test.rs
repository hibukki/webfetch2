use std::process::{Command, Stdio, Child, ChildStdin, ChildStdout};
use std::io::{BufRead, BufReader, Write};
use serde_json::{json, Value};

/// Helper struct to manage MCP server subprocess communication
struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpClient {
    /// Spawn the MCP server binary
    fn spawn() -> Self {
        let mut child = Command::new("cargo")
            .args(["run", "--release", "--quiet"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Suppress stderr logs
            .spawn()
            .expect("Failed to spawn MCP server");

        let stdin = child.stdin.take().expect("Failed to get stdin");
        let stdout = child.stdout.take().expect("Failed to get stdout");
        let stdout = BufReader::new(stdout);

        McpClient {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    /// Send a JSON-RPC notification (no response expected)
    fn send_notification(&mut self, method: &str, params: Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let notification_str = serde_json::to_string(&notification).expect("Failed to serialize notification");
        writeln!(self.stdin, "{notification_str}").expect("Failed to write to stdin");
        self.stdin.flush().expect("Failed to flush stdin");
    }

    /// Send a JSON-RPC request and return the response
    fn send_request(&mut self, method: &str, params: Value) -> Value {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params
        });

        self.next_id += 1;

        // Write request as single line
        let request_str = serde_json::to_string(&request).expect("Failed to serialize request");
        writeln!(self.stdin, "{request_str}").expect("Failed to write to stdin");
        self.stdin.flush().expect("Failed to flush stdin");

        // Read response
        let mut response_line = String::new();
        self.stdout
            .read_line(&mut response_line)
            .expect("Failed to read response");

        serde_json::from_str(&response_line).expect("Failed to parse response JSON")
    }

    /// Initialize the MCP server
    fn initialize(&mut self) -> Value {
        let response = self.send_request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }),
        );

        // Send the initialized notification to complete the handshake
        self.send_notification("notifications/initialized", json!({}));

        response
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn test_initialize() {
    let mut client = McpClient::spawn();
    let response = client.initialize();

    // Verify it's a valid response
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_number());
    assert!(response["result"].is_object());

    // Verify protocol version
    let result = &response["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");

    // Verify capabilities include tools
    assert!(result["capabilities"]["tools"].is_object());

    // Verify server info exists
    assert!(result["serverInfo"].is_object());
}

#[test]
fn test_tools_list() {
    let mut client = McpClient::spawn();
    client.initialize();

    let response = client.send_request("tools/list", json!({}));

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_object());

    // Verify tools array exists and contains "fetch"
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    assert_eq!(tools.len(), 1, "Should have exactly one tool");

    let fetch_tool = &tools[0];
    assert_eq!(fetch_tool["name"], "fetch");
    assert!(fetch_tool["description"].is_string());
    assert!(fetch_tool["inputSchema"].is_object());
}

#[test]
fn test_fetch_success() {
    let mut client = McpClient::spawn();
    client.initialize();

    // Clean up any existing .tempwebfetch directory
    let _ = std::fs::remove_dir_all(".tempwebfetch");

    let response = client.send_request(
        "tools/call",
        json!({
            "name": "fetch",
            "arguments": {
                "url": "https://example.com"
            }
        }),
    );

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_object());

    // Verify content exists
    let content = response["result"]["content"]
        .as_array()
        .expect("content should be an array");

    assert!(!content.is_empty(), "Content should not be empty");

    let text = content[0]["text"]
        .as_str()
        .expect("First content item should have text");

    assert!(text.starts_with("Downloaded:"));
    assert!(text.contains(".tempwebfetch"));

    // Verify isError is false or not present
    assert!(
        response["result"]["isError"].is_null() || response["result"]["isError"] == false,
        "isError should be false or null"
    );
}

#[test]
fn test_fetch_invalid_url() {
    let mut client = McpClient::spawn();
    client.initialize();

    let response = client.send_request(
        "tools/call",
        json!({
            "name": "fetch",
            "arguments": {
                "url": "not-a-valid-url"
            }
        }),
    );

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");

    // Should have an error field (JSON-RPC error response)
    assert!(
        response["error"].is_object(),
        "Invalid URL should return JSON-RPC error"
    );

    let error = &response["error"];
    assert!(error["code"].is_number());
    assert!(error["message"].is_string());

    // Error message should mention invalid URL
    let message = error["message"].as_str().unwrap();
    assert!(
        message.to_lowercase().contains("invalid url") || message.to_lowercase().contains("url"),
        "Error message should mention URL issue: {message}"
    );
}

#[test]
fn test_fetch_http_error() {
    let mut client = McpClient::spawn();
    client.initialize();

    let response = client.send_request(
        "tools/call",
        json!({
            "name": "fetch",
            "arguments": {
                "url": "https://httpbin.org/status/404"
            }
        }),
    );

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");

    // Should have an error
    assert!(
        response["error"].is_object(),
        "HTTP 404 should return an error"
    );

    let error = &response["error"];
    let message = error["message"].as_str().unwrap();

    // Error message should mention HTTP error (status code may vary if httpbin.org is having issues)
    assert!(
        message.contains("HTTP") || message.to_lowercase().contains("error"),
        "Error message should mention HTTP error: {message}"
    );
}

#[test]
fn test_multiple_requests() {
    let mut client = McpClient::spawn();
    client.initialize();

    // Clean up
    let _ = std::fs::remove_dir_all(".tempwebfetch");

    // Make multiple requests in sequence
    for i in 0..3 {
        let response = client.send_request(
            "tools/call",
            json!({
                "name": "fetch",
                "arguments": {
                    "url": format!("https://example.com/test{}.html", i)
                }
            }),
        );

        assert_eq!(response["jsonrpc"], "2.0");
        assert!(
            response["result"].is_object() || response["error"].is_object(),
            "Each request should get a response"
        );
    }
}
