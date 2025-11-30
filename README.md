# WebFetch2 MCP Server

A better web fetch tool that downloads files and lets the LLM use its native file reading tools.

Also auto-discovers [llms.txt](https://llmstxt.org/) files on the domain.

## Why?

Claude Code's built-in WebFetch tool has limitations:
- Uses Haiku to filter content by prompt, returning only a small subset
- Can't re-read the page with different focus
- May lose important context through summarization

This tool simply downloads the full file. The LLM can then use its native file reading tools to read it multiple times with full context.

## Installation

Requires [Rust](https://rustup.rs/).

### From GitHub (no clone needed)

```sh
cargo install --git https://github.com/hibukki/webfetch2
claude mcp remove webfetch2 ; claude mcp add --transport stdio webfetch2 -- ~/.cargo/bin/webfetch2
```

### From local clone

```sh
cargo build --release
claude mcp remove webfetch2 ; claude mcp add --transport stdio webfetch2 -- "$(pwd)/target/release/webfetch2"
```

Then restart Claude Code. Verify with `/mcp`.

## Development

```bash
cargo test
cargo test -- --nocapture
```
