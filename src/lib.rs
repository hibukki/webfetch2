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

    #[tool(description = "Download content from a URL and save it to .tempwebfetch/ directory. Also checks for /llms.txt on the domain.")]
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

        // Build llms file URLs for root and subpath
        let base_url = format!(
            "{}://{}",
            parsed_url.scheme(),
            parsed_url.host_str().unwrap_or("")
        );
        let host = parsed_url.host_str().unwrap_or("unknown");

        // Get parent directory path (e.g., "/docs/page.html" -> "/docs/")
        let path = parsed_url.path();
        let parent_path = if path.ends_with('/') {
            path.to_string()
        } else {
            path.rsplit_once('/').map(|(p, _)| format!("{p}/")).unwrap_or_else(|| "/".to_string())
        };

        // Build list of llms files to check (root + subpath if different)
        let mut llms_files = vec![
            ("llms.txt", format!("{base_url}/llms.txt"), temp_dir.join(format!("llms_{host}.txt"))),
            ("llms-ctx.txt", format!("{base_url}/llms-ctx.txt"), temp_dir.join(format!("llms-ctx_{host}.txt"))),
            ("llms-ctx-full.txt", format!("{base_url}/llms-ctx-full.txt"), temp_dir.join(format!("llms-ctx-full_{host}.txt"))),
        ];

        // Add subpath llms files if not at root
        if parent_path != "/" {
            let subpath_id = parent_path.trim_matches('/').replace('/', "_");
            llms_files.extend([
                ("llms.txt (subpath)", format!("{base_url}{parent_path}llms.txt"), temp_dir.join(format!("llms_{host}_{subpath_id}.txt"))),
                ("llms-ctx.txt (subpath)", format!("{base_url}{parent_path}llms-ctx.txt"), temp_dir.join(format!("llms-ctx_{host}_{subpath_id}.txt"))),
                ("llms-ctx-full.txt (subpath)", format!("{base_url}{parent_path}llms-ctx-full.txt"), temp_dir.join(format!("llms-ctx-full_{host}_{subpath_id}.txt"))),
            ]);
        }

        if use_cache && file_path.exists() {
            let metadata = tokio::fs::metadata(&file_path).await.ok();
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            let llms_note: String = llms_files
                .iter()
                .filter(|(_, _, path)| path.exists())
                .map(|(name, _, path)| format!("\n{name}: {}", path.display()))
                .collect();
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Cached: {} ({} bytes){llms_note}",
                file_path.display(),
                size
            ))]));
        }

        // Fetch main URL and all llms files in parallel
        let llms_futures: Vec<_> = llms_files.iter().map(|(_, url, _)| reqwest::get(url)).collect();
        let (main_result, llms_results) = tokio::join!(
            reqwest::get(url.clone()),
            futures::future::join_all(llms_futures),
        );

        // Process main response
        let response = main_result.map_err(|e| {
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

        // Process llms files if successful
        let mut llms_notes = Vec::new();
        for (i, result) in llms_results.into_iter().enumerate() {
            if let Ok(resp) = result {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        let (name, _, path) = &llms_files[i];
                        if tokio::fs::write(path, &bytes).await.is_ok() {
                            llms_notes.push(format!("{name}: {} ({} bytes)", path.display(), bytes.len()));
                        }
                    }
                }
            }
        }
        let llms_note = if llms_notes.is_empty() {
            String::new()
        } else {
            format!("\n{}", llms_notes.join("\n"))
        };

        let type_info = content_type.map(|t| format!(", {t}")).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Downloaded: {} ({} bytes{}){llms_note}",
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
