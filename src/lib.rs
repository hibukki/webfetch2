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
    pub url: String,
    #[serde(default)]
    pub use_cache: bool,
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

    #[tool(description = "Download content from a URL and save it to .tempwebfetch/ directory.")]
    pub async fn fetch(
        &self,
        Parameters(FetchRequest { url, use_cache }): Parameters<FetchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let parsed_url = url::Url::parse(&url).map_err(|e| {
            McpError::invalid_params(
                format!("Invalid URL: {e}"),
                Some(json!({"url": url, "error": e.to_string()})),
            )
        })?;

        let temp_dir = PathBuf::from(".tempwebfetch");
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to create .tempwebfetch directory: {e}"),
                Some(json!({"error": e.to_string()})),
            )
        })?;

        let filename = Self::generate_filename(&parsed_url);
        let file_path = temp_dir.join(&filename);

        if use_cache && file_path.exists() {
            let metadata = tokio::fs::metadata(&file_path).await.ok();
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Cached: {} ({} bytes)",
                file_path.display(),
                size
            ))]));
        }

        let response = reqwest::get(url.clone()).await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to fetch URL: {e}"),
                Some(json!({"url": url, "error": e.to_string()})),
            )
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("HTTP {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown")),
                Some(json!({"url": url, "status": status.as_u16()})),
            ));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let bytes = response.bytes().await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to read response body: {e}"),
                Some(json!({"url": url, "error": e.to_string()})),
            )
        })?;

        let size = bytes.len();

        tokio::fs::write(&file_path, &bytes).await.map_err(|e| {
            McpError::internal_error(
                format!("Failed to write file: {e}"),
                Some(json!({"path": file_path.display().to_string(), "error": e.to_string()})),
            )
        })?;

        let type_info = content_type.map(|t| format!(", {t}")).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Downloaded: {} ({} bytes{})",
            file_path.display(),
            size,
            type_info
        ))]))
    }

    fn generate_filename(url: &url::Url) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.as_str().hash(&mut hasher);
        let hash_suffix = format!("{:x}", hasher.finish() & 0xFFFF); // short 4-char suffix

        let last_segment = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|s| !s.is_empty());

        match last_segment {
            Some(name) if name.contains('.') => {
                let sanitized: String = name
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
                    .collect();
                format!("{sanitized}_{hash_suffix}")
            }
            _ => format!("{hash_suffix}.html"),
        }
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
