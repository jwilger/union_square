# Contributing to Union Square

Thank you for your interest in contributing to Union Square! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/union_square.git`
3. Add upstream remote: `git remote add upstream https://github.com/jwilger/union_square.git`
4. Create a feature branch: `git checkout -b my-feature-branch`

## Development Setup

### Prerequisites

- Rust 1.75+ (check with `rustc --version`)
- PostgreSQL 14+
- Docker and Docker Compose (for PostgreSQL databases)
- Nix (optional, for development environment)

### Initial Setup

```bash
# Enter development environment (if using Nix)
nix develop

# Start databases
docker-compose up -d

# Run tests to verify setup
cargo test --workspace
```

## Commit Guidelines

### Commit Message Format

We follow a specific commit message format:

```
Short summary (max 50 chars)

Detailed explanation of the change. Wrap lines at 72 characters.
Focus on WHY the change was made, not just what changed.

Include any breaking changes, performance implications, or other
important notes.
```

Example:
```
Add caching support for LLM responses

Response caching reduces costs and latency for repeated queries.
The cache is configurable per-application with request-level
overrides via headers.

Implements LRU cache with configurable TTL. Cache hits are clearly
marked in session logs for transparency. Cache keys can be based on
request content, headers, or custom rules.
```

### GPG Commit Signing (Recommended)

While not required, we encourage contributors to sign their commits with GPG for added security and authenticity.

#### Setting up GPG Signing

1. **Generate a GPG key** (if you don't have one):
   ```bash
   gpg --full-generate-key
   ```
   - Choose RSA and RSA (default)
   - Key size: 4096 bits
   - Expiration: Your preference (1-2 years recommended)
   - Use your GitHub email address

2. **List your GPG keys**:
   ```bash
   gpg --list-secret-keys --keyid-format=long
   ```
   Look for a line like `sec rsa4096/3AA5C34371567BD2`

3. **Export your public key**:
   ```bash
   gpg --armor --export 3AA5C34371567BD2
   ```
   Copy the output including `-----BEGIN PGP PUBLIC KEY BLOCK-----` and `-----END PGP PUBLIC KEY BLOCK-----`

4. **Add the key to GitHub**:
   - Go to Settings → SSH and GPG keys
   - Click "New GPG key"
   - Paste your public key

5. **Configure Git to sign commits**:
   ```bash
   git config --global user.signingkey 3AA5C34371567BD2
   git config --global commit.gpgsign true
   ```

6. **Configure GPG agent** (for password caching):
   ```bash
   echo "default-cache-ttl 3600" >> ~/.gnupg/gpg-agent.conf
   echo "max-cache-ttl 86400" >> ~/.gnupg/gpg-agent.conf
   ```

#### Verifying Signed Commits

To verify signatures on existing commits:
```bash
git log --show-signature
```

To verify a specific commit:
```bash
git verify-commit <commit-hash>
```

## Development Workflow

1. **Create a feature branch** from `main`
2. **Make your changes** following our coding standards
3. **Write tests** for new functionality
4. **Run the test suite**: `cargo test --workspace`
5. **Run linting**: `cargo clippy --workspace --all-targets -- -D warnings`
6. **Format code**: `cargo fmt`
7. **Commit your changes** with descriptive messages
8. **Push to your fork** and create a pull request

## Testing

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests with nextest (recommended)
cargo nextest run --workspace

# Run integration tests only
cargo test --test '*' --workspace
```

### Writing Tests

- Write unit tests for pure functions
- Write integration tests for proxy operations
- Use property-based testing for invariants
- Follow the existing test patterns in the codebase
- Test streaming responses and error conditions

## Pull Request Process

1. **Update documentation** for any changed functionality
2. **Add tests** for new features
3. **Ensure CI passes** - all checks must be green
4. **Update CHANGELOG.md** with your changes (once we have one)
5. **Request review** from maintainers

### PR Title Format

Use clear, descriptive titles:
- ✅ "Add response caching with configurable TTL"
- ✅ "Fix streaming response handling for OpenAI API"
- ❌ "Fix bug"
- ❌ "Update code"

## Security

### Security Checklist for Contributors

Before submitting your PR, ensure:

- [ ] No hardcoded secrets, API keys, or credentials
- [ ] All user input is validated using type-safe validators
- [ ] SQL queries use parameterized statements (via `sqlx`)
- [ ] Error messages don't leak sensitive information
- [ ] Privacy headers are respected (do-not-record)
- [ ] New dependencies are justified and from reputable sources
- [ ] Tests don't contain real API keys or PII

### Reporting Security Issues

For security vulnerabilities, please email security@example.com instead of using the issue tracker.

## Code Style

### Rust Guidelines

1. **Follow Rust idioms** - use `clippy` to catch anti-patterns
2. **Use meaningful names** - prefer clarity over brevity
3. **Document public APIs** - all public items need doc comments
4. **Prefer composition** - small, focused functions that compose
5. **Handle errors explicitly** - use `Result` types, avoid `unwrap()`
6. **Minimize proxy overhead** - performance is critical

### Type-Driven Development

Union Square follows strict type-driven development principles:

1. **Types first** - design types that make illegal states unrepresentable
2. **Parse, don't validate** - use smart constructors with validation
3. **No primitive obsession** - wrap primitives in domain types
4. **Total functions** - handle all cases explicitly

Example:
```rust
// Good: Domain type with validation
#[nutype(
    validate(not_empty, regex = "^[a-zA-Z0-9-]+$"),
    derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Deref, Serialize, Deserialize)
)]
pub struct SessionId(String);

// Bad: Using raw String
pub fn record_session(session_id: String) { ... }

// Good: Using domain type
pub fn record_session(session_id: SessionId) { ... }
```

### Performance Considerations

Since Union Square is a proxy, performance is critical:

- Avoid blocking operations in the request path
- Use async/await properly - don't block the runtime
- Profile before optimizing
- Document any performance-critical code

## Documentation

### Code Documentation

- Document all public APIs with doc comments
- Include examples in doc comments where helpful
- Explain "why" not just "what"
- Document performance implications
- Note any provider-specific behavior

### User Documentation

When adding new features:
1. Update relevant sections in the README
2. Add configuration examples
3. Update API documentation
4. Document any new headers or parameters

### Architecture Decision Records (ADRs)

We use ADRs to document significant architectural decisions. Contributors should:

#### When to Create an ADR

Create an ADR when:
- Choosing between multiple technology options
- Introducing a new architectural pattern
- Making significant changes to existing architecture
- Deciding on API design approaches
- Establishing security or performance strategies

#### How to Create an ADR

1. **Create a new ADR**:
   ```bash
   npm run adr:new
   # Follow the interactive prompts
   ```

2. **Use the template structure**:
   - Context and problem statement
   - Decision drivers (what factors influence the decision)
   - Considered options with pros/cons
   - Decision outcome
   - Consequences (both positive and negative)

3. **Number ADRs sequentially**: The tool handles this automatically

4. **Link related ADRs**: Reference other ADRs when decisions are related

#### ADR Guidelines

- **Be thorough**: Document all viable options considered
- **Be honest**: Include both positive and negative consequences
- **Be clear**: Write for future developers who lack current context
- **Be timely**: Create ADRs when making decisions, not retroactively

#### Reviewing ADRs

When reviewing PRs with ADRs:
- Ensure all reasonable alternatives are considered
- Check that decision drivers align with project goals
- Verify consequences are realistic and complete
- Confirm the decision follows from the analysis

## Questions?

- Open a [Discussion](https://github.com/jwilger/union_square/discussions) for questions
- Check existing issues before creating new ones
- Review the [PRD](PRD.md) for product context

## Recognition

Contributors will be recognized in:
- The project README
- Release notes
- Special thanks in documentation

Thank you for contributing to Union Square!
