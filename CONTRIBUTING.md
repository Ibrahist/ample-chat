# Contributing to ample-chat

Thank you for your interest in improving `ample-chat`!

## Code Quality & Safety Rules

1. **Zero Unsafe Code**: `#![forbid(unsafe_code)]` is strictly enforced across every crate in this workspace. No PR introducing `unsafe` blocks will be accepted.
2. **RAM-Only Policy**: No code may persist chat messages, private keys, or passwords to disk, temporary files, logs, or databases.
3. **No Secrets in Logs**: Tracing must never print or format passwords, private keys, room salts, or unencrypted plaintext.
4. **Memory Hygiene**: Secret keys, tokens, and passwords must be zeroized upon drop using `zeroize::Zeroizing` or implementing `Zeroize`.

## Development Setup

```bash
# Clone the repository
git clone https://github.com/ample-chat/ample-chat.git
cd ample-chat

# Format check
cargo fmt --all -- --check

# Run linter
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --workspace --all-features
```

## Pull Request Guidelines

1. Ensure all existing and new unit/integration tests pass.
2. Ensure `cargo fmt` and `cargo clippy -- -D warnings` produce zero warnings.
3. Add appropriate test coverage for any new features or bug fixes.
4. Update `CHANGELOG.md` with a summary of user-facing changes.
