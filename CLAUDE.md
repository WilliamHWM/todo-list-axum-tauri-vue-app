# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

**Development (Tauri + Axum + Vue):**
```bash
pnpm install              # Install dependencies
pnpm tauri dev            # Run dev server with hot reload
```

**Frontend-only:**
```bash
pnpm dev                  # Vite frontend only
pnpm build                # Build frontend (run before Tauri production build)
pnpm preview              # Preview built frontend
```

**Rust:**
```bash
cd src-tauri
cargo check               # Check Rust code
cargo run                 # Directly run Axum + Tauri without Tauri CLI
```

**Production Build:**
```bash
pnpm tauri build          # Full release build (frontend + bundled app)
```

## Architecture Overview

This is a single-process desktop application combining:

- **Tauri**: Electron alternative providing the native window and Rust runtime
- **Axum**: Embedded HTTP server running alongside Tauri
- **Vue 3**: Frontend UI with TypeScript
- **SQLite + SQLx**: Local persistent storage

### Data Flow

```
Vue → src/api/tasks.ts → Tauri invoke(get_api_port)
        ↓                                    ↓
   HTTP fetch                      ← Axum embedded server
        ↓                                    ↓
   /api/tasks                        routes.rs → db.rs → SQLite
```

The key insight is that Axum binds to a random local port (`127.0.0.1:0`), Tauri captures the port and stores it as state, and Vue queries that port via `tauri::invoke` to construct the API URL dynamically. This avoids hardcoding ports and avoids CORS issues since both run on localhost.

### Key Files

| Layer | File | Responsibility |
|-------|------|----------------|
| Frontend | `src/App.vue` | Root component composition |
| Frontend | `src/components/TaskList.vue` | Todo list UI, filtering, editing |
| Frontend | `src/api/tasks.ts` | Fetch wrapper + Tauri port resolution |
| Frontend | `src/main.ts` | Vue app entry |
| Backend (API) | `src-tauri/src/routes.rs` | Axum HTTP router, validation, error handling |
| Backend (DB) | `src-tauri/src/db.rs` | SQLx database operations |
| Backend (Models) | `src-tauri/src/models.rs` | Request/response schemas |
| Backend (App) | `src-tauri/src/lib.rs` | Entry point: starts Axum + Tauri together |
| Backend (CLI) | `src-tauri/src/main.rs` | Calls `lib.rs::run()` |
| Config | `package.json`, `vite.config.ts` | Vite & frontend scripts |
| Config | `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` | Rust & Tauri config |

### Important Patterns

- **CORS**: The Axum router uses `tower_http::cors` allowing any origin — safe since everything runs locally.
- **Port resolution**: Frontend caches the result of `get_api_port` invocation to avoid repeated Tauri calls.
- **Idempotency**: Tasks use UUIDs for primary keys; SQLite `RETURNING` clauses ensure consistent timestamps.
- **Error handling**: Centralized `ApiError` type in `routes.rs` returns structured JSON errors to the frontend.
- **Migration schema**: `src-tauri/migrations/001_init.sql` creates the `tasks` table at DB initialization.

### SQLite Database

- File location: `./data.db` (relative to working directory on launch)
- Table structure: `tasks(id, title, completed, created_at)`
- Production note: In a real app, the DB path should be moved to Tauri's app data directory instead of working directory.
