<p align="center">
  <h1 align="center">🔐 ample-chat</h1>
</p>

<p align="center">
  <b>Encrypted, RAM-only terminal chat.</b><br>
  No disks. No databases. No logs. Zero-knowledge authentication.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-100%25%20Safe-orange?style=for-the-badge&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Encryption-AES--256--GCM-red?style=for-the-badge&logo=letsencrypt" alt="AES-256-GCM">
  <img src="https://img.shields.io/badge/Authentication-SRP--6a-blue?style=for-the-badge" alt="SRP-6a">
  <img src="https://img.shields.io/badge/Memory-RAM--Only-purple?style=for-the-badge" alt="RAM Only">
</p>

<p align="center">
  <img src="https://img.shields.io/github/license/Ibrahist/ample-chat?style=flat-square" alt="License">
  <img src="https://img.shields.io/github/stars/Ibrahist/ample-chat?style=flat-square" alt="Stars">
  <img src="https://img.shields.io/github/forks/Ibrahist/ample-chat?style=flat-square" alt="Forks">
  <img src="https://img.shields.io/github/issues/Ibrahist/ample-chat?style=flat-square" alt="Issues">
</p>

<p align="center">
  <sub>
    This project is an independent Rust implementation inspired by the ideas and
    privacy-focused approach of <b>cmd-chat</b>.
  </sub>
</p>

<p align="center">
  💡 Inspired by
  <a href="https://github.com/diorwave/cmd-chat">
    <b>diorwave/cmd-chat</b>
  </a>
</p>

---

> **Privacy-first • High-performance • End-to-end encrypted • Zero persistence**

`ample-chat` is a high-performance, memory-only terminal chat application written in **100% safe, idiomatic Rust**.

---

## 🛰️ Real Topology & Honest Architecture

The original `cmd-chat` claimed to be "peer-to-peer / no central server". In technical reality, it operated as a relayed client-server architecture where one participant acts as host and other participants connect to that host over HTTP and WebSockets.

`ample-chat` embraces an honest architectural model:

```text
┌─────────────┐                         ┌──────────────────────┐                         ┌─────────────┐
│  Client A   │ ──── E2E Ciphertext ──►│  ample-chat Relay    │─── E2E Ciphertext ────►│  Client B   │
│   (Alice)   │ ◄──── E2E Ciphertext ──│  (In-Memory Server)  │◄── E2E Ciphertext ─────│    (Bob)    │
└─────────────┘                         └──────────────────────┘                         └─────────────┘
      │                                           │                                           │
      ▼                                           ▼                                           ▼
 Room Key Derived                         Stores RAM Ciphertext                       Room Key Derived
 Argon2id + HKDF                          CANNOT Decrypt Messages                     Argon2id + HKDF
```

### 👁️ What the Host / Relay Server CAN See

- 🌐 IP addresses and connection timestamps of connected clients.
- 👥 Total count and usernames of currently connected peers.
- 📡 Transport-level metadata (WebSocket frame sizes, transmission frequency).
- 🔐 Raw encrypted ciphertext frames and nonces.

### 🚫 What the Host / Relay Server CANNOT See

- 🔑 **Your password:** Authentication uses SRP-6a (Secure Remote Password). Passwords never cross the network.
- 💬 **Your plaintext messages:** Messages are encrypted end-to-end on clients prior to sending.
- 🔐 **Your room encryption key:** The room key is derived on clients via Argon2id + HKDF using the password; the server never derives or receives it.
- 🗃️ **Historical chat logs:** Nothing is written to disk or database. All state vanishes when the server process ends.

---

## 🌳 How It Works

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                            🔑 SRP-6a AUTHENTICATION                          │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   CLIENT (Alice)                                    SERVER (Relay)           │
│      │                                                     │                 │
│      │──────────── POST /srp/init {username, A} ──────────►│                 │
│      │             (A = client public ephemeral g^a)       │                 │
│      │                                                     │                 │
│      │◄─────────── {user_id, B, salt, room_salt} ─────────│                 │
│      │             (B = server public ephemeral kv + g^b) │                 │
│      │             (room_salt = random 16 bytes)          │                 │
│      │                                                     │                 │
│      │  [client derives room_key via Argon2id + HKDF]      │                 │
│      │                                                     │                 │
│      │──────────── POST /srp/verify {user_id, M} ─────────►│                 │
│      │             (M = client proof H(A || B || K))       │                 │
│      │                                                     │                 │
│      │◄─────────── {H_AMK, session_key} ──────────────────│                 │
│      │                                                     │                 │
│      │       🤝 Mutual authentication complete             │                 │
│      │                                                     │                 │
├──────────────────────────────────────────────────────────────────────────────┤
│                     🔒 E2E ENCRYPTED WEBSOCKET CHAT                         │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│      │════════════ WebSocket /ws/chat?user_id ═══════════►│                 │
│      │                                                     │                 │
│      │      🔐 Encrypt(Payload, room_key)                  │                 │
│      │────────────────────────────────────────────────────►│                 │
│      │                                                     │                 │
│      │◄──────────── Broadcast raw ciphertext ──────────────│                 │
│      │                                                     │                 │
│      │      🔓 Decrypt(Ciphertext, room_key)               │                 │
│      │                                                     │                 │
│      │      [server holds ONLY ciphertext]                 │                 │
│      │      [keys & plaintext zeroized on disconnect]      │                 │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 🔑 Key Hierarchy

```text
                         🔑 password
                              │
                 ┌────────────┴────────────┐
                 │                         │
                 ▼                         ▼
             SRP-6a                    Argon2id
          RFC 5054 2048-bit          password + room_salt
                 │                         │
                 ▼                         ▼
          session_key              HKDF-SHA256
       per-client authentication         │
                                           ▼
                                   🔐 room_key
                                           │
                                           ▼
                                  AES-256-GCM
                                   256-bit key
```

### 📦 Encrypted Message Wire Frame

```text
┌─────────────────────────────────────────────────────────────────┐
│                     USER PLAINTEXT MESSAGE                      │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  EncryptedPayload    │
                    │                      │
                    │  sender              │
                    │  counter             │
                    │  timestamp_ms        │
                    │  content             │
                    └──────────┬───────────┘
                               │
                               ▼
                        🔐 AES-256-GCM
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Version │       Nonce        │ Ciphertext + Authentication Tag │
│  1 byte │     12 bytes       │              N + 16 bytes       │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
                         Base64 Frame
                               │
                               ▼
                    WebSocket Message
                               │
                               ▼
                       🛰️ Relay Server
                               │
                               ▼
                     Broadcast to Peers
```

---

## 🔒 Security Improvements Over `cmd-chat`

| 🔐 Feature              | 🐍 Original Python `cmd-chat`      | 🦀 Rust `ample-chat`                |
| ----------------------- | ---------------------------------- | ----------------------------------- |
| 🔑 Room Key Derivation  | Pure `HKDF(password, room_salt)`   | `Argon2id` + `HKDF-SHA256`          |
| 🔐 Symmetric Cipher     | Fernet (AES-128-CBC + HMAC-SHA256) | AES-256-GCM                         |
| 🛡️ Replay & Spoofing    | None                               | Monotonic counter + verified sender |
| 🧠 Memory Safety        | Python memory management           | `zeroize` + `secrecy`               |
| 🚫 Unsafe Code          | Dynamic Python runtime             | `#![forbid(unsafe_code)]`           |
| ⏱️ Constant-time Checks | Standard equality                  | `subtle` constant-time verification |
| 🔑 Credential Hygiene   | CLI password exposure              | Environment variable / hidden stdin |
| 🚦 Abuse Controls       | Unlimited throughput               | Token-bucket rate limiting          |
| 🔒 Transport Warnings   | Silent plaintext operation         | Loud runtime warnings without TLS   |

---

## 📦 Installation

### 📥 From Prebuilt Release Binaries

Download the pre-compiled binary for your architecture from the **GitHub Releases** page:

- 🐧 Linux — x86_64, aarch64
- 🍎 macOS — x86_64, Apple Silicon aarch64
- 🪟 Windows — x86_64 msvc

```bash
tar -xzf ample-chat-0.1.0-x86_64-unknown-linux-musl.tar.gz

chmod +x ample-chat-0.1.0-x86_64-unknown-linux-musl/ample-chat

sudo mv ample-chat-0.1.0-x86_64-unknown-linux-musl/ample-chat /usr/local/bin/
```

### 🦀 Via Cargo

```bash
cargo install ample-chat
```

### 🐳 Via Docker

```bash
docker pull ghcr.io/ample-chat/ample-chat:latest
```

### 🛠️ Building From Source

```bash
git clone https://github.com/ample-chat/ample-chat.git
cd ample-chat
cargo build --release
```

---

## 🚀 Usage

### 1️⃣ Host a Chat Room

Start the relay server:

```bash
# Interactive password prompt
# Recommended — doesn't appear in shell history
ample-chat serve 0.0.0.0 8000

# Or via environment variable
export AMPLE_CHAT_PASSWORD="my-super-secret-room-password"
ample-chat serve 0.0.0.0 8000

# Or with TLS certificates enabled
ample-chat serve 0.0.0.0 8443 \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem
```

### 🐳 Using Docker

```bash
docker run -it --rm \
  -p 8000:8000 \
  -e AMPLE_CHAT_PASSWORD="secret" \
  ghcr.io/ample-chat/ample-chat:latest
```

### 2️⃣ Connect to a Chat Room

Join an active room:

```bash
# Interactive password prompt
ample-chat connect 127.0.0.1 8000 alice

# Or using TLS / WSS
ample-chat connect mychat.example.com 8443 alice --tls
```

### 🎮 TUI Controls

| Key      | Action                                 |
| -------- | -------------------------------------- |
| `Enter`  | 💬 Broadcast message                   |
| `Esc`    | 🚪 Quit immediately                    |
| `Ctrl+C` | 🧹 Quit and wipe session keys from RAM |

---

## 🛡️ Threat Model & Limitations

### 🌐 1. Host Metadata Visibility

The host running `ample-chat serve` sees client IP addresses, message sizes, and traffic patterns.

If anonymity is required, connect through **Tor or a trusted VPN**.

### 💻 2. Endpoint Compromise

If an attacker compromises your local terminal or has physical access to your machine while `ample-chat` is running, they can inspect memory or keystrokes.

### 🔑 3. Password Entropy

Because room keys are derived from the shared room password using Argon2id, use a strong passphrase with high entropy.

### 🔒 4. Transport Security (TLS)

When running without `--tls`, eavesdroppers on the local network (e.g. public Wi-Fi) can observe ciphertext and SRP public ephemerals.

**Always use TLS in untrusted network environments.**

---

## 🛡️ CI/CD & Branch Protection

Before merging into `main`, GitHub repository branch protection rules must require the following status checks:

| ✅ Check                         | 🔧 Workflow    |
| -------------------------------- | -------------- |
| Test — `ubuntu-latest`           | `ci.yml`       |
| Test — `macos-latest`            | `ci.yml`       |
| Test — `windows-latest`          | `ci.yml`       |
| MSRV Check — Rust 1.75           | `ci.yml`       |
| Cargo Audit — RustSec            | `security.yml` |
| Cargo Deny                       | `security.yml` |
| Unsafe Code Audit — Cargo Geiger | `security.yml` |

---

## 📊 Security Stack

```text
                    🛡️ ample-chat
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
     🔐 Auth          🔑 Keys          💬 Chat
        │                │                │
      SRP-6a          Argon2id        AES-256-GCM
        │                │                │
        ▼                ▼                ▼
   Zero-Knowledge    HKDF-SHA256     E2E Encryption
        │                │                │
        └────────────────┼────────────────┘
                         │
                         ▼
                    🧠 RAM ONLY
                         │
                         ▼
                  🗑️ ZERO PERSISTENCE
```
---

## 📜 License & Credits

`ample-chat` is licensed under the **MIT License**.

This project is an independent Rust implementation inspired by and based on the ideas of [`cmd-chat`](https://github.com/diorwave/cmd-chat) by **diorwave**, originally licensed under the MIT License.

---

<p align="center">
  🔐 <b>Privacy-first communication.</b><br>
  ⚡ <b>Fast. Secure. Memory-only.</b><br>
  🦀 <b>Built with Rust.</b>
</p>

<p align="center">
  <sub>ample-chat — encrypted terminal communication without persistent storage.</sub>
</p>
