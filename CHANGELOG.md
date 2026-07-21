# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Updated all workspace dependencies to latest compatible versions (clap 4.6.3, regex 1.13.1, syn 2.0.119, proc-macro2 1.0.107)
- Pinned Rust toolchain to `stable` channel (was previously pinned to 1.85)
- Replaced `lazy_static` with `std::sync::LazyLock` in `nix-lint-macros` and `nix-lint-rules` crates
- Precompiled regex patterns in `no_secrets.rs` into `LazyLock` for improved performance and ReDoS mitigation
- Removed `module_name_repetitions` clippy allow from `nix-lint-rules/src/lib.rs`
- Fixed unsafe array indexing in `nix-lint-core/src/lib.rs` (replaced `errors[0]` with `errors.first()`)
- Removed `missing_errors_doc` and `missing_panics_doc` workspace-level clippy allows
- Removed `clippy::collapsible_if` allows from `nix-lint/src/main.rs` and `nix-lint-rules/src/lib.rs`
- Removed `#![allow(dead_code)]` from all 20 test modules in `nix-lint-rules/src/` rule files
- Pinned all GitHub Actions to SHA references for supply-chain security
- Added `nix fmt --check` step to CI workflow
- Added macOS runner to CI matrix
