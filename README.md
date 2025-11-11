# WebFetch2 MCP Server

A Model Context Protocol (MCP) server that downloads web content to local storage and returns file paths. Built in Rust using the official MCP Rust SDK.

## Features

- **Simple fetch tool**: Download content from any URL
- **Local storage**: Saves content to `.tempwebfetch/` directory
- **Deterministic filenames**: Uses content-addressed naming with URL hashing
- **Comprehensive error handling**: Clear, actionable error messages
- **Tested**: Includes snapshot tests for reliability

## Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Cargo (comes with Rust)

### Build from Source

```bash
# Clone the repository (or navigate to the project directory)
cd webfetch2

# Build the project
cargo build --release

# The binary will be at: target/release/webfetch2
```

## Usage

### As an MCP Server

WebFetch2 communicates over standard input/output (stdio) and is designed to be used with MCP-compatible clients like Claude Desktop.

### Claude Desktop Configuration

Add this to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "webfetch2": {
      "command": "/absolute/path/to/webfetch2/target/release/webfetch2",
      "args": []
    }
  }
}
```

After adding the configuration:

1. Restart Claude Desktop
2. The MCP UI elements should appear
3. You can now use the `fetch` tool

### Claude Code Configuration

```sh
cargo build --release && claude mcp add --transport stdio webfetch2 -- "$(pwd)/target/release/webfetch2"
```

Then restart Claude Code.
Verify the tool is available with `/mcp`

### Example Usage in Claude

```
Use the fetch tool to download https://grugbrain.dev/
```

The server will:

1. Download the content from the URL
2. Save it to `.tempwebfetch/` with a content-addressed filename
3. Return the absolute path to the downloaded file

## Available Tools

### `fetch`

Downloads content from a URL and saves it locally.

**Parameters:**

- `url` (string, required): The URL to fetch content from

**Returns:**

- Success message with the absolute file path

**Example:**

```json
{
  "url": "https://example.com/page.html"
}
```

**Response:**

```
Content downloaded successfully to: /absolute/path/to/webfetch2/.tempwebfetch/8c0568b060b2f1b6.html
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Accept snapshot changes (after reviewing)
cargo insta accept
```

### Project Structure

```
webfetch2/
├── src/
│   └── main.rs          # Server implementation
├── tests/
│   ├── integration_test.rs   # Integration tests
│   └── snapshots/            # Snapshot test baselines
├── .tempwebfetch/            # Downloaded content (gitignored)
├── Cargo.toml
└── README.md
```

### Testing with MCP Inspector

You can test the server using the official MCP Inspector:

```bash
npx @modelcontextprotocol/inspector cargo run --release
```

## How It Works

1. **URL Validation**: Parses and validates the provided URL
2. **Directory Creation**: Ensures `.tempwebfetch/` exists
3. **Filename Generation**: Creates a deterministic filename using URL hash + extension
4. **Download**: Fetches content using reqwest
5. **Storage**: Writes content to the local file
6. **Response**: Returns the absolute path to the downloaded file

## Error Handling

The server provides clear, actionable error messages for common issues:

- **Invalid URL**: Suggests correct URL format
- **Network errors**: Advises checking connectivity
- **HTTP errors**: Reports status codes with descriptions
- **File I/O errors**: Suggests checking permissions and disk space

## Contributing

Contributions are welcome! Please ensure:

1. Tests pass: `cargo test`
2. Code builds without warnings: `cargo build --release`
3. Follow existing code style
4. Add tests for new functionality

## License

[Add your license here]

## Acknowledgments

Built with:

- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - Rust MCP SDK
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [tokio](https://tokio.rs/) - Async runtime
- [insta](https://github.com/mitsuhiko/insta) - Snapshot testing
