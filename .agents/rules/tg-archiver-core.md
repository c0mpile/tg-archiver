---
trigger: always_on
---

# tg-archiver — Core Rules

## 1. Workspace Context

`tg-archiver` is a terminal application that mirrors media files (video, audio,
image, archive) from a public Telegram source channel to a topic inside a
private destination group, with parallel downloads, configurable filters, and
full pause/resume support across sessions.
```
~/dev/tg-archiver/              ← project root (NOT inside the repo)
├── .agents/
│   ├── rules/                  # all project rule files
│   └── tasks/                  # task audit trail — ALL agent tasks write here, never inside repo/
└── repo/                       ← git repository root
    ├── src/
    │   ├── main.rs             # entry point — initialises runtime, TUI, app state
    │   ├── app/                # top-level App struct, event loop, state machine
    │   ├── tui/                # ratatui widgets, layout, input handling
    │   ├── telegram/           # grammers-client wrappers (client init, channel/group resolution, forward-as-copy)
    │   ├── archive/            # forward-as-copy worker, chunked message scanning, cursor management
    │   ├── config/             # config struct, .env loading, validation
    │   ├── state/              # persistent state (serde + JSON, saved to XDG state dir)
    │   └── error.rs            # unified error type (anyhow + thiserror)
    ├── Cargo.toml
    ├── Cargo.lock              # always committed
    ├── .env.example            # committed — shows required keys with empty values
    ├── .env                    # never committed — holds real credentials
    └── tests/                  # integration tests

```

**Language:** Rust (stable channel, minimum version pinned in `Cargo.toml` via `rust-version`).
**Build:** `cargo build --release` produces `target/release/tg-archiver`.
**Tests:** `cargo test` (unit + integration); single test: `cargo test <test_name>`.

---

## 2. Development Environment

### First-time setup
```zsh
cd ~/dev/tg-archiver/repo
cp .env.example .env          # then fill in credentials
cargo build                   # pulls all crates, verifies compilation
```

### Required tools

- `rustup` with the **stable** toolchain; `rustfmt` and `clippy` are included.
- No system packages beyond the Rust toolchain are required to build.

### Environment variables (`.env` at repo root)

| Variable | Purpose |
|---|---|
| `TG_API_ID` | Telegram API ID (integer) from my.telegram.org |
| `TG_API_HASH` | Telegram API hash (string) from my.telegram.org |
| `TG_SESSION_FILE` | Absolute path to the `.session` file (e.g. `~/.config/tg-archiver/session`) |
| `TG_PHONE` | Phone number for first-run interactive auth (optional after session exists) |

The app loads `.env` at startup via the `dotenvy` crate. It must **never**
fall back to hard-coded defaults for credential values — if a required variable
is missing, the app must exit with a clear error message naming the missing key.

### Session file

The grammers session file (`TG_SESSION_FILE`) lives outside the repo at the
path set in `.env`. Suggested default: `~/.config/tg-archiver/tg-archiver.session`.
**Never place or commit the session file inside `repo/`.**

---

## 3. Language & Toolchain Rules

### Formatter

`rustfmt` with default settings. Invoke with `cargo fmt`. Apply before every
commit. No `rustfmt.toml` exists by default — if one is created, place it at
`repo/rustfmt.toml` and document non-default settings here.

### Linter

`cargo clippy -- -D warnings`. All clippy warnings are treated as errors.
Place any per-project allows/denies in `repo/.clippy.toml`. Run clippy before
marking any task complete.

### Error handling

Use `anyhow::Result` for application-level error propagation. Use `thiserror`
to define typed domain errors in `src/error.rs` for cases that callers need
to match on (e.g. `FloodWait`, `AuthRequired`, `SessionExpired`). Never use
`Box<dyn std::error::Error>` directly.

### Key crates

| Crate | Purpose |
|---|---|
| `grammers-client` | Telegram MTProto client |
| `grammers-session` | Session persistence backend for grammers |
| `tokio` | Async runtime (`features = ["full"]`) |
| `ratatui` | TUI framework |
| `crossterm` | Terminal backend for ratatui |
| `serde` / `serde_json` | State serialisation |
| `dotenvy` | `.env` loading |
| `anyhow` | Error propagation |
| `thiserror` | Typed error definitions |

**Do not add `tokio-compat`, `async-std`, or any second async runtime.**

### Tests

Unit tests live in `#[cfg(test)]` modules within each source file. Integration
tests live in `repo/tests/`. Run all: `cargo test`. Run one:
`cargo test <test_name> -- --nocapture`. Tests requiring live Telegram
credentials must be gated with `#[ignore]` and documented with
`// requires: TG_API_ID, TG_API_HASH`.

---

## 4. Git & Change Management

Repository is **private**.

### Commit scopes

| Scope | Covers |
|---|---|
| `tui` | ratatui widgets, layout, input |
| `telegram` | grammers client wrappers, flood-wait handling |
| `archive` | Download scheduler, worker pool, upload logic |
| `state` | Persistent state serialisation/deserialisation |
| `config` | Config struct, `.env` loading |
| `app` | App struct, event loop, state machine |
| `error` | Error types |

Examples: `feat(archive): add semaphore-gated parallel download pool`,
`fix(telegram): retry after FloodWait with 2s buffer`.

### Never commit

- `.env`
- `*.session`
- `target/`

All three must be present in `.gitignore`. Verify before any commit touching
root-level files.

---

## 5. Security & Secrets

- `.env` holds `TG_API_ID`, `TG_API_HASH`, `TG_SESSION_FILE`, and optionally
  `TG_PHONE`. This file must never be staged or committed.
- `.env.example` is committed and must contain all required keys with **empty
  values only** (e.g. `TG_API_ID=`).
- The session file grants full account access. It must live outside `repo/`,
  must have permissions `600`, and must never appear in any diff.
- Before any commit, verify `git status` and `git diff --cached` show no
  `.env`, `*.session`, or any literal API hash or API ID value.