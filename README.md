# nix-flake-parts-lint

AST-based linter for Nix Flake Parts projects.

This tool statically analyzes Nix Flake configurations, checking for common issues, best practices, and potential problems. It's built in Rust using the `rnix` parser and provides structured linting rules for Nix projects.

## Features

- AST-based Nix parsing using the `rnix` crate
- Modular rule system with a registry of linter rules
- Supports multiple severity levels (ERROR, WARN, HINT)
- Shell completion for bash, zsh, and fish
- Nix package builder included

## Installation

### Using Nix

```bash
nix-build
```

Or use flake integration if you have flakes enabled.

### From source

```bash
cargo build --release
```

## Usage

```bash
nix-lint <directory>
```

The linter will recursively scan the directory for `.nix` files and report any issues found.

### Exit codes

- `0` - No issues found
- `1` - Issues were found (or usage error)

## Project structure

| Crate | Description |
|-------|-------------|
| `nix-lint` | CLI binary and entry point |
| `nix-lint-core` | Core linting engine and data structures |
| `nix-lint-rules` | Linting rule implementations |
| `nix-lint-macros` | Procedural macros for rule definitions |

## License

MIT
