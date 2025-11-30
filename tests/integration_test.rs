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
            use_cache: false,
        }))
        .await
        .expect("Fetch should succeed");

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

    assert!(result_text.starts_with("Downloaded: .tempwebfetch/"));

    let file_path_str = result_text
        .strip_prefix("Downloaded: ")
        .and_then(|s| s.split_whitespace().next())
        .expect("Result should contain file path");
    let file_path = Path::new(file_path_str);

    assert!(file_path.exists(), "Downloaded file should exist");
    assert!(file_path.starts_with(".tempwebfetch"));

    let content = tokio::fs::read_to_string(&file_path).await.expect("Failed to read file");
    assert!(content.contains("grug"));
}

#[tokio::test]
async fn test_fetch_creates_directory() {
    let _ = tokio::fs::remove_dir_all(".tempwebfetch").await;

    let temp_dir = Path::new(".tempwebfetch");
    assert!(!temp_dir.exists());

    let webfetch = WebFetch::new();
    webfetch
        .fetch(Parameters(FetchRequest {
            url: "https://httpbin.org/html".to_string(),
            use_cache: false,
        }))
        .await
        .expect("Fetch should succeed");

    assert!(temp_dir.exists());
    assert!(temp_dir.is_dir());
}

#[tokio::test]
async fn test_invalid_url_format() {
    let webfetch = WebFetch::new();
    let result = webfetch
        .fetch(Parameters(FetchRequest {
            url: "not-a-valid-url".to_string(),
            use_cache: false,
        }))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Invalid URL"));
}

#[tokio::test]
async fn test_http_404_error() {
    let webfetch = WebFetch::new();
    let result = webfetch
        .fetch(Parameters(FetchRequest {
            url: "https://httpbin.org/status/404".to_string(),
            use_cache: false,
        }))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("404"));
}

#[tokio::test]
async fn test_file_path_is_relative() {
    let _ = tokio::fs::remove_dir_all(".tempwebfetch").await;

    let webfetch = WebFetch::new();
    let result = webfetch
        .fetch(Parameters(FetchRequest {
            url: "https://httpbin.org/html".to_string(),
            use_cache: false,
        }))
        .await
        .expect("Fetch should succeed");

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
        .strip_prefix("Downloaded: ")
        .and_then(|s| s.split_whitespace().next())
        .expect("Result should contain file path");
    let file_path = Path::new(file_path_str);

    assert!(file_path.is_relative());
    assert!(file_path.starts_with(".tempwebfetch"));
    assert!(file_path.file_name().is_some());
    assert!(file_path.exists());
}
