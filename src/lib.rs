use anyhow::Result;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FetchRequest {
    /// The URL to fetch content from
    pub url: String,
}

#[derive(Clone)]
pub struct WebFetch {
    tool_router: ToolRouter<WebFetch>,
}

impl Default for WebFetch {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl WebFetch {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Download content from a URL and save it to .tempwebfetch/ directory. Returns the local file path where the content was saved.")]
    pub async fn fetch(
        &self,
        Parameters(FetchRequest { url }): Parameters<FetchRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Validate URL
        let parsed_url = url::Url::parse(&url).map_err(|e| {
            McpError::invalid_params(
                format!("Invalid URL: {}. Please provide a valid HTTP/HTTPS URL (e.g., https://example.com)", e),
                Some(json!({"url": url, "error": e.to_string()})),
            )
        })?;

        // Create .tempwebfetch directory if it doesn't exist
        let temp_dir = PathBuf::from(".tempwebfetch");
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to create .tempwebfetch directory: {}. Check file permissions.", e),
                Some(json!({"error": e.to_string()})),
            )
        })?;

        // Generate filename from URL
        let filename = Self::generate_filename(&parsed_url);
        let file_path = temp_dir.join(&filename);

        // Download content
        let response = reqwest::get(url.clone()).await.map_err(|e| {
            let error_str = e.to_string();
            if e.is_timeout() {
                McpError::internal_error(
                    "Request timed out. Check your network connection and try again.",
                    Some(json!({"url": url, "error": error_str})),
                )
            } else if e.is_connect() {
                McpError::internal_error(
                    "Failed to connect to the server. Check your network connection and that the URL is accessible.",
                    Some(json!({"url": url, "error": error_str})),
                )
            } else {
                McpError::internal_error(
                    format!("Failed to fetch URL: {}", error_str),
                    Some(json!({"url": url, "error": error_str})),
                )
            }
        })?;

        // Check HTTP status
        let status = response.status();
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("HTTP error {}: {}.", status.as_u16(), status.canonical_reason().unwrap_or("Unknown")),
                Some(json!({"url": url, "status": status.as_u16()})),
            ));
        }

        // Capture Content-Type header before consuming response
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Get response bytes
        let bytes = response.bytes().await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to read response body: {}", e),
                Some(json!({"url": url, "error": e.to_string()})),
            )
        })?;

        // Collect metadata
        let file_size = bytes.len();

        // Write to file
        tokio::fs::write(&file_path, &bytes).await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to write file: {}. Check disk space and file permissions.", e),
                Some(json!({"path": file_path.display().to_string(), "error": e.to_string()})),
            )
        })?;

        // Format response with metadata
        let response_text = Self::format_success_message(&file_path, file_size, content_type, &bytes);

        // Return relative path with metadata
        Ok(CallToolResult::success(vec![Content::text(response_text)]))
    }

    pub fn format_success_message(
        file_path: &std::path::Path,
        file_size: usize,
        content_type: Option<String>,
        bytes: &[u8],
    ) -> String {
        // Detect file type from magic bytes
        let detected_type = infer::get(bytes);
        let magic_type = detected_type
            .map(|t| t.mime_type().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Format metadata response
        let mut metadata_parts = vec![
            format!("File: {}", file_path.display()),
            format!("Size: {} bytes", file_size),
        ];

        if let Some(ct) = content_type {
            metadata_parts.push(format!("Content-Type: {}", ct));
        }

        metadata_parts.push(format!("Detected type: {}", magic_type));

        format!(
            "Content downloaded successfully\n\n{}",
            metadata_parts.join("\n")
        )
    }

    fn generate_filename(url: &url::Url) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.as_str().hash(&mut hasher);
        let hash = hasher.finish();

        // Try to get file extension from URL path
        let extension = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .and_then(|last| {
                if last.contains('.') {
                    last.split('.').next_back()
                } else {
                    None
                }
            })
            .unwrap_or("html");

        format!("{:x}.{}", hash, extension)
    }
}

#[tool_handler]
impl ServerHandler for WebFetch {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides a tool to download web content. \
                Use the 'fetch' tool with a URL to download content to the .tempwebfetch/ directory."
                    .to_string(),
            ),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}
