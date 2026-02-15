# env-guard

A fast CLI tool that validates environment variables against a schema before running commands. Written in Rust for zero-overhead startup.

## Why?

Missing or malformed environment variables cause runtime crashes. `env-guard` catches these at startup — before your app even loads.

## Usage

```bash
# Define a schema
cat > .env.schema <<EOF
DATABASE_URL=required,url
PORT=required,integer,range:1024-65535
LOG_LEVEL=optional,enum:debug|info|warn|error
API_KEY=required,min_length:32
EOF

# Validate and run
env-guard --schema .env.schema -- cargo run

# Just validate (no command)
env-guard --schema .env.schema --check
```

## Schema Format

```
VAR_NAME=<required|optional>[,<validator>...]

Validators:
  - url          Must be a valid URL
  - integer      Must be an integer
  - range:N-M    Integer within range
  - enum:a|b|c   One of listed values
  - min_length:N At least N characters
  - regex:PATTERN  Match regex
```

## Install

```bash
cargo install env-guard
```

## Build

```bash
cargo build --release
```

## License

MIT