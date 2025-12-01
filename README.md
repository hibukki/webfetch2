# WebFetch2 MCP Server

A better web fetch tool that downloads files and lets the LLM use its native file reading tools.

Also auto-discovers [llms.txt](https://llmstxt.org/) files on the domain.

## Why?

The built-in WebFetch tool has limitations::

> Fetches content from a specified URL and processes it using an AI model
> Results may be summarized if the content is very large

And recommends using another tool if possible:

> IMPORTANT: If an MCP-provided web fetch tool is available, prefer using
> that tool instead of this one, as it may have fewer restrictions. All
> MCP-provided tools start with "mcp\_\_".

The quotes are from the original tool's description (got them from asking claude-code, ask your own claude-code to verify!)

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
