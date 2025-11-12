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

        // Get response bytes
        let bytes = response.bytes().await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to read response body: {}", e),
                Some(json!({"url": url, "error": e.to_string()})),
            )
        })?;

        // Write to file
        tokio::fs::write(&file_path, bytes).await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to write file: {}. Check disk space and file permissions.", e),
                Some(json!({"path": file_path.display().to_string(), "error": e.to_string()})),
            )
        })?;

        // Return relative path
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Content downloaded successfully to: {}",
            file_path.display()
        ))]))
    }

    fn generate_filename(url: &url::Url) -> String {
        use sha2::{Sha256, Digest};

        // Use SHA-256 for stable hashing across runs and Rust versions
        let mut hasher = Sha256::new();
        hasher.update(url.as_str().as_bytes());
        let hash = hasher.finalize();

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

        // Use first 16 hex characters of SHA-256 hash to keep filenames reasonable
        format!("{:x}.{}", &hash[..8].iter().fold(0u64, |acc, &b| (acc << 8) | b as u64), extension)
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
