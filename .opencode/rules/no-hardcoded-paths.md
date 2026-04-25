# Rule: No Hardcoded Paths

Never hardcode file paths, URLs, or environment-specific values in source code.

## Forbidden

```rust
const DB_URL: &str = "postgres://localhost:5432/union_square";
let path = "/home/jwilger/projects/union_square/config.toml";
let endpoint = "https://bedrock.us-east-1.amazonaws.com";
```

## What To Do Instead

- Use configuration files (`config` crate) loaded at startup
- Use environment variables for secrets and deployment-specific values
- Use `std::env::current_dir()` or `std::env::current_exe()` for relative paths
- Use `tempfile` crate for temporary paths in tests

## Configuration Pattern

```rust
#[derive(Debug, Deserialize)]
struct Config {
    database_url: String,
    aws_region: String,
}

let config = Config::builder()
    .add_source(config::Environment::with_prefix("UNION_SQUARE"))
    .add_source(config::File::with_name("config"))
    .build()?;
```

## Rationale

Hardcoded paths break on other developers' machines, in CI, and in production. They make the code non-portable and harder to test.

## Enforcement

- `ast-grep` rule scanning for hardcoded path patterns
- Code review by `security-reviewer`
