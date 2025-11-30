# WebFetch2 MCP Server

A better web fetch tool that simply downloads the file and lets the caller (e.g claude code) use their existing tools to process the file.

## Installation

### Prerequisites

[rust](https://rustup.rs/)

## Usage

### Claude Code Configuration

```sh
claude mcp remove webfetch2 ; cargo build --release && claude mcp add --transport stdio webfetch2 -- "$(pwd)/target/release/webfetch2"
```

Then restart Claude Code.
Verify the tool is available with `/mcp`

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
