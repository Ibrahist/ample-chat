# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-25

### Added
- Complete rewrite and port of Python `cmd-chat` into idiomatic, 100% safe Rust.
- `#![forbid(unsafe_code)]` enforced across all crates.
- Zero-knowledge SRP-6a (RFC 5054) authentication flow over HTTP endpoints (`/srp/init`, `/srp/verify`).
- Argon2id memory-hard key derivation preceding HKDF-SHA256 expansion for room encryption key.
- AES-256-GCM authenticated encryption for end-to-end messaging with 12-byte CSPRNG nonces.
- Anti-spoofing and anti-replay protection with monotonic counters and sender IDs inside encrypted payloads.
- High-performance, RAM-only axum WebSocket relay server with token-bucket rate limiting.
- Rich terminal UI built with `ratatui` and `crossterm` featuring user list, message backlog, auto-scroll, and live cursor.
- Panic hook and signal handlers restoring terminal raw mode cleanly on exit or panic.
- Credential management via `AMPLE_CHAT_PASSWORD` environment variable, hidden stdin prompt, or CLI flag.
- Multi-arch Docker container packaging based on distroless non-root image.
- Comprehensive test suite (unit tests, property-based tests via `proptest`, multi-client E2E integration test, and `cargo-fuzz` target).
- Production-grade CI/CD pipelines (matrix CI, security audit with `cargo deny` and `cargo audit`, multi-target release builds, Trivy vulnerability scan).
