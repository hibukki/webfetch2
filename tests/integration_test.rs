use std::path::Path;
use webfetch2::{WebFetch, FetchRequest};
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn test_fetch_grugbrain() {
    // Clean up any existing .tempwebfetch directory
    let _ = tokio::fs::remove_dir_all(".tempwebfetch").await;

    let webfetch = WebFetch::new();
    let url = "https://grugbrain.dev/";

    let result = webfetch
        .fetch(Parameters(FetchRequest {
            url: url.to_string(),
        }))
        .await
        .expect("Fetch should succeed");

    // Extract the text content from the result
    let result_text = result.content
        .iter()
        .filter_map(|c| {
            if let rmcp::model::RawContent::Text(t) = &c.raw {
                Some(t.text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    // Snapshot the result message
    insta::assert_snapshot!("fetch_grugbrain_result", result_text);

    // Extract the file path from the result message
    let file_path_str = result_text
        .strip_prefix("Content downloaded successfully to: ")
        .expect("Result should contain file path");
    let file_path = Path::new(file_path_str);

    // Verify file exists
    assert!(file_path.exists(), "Downloaded file should exist");

    // Verify the file is in the correct location
    assert!(file_path.starts_with(".tempwebfetch"), "Path should start with .tempwebfetch");

    // Verify file content (snapshot first 500 bytes to check it's HTML)
    let content = tokio::fs::read_to_string(&file_path)
        .await
        .expect("Failed to read file");

    let preview = if content.len() > 500 {
        &content[..500]
    } else {
        &content
    };

    insta::assert_snapshot!("fetch_grugbrain_content_preview", preview);
}

#[tokio::test]
async fn test_fetch_creates_directory() {
    // Clean up
    let _ = tokio::fs::remove_dir_all(".tempwebfetch").await;

    // Verify directory doesn't exist before fetch
    let temp_dir = Path::new(".tempwebfetch");
    assert!(!temp_dir.exists(), ".tempwebfetch should not exist before fetch");

    // Fetch a simple URL
    let webfetch = WebFetch::new();
    let url = "https://httpbin.org/html";

    webfetch
        .fetch(Parameters(FetchRequest {
            url: url.to_string(),
        }))
        .await
        .expect("Fetch should succeed");

    // Verify directory was created by fetch
    assert!(temp_dir.exists(), ".tempwebfetch directory should exist after fetch");
    assert!(temp_dir.is_dir(), ".tempwebfetch should be a directory");
}

#[tokio::test]
async fn test_invalid_url_format() {
    let webfetch = WebFetch::new();
    let invalid_url = "not-a-valid-url";

    let result = webfetch
        .fetch(Parameters(FetchRequest {
            url: invalid_url.to_string(),
        }))
        .await;

    assert!(result.is_err(), "Invalid URL should fail");

    // Snapshot the error message from the actual implementation
    let error = result.unwrap_err();
    insta::assert_snapshot!("invalid_url_error", error.message);
}

#[tokio::test]
async fn test_http_404_error() {
    let webfetch = WebFetch::new();
    let url = "https://httpbin.org/status/404";

    let result = webfetch
        .fetch(Parameters(FetchRequest {
            url: url.to_string(),
        }))
        .await;

    assert!(result.is_err(), "404 should return an error");

    // Snapshot the error message from the actual implementation
    let error = result.unwrap_err();
    insta::assert_snapshot!("http_404_error", error.message);
}

#[tokio::test]
async fn test_file_path_is_relative() {
    // Clean up
    let _ = tokio::fs::remove_dir_all(".tempwebfetch").await;

    let webfetch = WebFetch::new();
    let url = "https://httpbin.org/html";

    let result = webfetch
        .fetch(Parameters(FetchRequest {
            url: url.to_string(),
        }))
        .await
        .expect("Fetch should succeed");

    // Extract the file path from the result
    let result_text = result.content
        .iter()
        .filter_map(|c| {
            if let rmcp::model::RawContent::Text(t) = &c.raw {
                Some(t.text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    let file_path_str = result_text
        .strip_prefix("Content downloaded successfully to: ")
        .expect("Result should contain file path");
    let file_path = Path::new(file_path_str);

    // Verify the path is relative and has expected structure
    assert!(file_path.is_relative(), "Path should be relative");
    assert!(file_path.starts_with(".tempwebfetch"), "Path should start with .tempwebfetch");
    assert!(file_path.file_name().is_some(), "Path should have a filename");

    // Verify file actually exists
    assert!(file_path.exists(), "File should exist at the returned path");
}
