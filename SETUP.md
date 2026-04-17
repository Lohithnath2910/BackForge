# BackForge — Setup Guide

## Prerequisites

### 1. Install Rust (if not already installed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Restart terminal or run:
source $HOME/.cargo/env
```

### 2. Install Python dependencies (for server runner)
```bash
pip install fastapi uvicorn[standard] sqlalchemy passlib[bcrypt] python-jose[cryptography] python-multipart pydantic-settings
```

---

## Build & Run

```bash
cd backforge

# Debug build (faster compile, use during dev)
cargo run

# Release build (faster binary)
cargo build --release
./target/release/backforge

# Pass project name directly
cargo run -- "MyShopAPI"
./target/release/backforge "MyShopAPI"
```

---

## VSCode Setup

1. Install the **rust-analyzer** extension (Extension ID: `rust-lang.rust-analyzer`)
2. Open the `backforge/` folder in VSCode
3. Use the integrated terminal: `cargo run`

### Recommended VSCode settings for this project (`.vscode/settings.json`):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "terminal.integrated.env.linux": {},
  "terminal.integrated.env.osx": {}
}
```

> ⚠️ **Terminal note:** BackForge needs a real terminal (TTY) to run. Use VSCode's integrated terminal (`Ctrl+`` `), NOT the output panel or run button.

---

## Quick Workflow

```
backforge
  │
  ├─ m → Models screen
  │    ├─ n → New model (type name, Enter)
  │    └─ Enter → Model editor
  │               └─ n → Add field (Tab between inputs, ←→ to pick type)
  │
  ├─ e → Endpoints screen
  │    └─ n → New endpoint (path, method, CRUD, model link, body params)
  │
  ├─ a → Auth setup
  │    └─ e → Enable (pick JWT / Session / API Key) → 5 routes auto-added
  │
  ├─ s → Server runner
  │    └─ s → Start uvicorn (streams logs live)
  │         → visit http://localhost:8000/docs in browser
  │
  └─ x → Export
       └─ Enter → Generates full FastAPI project to ./backforge_output/
```

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `cargo: command not found` | Install Rust via rustup |
| `python3: command not found` | Install Python 3.9+ |
| Server says "Missing deps" | `pip install fastapi uvicorn sqlalchemy` |
| Terminal looks garbled | Use a modern terminal (not CMD.exe on Windows — use WSL or Windows Terminal) |
| Colors look wrong | Ensure your terminal supports 24-bit color (most do) |
