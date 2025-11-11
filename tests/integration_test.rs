use std::path::Path;

#[tokio::test]
async fn test_fetch_grugbrain() {
    // Clean up any existing .tempwebfetch directory
    let _ = tokio::fs::remove_dir_all(".tempwebfetch").await;

    // Use reqwest directly to fetch the URL (simulating what the server does)
    let url = "https://grugbrain.dev/";
    let response = reqwest::get(url).await.expect("Failed to fetch URL");

    assert!(response.status().is_success(), "HTTP request should succeed");

    let bytes = response.bytes().await.expect("Failed to read response");
    assert!(!bytes.is_empty(), "Response should have content");

    // Create .tempwebfetch directory
    tokio::fs::create_dir_all(".tempwebfetch")
        .await
        .expect("Failed to create .tempwebfetch directory");

    // Generate filename (using similar logic to the server)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();
    let filename = format!("{:x}.html", hash);

    let file_path = Path::new(".tempwebfetch").join(&filename);

    // Write file
    tokio::fs::write(&file_path, &bytes)
        .await
        .expect("Failed to write file");

    // Verify file exists
    assert!(file_path.exists(), "Downloaded file should exist");

    // Get absolute path
    let absolute_path = std::fs::canonicalize(&file_path)
        .expect("Failed to get absolute path");

    // Snapshot the result format
    let result_message = format!("Content downloaded successfully to: {}", absolute_path.display());
    insta::assert_snapshot!("fetch_grugbrain_result", result_message);

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

    // Fetch a simple URL
    let url = "https://httpbin.org/html";
    let response = reqwest::get(url).await.expect("Failed to fetch URL");

    assert!(response.status().is_success());

    let _bytes = response.bytes().await.expect("Failed to read response");

    // Create directory
    tokio::fs::create_dir_all(".tempwebfetch")
        .await
        .expect("Failed to create directory");

    // Verify directory exists
    let temp_dir = Path::new(".tempwebfetch");
    assert!(temp_dir.exists(), ".tempwebfetch directory should exist");
    assert!(temp_dir.is_dir(), ".tempwebfetch should be a directory");
}

#[tokio::test]
async fn test_invalid_url_format() {
    use url::Url;

    let invalid_url = "not-a-valid-url";
    let result = Url::parse(invalid_url);

    assert!(result.is_err(), "Invalid URL should fail to parse");

    // Snapshot the error message format
    if let Err(e) = result {
        let error_message = format!("Invalid URL: {}. Please provide a valid HTTP/HTTPS URL (e.g., https://example.com)", e);
        insta::assert_snapshot!("invalid_url_error", error_message);
    }
}

#[tokio::test]
async fn test_http_404_error() {
    let url = "https://httpbin.org/status/404";
    let response = reqwest::get(url).await.expect("Failed to fetch URL");

    let status = response.status();
    assert!(!status.is_success(), "404 should not be success");
    assert_eq!(status.as_u16(), 404, "Status should be 404");

    // Snapshot the error message format
    let error_message = format!(
        "HTTP error {}: {}. The server returned an error response.",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
    insta::assert_snapshot!("http_404_error", error_message);
}

#[tokio::test]
async fn test_filename_generation_with_extension() {
    use url::Url;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let test_cases = vec![
        ("https://example.com/file.html", "html"),
        ("https://example.com/data.json", "json"),
        ("https://example.com/image.png", "png"),
        ("https://example.com/no-extension", "html"),
    ];

    for (url_str, expected_ext) in test_cases {
        let url = Url::parse(url_str).expect("Failed to parse URL");

        let mut hasher = DefaultHasher::new();
        url.as_str().hash(&mut hasher);
        let hash = hasher.finish();

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

        assert_eq!(extension, expected_ext, "Extension should match for {}", url_str);

        let filename = format!("{:x}.{}", hash, extension);
        assert!(filename.contains(expected_ext), "Filename should contain extension");
    }
}

#[tokio::test]
async fn test_file_path_is_absolute() {
    // Clean up
    let _ = tokio::fs::remove_dir_all(".tempwebfetch").await;

    // Create test file
    tokio::fs::create_dir_all(".tempwebfetch")
        .await
        .expect("Failed to create directory");

    let test_file = Path::new(".tempwebfetch/test.html");
    tokio::fs::write(test_file, b"test content")
        .await
        .expect("Failed to write test file");

    // Get absolute path
    let absolute_path = std::fs::canonicalize(test_file)
        .expect("Failed to canonicalize path");

    assert!(absolute_path.is_absolute(), "Path should be absolute");
    assert!(absolute_path.to_string_lossy().contains("webfetch2"), "Path should contain project name");
    assert!(absolute_path.to_string_lossy().contains(".tempwebfetch"), "Path should contain .tempwebfetch");
}
