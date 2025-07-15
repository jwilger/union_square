> *"I don't care what they're talking about. All I want is a nice, fat recording."*
> 
> — Harry Caul, The Conversation (1974)

<table>
<tr>
<td width="200" align="center">
<img src="logo.svg" width="180" alt="Union Square Logo">
</td>
<td>

# Union Square

**A transparent, high-performance proxy for LLM API calls**

[![Build Status](https://img.shields.io/badge/build-not%20yet-lightgrey)](https://github.com/jwilger/union_square/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
![Status: Early Development](https://img.shields.io/badge/Status-Early%20Development-orange)

</td>
</tr>
</table>

---

Comprehensive observability, testing, and optimization for AI-powered applications.

## Project Status

🚧 **Early Development** - This project is in the initial development phase. Core functionality is being implemented.

## Overview

Union Square acts as a drop-in proxy between your applications and LLM providers (OpenAI, Anthropic, AWS Bedrock, Google Vertex AI), capturing every interaction for analysis, debugging, and testing. It's designed to add minimal latency (< 5ms) while providing powerful observability features.

### Key Features

- **Transparent Proxy** - Drop-in replacement requiring no code changes
- **Multi-Provider Support** - OpenAI, Anthropic, AWS Bedrock, Google Vertex AI
- **Low Latency** - Asynchronous recording with < 5ms overhead
- **Test Case Extraction** - Convert problematic conversations into automated tests
- **Cost & Performance Analytics** - Track token usage, latency, and costs
- **Streaming Support** - Full support for SSE/streaming responses
- **Privacy Controls** - Configurable recording rules and PII detection
- **Self-Hosted** - Run in your own infrastructure

## Use Cases

### For Developers
- Debug LLM interactions with full conversation history
- Extract and run test cases to prevent regressions
- A/B test different models and providers
- Monitor performance and error rates

### For Customer Support
- Look up customer sessions to troubleshoot issues
- Flag problematic conversations for engineering review
- Add context and notes to help developers

### For Management
- Track AI costs across applications and teams
- Monitor performance metrics and F-scores
- Optimize model selection and usage patterns

## Quick Start

### Prerequisites

- Rust 1.88+ (or use the Nix development shell which provides the exact version)
- PostgreSQL 14+
- Docker (optional, for containerized deployment)
- Nix (optional, but recommended for consistent development environment)

### Installation

```bash
# Clone the repository
git clone https://github.com/jwilger/union_square.git
cd union_square

# Option 1: Using Nix (Recommended)
# This provides a consistent development environment with Rust 1.88.0
nix develop

# Option 2: Manual setup
# Ensure you have Rust 1.88+ installed
rustup update

# Start PostgreSQL
docker-compose up -d

# Build the project
cargo build --release

# Run the server
./target/release/union_square
```

### Using the Nix Development Shell

This project includes a Nix flake that provides a complete development environment. The Nix shell includes:

- **Rust 1.88.0** - The exact Rust version used by this project
- **Cargo** and all standard Rust tools (rustc, rustfmt, clippy, etc.)
- **Pre-configured environment** - All necessary build dependencies

To use the Nix development shell:

```bash
# Enter the development shell
nix develop

# All Rust tools are now available
cargo --version  # Should show cargo 1.88.0
rustc --version  # Should show rustc 1.88.0

# Run any cargo commands as usual
cargo build
cargo test
cargo run
```

The Nix shell ensures all developers and CI environments use identical tool versions, eliminating "works on my machine" issues.

### Basic Usage

Replace your LLM API endpoints with Union Square URLs:

```diff
- https://api.openai.com/v1/chat/completions
+ https://your-union-square.com/openai/v1/chat/completions
```

Add session tracking headers:

```http
X-Union-Square-Session-ID: your-session-id
X-Union-Square-Metadata: {"user_id": "12345", "feature": "chat"}
```

## Configuration

Union Square uses a TOML configuration file. See `config.example.toml` for all options.

```toml
[server]
port = 8080
host = "0.0.0.0"

[database]
url = "postgresql://user:pass@localhost/union_square"

[cache]
enabled = true
ttl_seconds = 3600

[privacy]
pii_detection = true
default_recording = true
```

## Architecture

Union Square follows a functional core, imperative shell architecture:

- **Proxy Layer** - Minimal overhead request forwarding
- **Recording Pipeline** - Asynchronous capture and storage
- **Analysis Engine** - Test evaluation and metrics calculation
- **Web Interface** - Leptos-based reactive UI

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

### Development Environment

We strongly recommend using the Nix development shell for a consistent environment:

```bash
# Enter the Nix shell (provides Rust 1.88.0 and all tools)
nix develop
```

### Common Commands

```bash
# Format code
cargo fmt

# Run lints
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test

# Type check
cargo check --all-targets
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Roadmap

- [x] Initial project setup
- [ ] Core proxy functionality
- [ ] Basic web interface
- [ ] Test case extraction
- [ ] Analytics dashboards
- [ ] Plugin system for exporters

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

## Security

For security vulnerabilities, please email security@example.com instead of using the issue tracker.
