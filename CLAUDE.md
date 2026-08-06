# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

**Development (Tauri + Axum + Vue):**
```bash
pnpm install              # Install dependencies
pnpm tauri dev            # Run dev server with hot reload (opens desktop window)
```

**Frontend-only:**
```bash
pnpm dev                  # Vite frontend only (browser preview)
pnpm build                # Type-check (vue-tsc) + build frontend
pnpm preview              # Preview built frontend
```

**Rust:**
```bash
cd src-tauri
cargo check               # Type-check Rust code
cargo test                # Run migration + API contract tests (no window opened)
```

**Production Build:**
```bash
pnpm tauri build          # Full release build (frontend + bundled app)
```

## Architecture Overview

Single-process desktop application combining:

- **Tauri 2**: native window + Rust runtime
- **Axum**: embedded HTTP server running alongside Tauri
- **Vue 3 + TypeScript**: frontend UI (Pinia + Vue Router + Element Plus)
- **SQLite + SQLx**: local persistent storage (WAL mode, `sqlx::migrate!`)

### Data Flow

```
Vue → stores (Pinia) → src/api (axios) → Tauri invoke(get_api_port)
        ↓                                          ↓
   HTTP (axios)                           Axum embedded server
        ↓                                          ↓
   /api/*  ⇠ { code, message, data } ⇢  handlers → db repos → SQLite
```

Key insight: Axum binds to a random local port (`127.0.0.1:0`), Tauri captures the port as state, and Vue resolves it via `invoke("get_api_port")` to build the API base URL. No hardcoded ports, no CORS issues (both run on localhost).

### API Contract (uniform envelope)

- Success: `200 OK` → `{ "code": 0, "message": "ok", "data": ... }`
- Error: `4xx/5xx` → `{ "code": <http status>, "message": "..." }`
- DELETE success → `204 No Content`
- The axios response interceptor unwraps `data` and converts errors to `ApiError`.

### Timezone Convention (IMPORTANT)

- Store & transmit **UTC only**, RFC 3339 with `Z` (e.g. `2026-08-06T07:30:00.000Z`).
- Backend generates timestamps with `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)`.
- SQLite column default: `strftime('%Y-%m-%dT%H:%M:%fZ','now')` (see migration 003).
- Frontend converts to local via `dayjs(...).local().format(...)` in `src/utils/format.ts`.
- Never do local-timezone math on the backend.

### Key Files

| Layer | File | Responsibility |
|-------|------|----------------|
| Frontend | `src/main.ts` | App entry: mounts Pinia, Router, Element Plus, icons |
| Frontend | `src/App.vue` | Shell layout + nav menu + `<router-view>` |
| Frontend | `src/router/index.ts` | Hash-mode routes, lazy-loaded views |
| Frontend | `src/stores/tasks.ts`, `src/stores/notes.ts` | Pinia stores: all list/CRUD business logic |
| Frontend | `src/api/http.ts` | axios instance, port discovery, envelope unwrap, `ApiError` |
| Frontend | `src/api/tasks.ts`, `src/api/notes.ts` | Per-resource API calls |
| Frontend | `src/types/` | Domain types mirroring the Rust contract |
| Frontend | `src/components/` | TaskForm / TaskList / NotesPanel (thin, call stores) |
| Frontend | `src/views/` | Routed pages (TasksView / AboutView) |
| Backend (App) | `src-tauri/src/lib.rs` | Boot: tracing → config → DB → Axum → Tauri |
| Backend (Config) | `src-tauri/src/config.rs` | `APP_*` env vars with defaults |
| Backend (Errors) | `src-tauri/src/error.rs` | `AppError` (thiserror), infra-level |
| Backend (API) | `src-tauri/src/api/mod.rs` | `AppState { db, config }` |
| Backend (API) | `src-tauri/src/api/response.rs` | `ApiResponse` envelope, `ApiResult` |
| Backend (API) | `src-tauri/src/api/error.rs` | `ApiError` → uniform error envelope |
| Backend (API) | `src-tauri/src/api/extract.rs` | `ValidatedJson` extractor (validator crate) |
| Backend (API) | `src-tauri/src/api/routes.rs` | Router + middleware (CORS / Timeout / Trace) |
| Backend (API) | `src-tauri/src/api/handlers/` | Feature-scoped handlers (health/tasks/notes) |
| Backend (DB) | `src-tauri/src/db/mod.rs` | Pool init (WAL) + `sqlx::migrate!` |
| Backend (DB) | `src-tauri/src/db/tasks.rs`, `db/notes.rs` | Repositories: all SQL lives here |
| Backend (Models) | `src-tauri/src/models/` | DTOs with `validator` annotations |
| Migrations | `src-tauri/migrations/` | Versioned SQL, tracked by `_sqlx_migrations` |
| Config | `package.json`, `vite.config.ts` | Frontend scripts, `@` alias, vendor chunks |
| Config | `src-tauri/Cargo.toml`, `tauri.conf.json` | Rust & Tauri config |

### Important Patterns

- **Layering**: handlers → repositories → SQLite. Handlers never write SQL; repos never build HTTP responses.
- **CORS**: `tower_http::cors` allows any origin — safe, everything binds to `127.0.0.1` only.
- **Request logging**: `TraceLayer` records method, URI, status, latency per request.
- **Timeouts**: `TimeoutLayer` applies `APP_REQUEST_TIMEOUT_SECS` to every request.
- **Validation**: DTOs derive `validator::Validate`; `ValidatedJson` returns 400 on violations.
- **Port resolution**: frontend caches the `get_api_port` promise to avoid repeated Tauri calls.
- **Idempotency**: UUID primary keys; SQLite `RETURNING` keeps read-back consistent with write.
- **Migrations**: `sqlx::migrate!` embeds `migrations/`; already-applied scripts are skipped.

### SQLite Database

- File: `./data.db` (WAL journal mode), created relative to launch dir.
- Tables: `tasks(id, title, completed, created_at)`, `notes(id, task_id, content, created_at)`.
- Production note: move the DB path to Tauri's app-data dir via `APP_DATABASE_URL`.
