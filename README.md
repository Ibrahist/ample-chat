# ample-chat

> **Encrypted, RAM-only terminal chat.** No disks. No databases. No logs. Zero-knowledge authentication.

`ample-chat` is a high-performance, memory-only terminal chat application written in 100% safe, idiomatic Rust.

---

## 🛰️ Real Topology & Honest Architecture

The original `cmd-chat` claimed to be "peer-to-peer / no central server". In technical reality, it operated as a **relayed client-server architecture** where one participant acts as host and other participants connect to that host over HTTP and WebSockets.

`ample-chat` embraces an honest architectural model:

```text
 ┌─────────────┐                      ┌──────────────────────┐                      ┌─────────────┐
 │ Client A    │ ── E2E Ciphertext ─► │  ample-chat Relay    │ ── E2E Ciphertext ─► │ Client B    │
 │ (Alice)     │ ◄── E2E Ciphertext ─ │  (In-Memory Server)  │ ◄── E2E Ciphertext ─ │ (Bob)       │
 └─────────────┘                      └──────────────────────┘                      └─────────────┘
  [Room Key Derived                    [Stores RAM Ciphertext                        [Room Key Derived
   via Argon2id+HKDF]                   CANNOT Decrypt Messages]                      via Argon2id+HKDF]
```

### What the Host / Relay Server CAN See:
- IP addresses and connection timestamps of connected clients.
- Total count and usernames of currently connected peers.
- Transport-level metadata (WebSocket frame sizes, transmission frequency).
- Raw encrypted ciphertext frames and nonces.

### What the Host / Relay Server CANNOT See:
- **Your password**: Authentication uses SRP-6a (Secure Remote Password). Passwords never cross the network.
- **Your plaintext messages**: Messages are encrypted end-to-end on clients prior to sending.
- **Your room encryption key**: The room key is derived on clients via Argon2id + HKDF using the password; the server never derives or receives it.
- **Historical chat logs**: Nothing is written to disk or database. All state vanishes when the server process ends.

---

## 🌳 How It Works

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                            SRP-6a AUTHENTICATION                             │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   CLIENT (Alice)                                    SERVER (Relay)           │
│      │                                                     │                 │
│      │──────────── POST /srp/init {username, A} ──────────►│                 │
│      │             (A = client public ephemeral g^a)       │                 │
│      │                                                     │                 │
│      │◄─────────── {user_id, B, salt, room_salt} ──────────│                 │
│      │             (B = server public ephemeral kv + g^b)  │                 │
│      │             (room_salt = random 16 bytes)           │                 │
│      │                                                     │                 │
│      │  [client derives room_key via Argon2id + HKDF:      │                 │
│      │   kdf_out  = Argon2id(password, room_salt)          │                 │
│      │   room_key = HKDF-SHA256(kdf_out, "ample-chat")]    │                 │
│      │                                                     │                 │
│      │  [both compute SRP premaster secret S & key K]      │                 │
│      │                                                     │                 │
│      │──────────── POST /srp/verify {user_id, M} ─────────►│                 │
│      │             (M = client proof H(A || B || K))       │                 │
│      │                                                     │ [verifies M]    │
│      │◄─────────── {H_AMK, session_key} ───────────────────│                 │
│      │             (H_AMK = server proof H(A || M || K))   │                 │
│      │                                                     │                 │
│      │  [client verifies server proof H_AMK]               │                 │
│      │  [mutual authentication complete; password safe]    │                 │
│      │                                                     │                 │
├──────────────────────────────────────────────────────────────────────────────┤
│                         E2E ENCRYPTED WEBSOCKET CHAT                         │
├──────────────────────────────────────────────────────────────────────────────┤
│      │                                                     │                 │
│      │════════════ WebSocket /ws/chat?user_id ════════════►│ (authenticated) │
│      │                                                     │                 │
│    ┌─┴─┐                                                 ┌─┴─┐               │
│    │ C │                                                 │ S │               │
│    │ L │── Encrypt(Payload, room_key) ──────────────────►│ E │ (relays only) │
│    │ I │   [1B version + 12B nonce + AES-256-GCM cipher] │ R │               │
│    │ E │                                                 │ V │               │
│    │ N │◄─ Broadcast raw ciphertext ─────────────────────│ E │               │
│    │ T │                                                 │ R │               │
│    └─┬─┘   Decrypt(Ciphertext, room_key)                 └─┬─┘               │
│      │     Verify auth tag, sender username, and counter   │                 │
│      │                                                     │                 │
│      │  [server holds ONLY ciphertext; cannot read text]   │                 │
│      │  [on disconnect: all keys & plaintext zeroized]     │                 │
│      │                                                     │                 │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│                                KEY HIERARCHY                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  password ──┬──► SRP-6a (RFC 5054 2048-bit) ──► session_key (per-client auth)│
│             │                                                                │
│             └──► Argon2id(password, room_salt)                               │
│                        │                                                     │
│                        ▼                                                     │
│                  HKDF-SHA256 (info: "ample-chat-room-key-v1")                │
│                        │                                                     │
│                        ▼                                                     │
│                   room_key (256-bit AES-GCM, shared, RAM-only)               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│                         ENCRYPTED MESSAGE WIRE FRAME                         │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  [User Plaintext Message]                                                    │
│          │                                                                   │
│          ▼                                                                   │
│  EncryptedPayload {                                                          │
│      sender: "alice",                                                        │
│      counter: 0,                   <── Prevents relay message reordering     │
│      timestamp_ms: 1774452000000,                                            │
│      content: "Hello world!"                                                 │
│  }                                                                           │
│          │                                                                   │
│          ▼ AES-256-GCM (Key: room_key, Nonce: 12-byte CSPRNG random)         │
│  WireFrame {                                                                 │
│      ┌───────────────┬──────────────────┬─────────────────────────────────┐  │
│      │ Version (1 B) │ Nonce (12 Bytes) │ Ciphertext + Auth Tag (N+16 B)  │  │
│      │     0x01      │   Random CSPRNG  │ AES-256-GCM Authenticated Data  │  │
│      └───────────────┴──────────────────┴─────────────────────────────────┘  │
│  }                                                                           │
│          │                                                                   │
│          ▼ Base64 Encoded Frame                                              │
│  WebSocket Message ──────► Server Relay ──────► Broadcast to Active Peers    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 🔒 Security Improvements Over `cmd-chat`

| Feature | Original Python `cmd-chat` | Rust `ample-chat` |
|---|---|---|
| **Room Key Derivation** | Pure `HKDF(password, room_salt)` (susceptible to fast GPU brute-force) | **`Argon2id` memory-hard KDF** followed by `HKDF-SHA256` expansion |
| **Symmetric Cipher** | Fernet (AES-128-CBC + HMAC-SHA256) | **AES-256-GCM** (authenticated encryption with 128-bit auth tags) |
| **Replay & Spoofing** | None (relay could replay or spoof sender identity) | **Monotonic counter & verified sender** authenticated inside encrypted payload |
| **Memory Safety** | Python memory management (keys persist in heap garbage) | **`zeroize` & `secrecy`** — keys & passwords zeroed on drop/exit |
| **Unsafe Code** | Dynamic Python runtime | **`#![forbid(unsafe_code)]`** across all workspace crates |
| **Constant-time Checks**| Standard equality | **`subtle` constant-time verification** for zero-knowledge proofs |
| **Credential Hygiene** | Password only via command-line arguments (leaks to `ps` / shell history) | **`AMPLE_CHAT_PASSWORD`**, secure hidden stdin prompt, or CLI flag |
| **Abuse Controls** | Unlimited throughput | **Token-bucket per-connection rate limiting** & strict size bounds |
| **Transport Warnings** | Silent over plaintext HTTP/WS | **Loud runtime warnings** when executing without TLS |

---

## 📦 Installation

### From Prebuilt Release Binaries
Download the pre-compiled binary for your architecture from the [GitHub Releases](https://github.com/ample-chat/ample-chat/releases) page:
- Linux (x86_64, aarch64 - fully static musl)
- macOS (x86_64, Apple Silicon aarch64)
- Windows (x86_64 msvc)

```bash
# Example on Linux
tar -xzf ample-chat-0.1.0-x86_64-unknown-linux-musl.tar.gz
chmod +x ample-chat-0.1.0-x86_64-unknown-linux-musl/ample-chat
sudo mv ample-chat-0.1.0-x86_64-unknown-linux-musl/ample-chat /usr/local/bin/
```

### Via Cargo
```bash
cargo install ample-chat
```

### Via Docker
```bash
docker pull ghcr.io/ample-chat/ample-chat:latest
```

### Building From Source
```bash
git clone https://github.com/ample-chat/ample-chat.git
cd ample-chat
cargo build --release
```

---

## 🚀 Usage

### 1. Host a Chat Room
Start the relay server:
```bash
# Interactive password prompt (recommended - doesn't appear in shell history)
ample-chat serve 0.0.0.0 8000

# Or via environment variable
export AMPLE_CHAT_PASSWORD="my-super-secret-room-password"
ample-chat serve 0.0.0.0 8000

# Or with TLS certificates enabled
ample-chat serve 0.0.0.0 8443 --cert /path/to/cert.pem --key /path/to/key.pem
```

Using Docker:
```bash
docker run -it --rm -p 8000:8000 -e AMPLE_CHAT_PASSWORD="secret" ghcr.io/ample-chat/ample-chat:latest
```

### 2. Connect to a Chat Room
Join an active room:
```bash
# Interactive password prompt
ample-chat connect 127.0.0.1 8000 alice

# Or using TLS / WSS
ample-chat connect mychat.example.com 8443 alice --tls
```

### TUI Controls
- **Type message** and press **Enter** to broadcast.
- **Esc** or **Ctrl+C** to quit immediately and wipe all session keys from RAM.

---

## 🛡️ Threat Model & Limitations

1. **Host Metadata Visibility**: The host running `ample-chat serve` sees client IP addresses, message sizes, and traffic patterns. If anonymity is required, connect through Tor or a trusted VPN.
2. **Endpoint Compromise**: If an attacker compromises your local terminal or has physical access to your machine while `ample-chat` is running, they can inspect memory or keystrokes.
3. **Password Entropy**: Because room keys are derived from the shared room password using Argon2id, use a strong passphrase with high entropy.
4. **Transport Security (TLS)**: When running without `--tls`, eavesdroppers on the local network (e.g. public Wi-Fi) can observe ciphertext and SRP public ephemerals. Always use TLS in untrusted network environments.

---

## 🛡️ CI/CD & Branch Protection

Before merging into `main`, GitHub repository branch protection rules **must** require the following status checks:
1. `Test (ubuntu-latest)`, `Test (macos-latest)`, `Test (windows-latest)` (`ci.yml`)
2. `MSRV Check (Rust 1.75)` (`ci.yml`)
3. `Cargo Audit (RustSec)` (`security.yml`)
4. `Cargo Deny (Licenses, Bans, Advisories, Sources)` (`security.yml`)
5. `Unsafe Code Audit (Cargo Geiger)` (`security.yml`)

---

## 📜 License & Credits

`ample-chat` is licensed under the [MIT License](LICENSE).

This project is a clean-room Rust implementation inspired by and based on [cmd-chat](https://github.com/diorwave/cmd-chat) by [diorwave](https://github.com/diorwave), originally licensed under the MIT License.
