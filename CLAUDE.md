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

The whole codebase (frontend and backend) follows a **diamond (hexagonal) architecture**:
a domain core (`domain` + `application`) in the middle, a **north gateway** (`north/`)
for inputs and a **south gateway** (`south/`) for outputs, plus `shared/` for
cross-cutting concerns.

### Diamond (north-south) Layer Mapping

| Layer | Rust (`src-tauri/src/`) | Frontend (`src/`) | Responsibility |
|-------|--------------------------|-------------------|----------------|
| **domain** (core) | `domain/` — entities (`Task`/`Note` with `new`/`update` invariants), **south ports** `TaskRepository`/`NoteRepository` traits, `TaskQuery`/`TaskList`, `DomainError`, `RepoError` | `domain/` — entity interfaces, **south ports** repository interfaces, `validateTaskTitle`/`validateNoteContent` | Business invariants; no framework/HTTP/SQL knowledge |
| **application** (core) | `application/` — use-case services + **north ports** `TaskUseCase`/`NoteUseCase` (`ports.rs`), DTOs, `ServiceError` | `application/` — Pinia store factories `createTasksStore(repo)`/`createNotesStore(repo)` | Use-case orchestration; depends only on domain south ports via DI, exposes north ports |
| **south** (gateway) | `south/` — sqlx pool + migration, `SqlxTaskRepository`/`SqlxNoteRepository` | `south/` — axios (`http.ts`), `HttpTaskRepository`/`HttpNoteRepository` | Implements domain south ports; all SQL / HTTP details live here |
| **north** (gateway) | `north/` — axum handlers, routes, middleware, `ApiError`/`ApiResponse`/`JsonBody`, `AppState` (holds `Arc<dyn TaskUseCase>`/`Arc<dyn NoteUseCase>`) | `north/` — router, `App.vue`, views, components | HTTP / UI translation only; depends only on application north ports |
| **shared** | `shared/` — `config.rs`, `time.rs`, `error.rs` | `shared/` — `di.ts` (frontend composition root), `format.ts` | Cross-cutting concerns any layer may use |

Key conventions:

- **Composition roots**: backend `src-tauri/src/lib.rs`, frontend `src/shared/di.ts` — they
  wire concrete south-gateway implementations into the domain ports consumed by the
  application layer, and expose the resulting services behind north ports.
- **North port**: `application::ports::TaskUseCase`/`NoteUseCase` — the north gateway
  (`north/`) depends only on these interfaces, never on concrete services.
- **Dependency direction**: `north → application → domain`; `south` implements `domain`
  ports; `shared` is referenced by any layer but never depends downward on business
  layers.
- **No `validator` crate**: validation lives in the domain entities (`Task::new`,
  `Task::update`, `Note::new`, `Note::update`) via `try_*`-style constructors/actions.
  Frontend mirrors the same rules with `validateTaskTitle`/`validateNoteContent`.

### Data Flow

```
Vue 组件 → north → application (Pinia store) → domain 南向端口 (仓储接口)
                                                    ↓ 组合根注入
                                         south (axios) → /api
                                                    ↓
后端: north (axum handler) → application (service) → domain 南向端口 (trait)
                                                    ↓ 组合根注入
                                         south (sqlx) → SQLite
```

Key insight: Axum binds to a random local port (`127.0.0.1:0`), Tauri captures the port as state, and Vue resolves it via `invoke("get_api_port")` to build the API base URL. No hardcoded ports, no CORS issues (both run on localhost).

### API Contract (uniform envelope)

- Success: `200 OK` → `{ "code": 0, "message": "ok", "data": ... }`
- Error: `4xx/5xx` → `{ "code": <http status>, "message": "..." }`
- DELETE success → `204 No Content`
- The axios response interceptor unwraps `data` and converts errors to `ApiError`.
- JSON body parse failures are mapped to `400` + envelope by the `JsonBody` extractor.

### Timezone Convention (IMPORTANT)

- Store & transmit **UTC only**, RFC 3339 with `Z` (e.g. `2026-08-06T07:30:00.000Z`).
- Entities generate timestamps via `shared::time::utc_now_rfc3339()`
  (`chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)`).
- SQLite column default: `strftime('%Y-%m-%dT%H:%M:%fZ','now')` (see migration 003).
- Frontend converts to local via `dayjs(...).local().format(...)` in `src/shared/format.ts`.
- Never do local-timezone math on the backend.

### Key Files

| Layer | File | Responsibility |
|-------|------|----------------|
| Frontend | `src/main.ts` | App entry: mounts Pinia, Router, Element Plus, icons |
| Frontend | `src/shared/di.ts` | Frontend composition root: injects HTTP repos into store factories |
| Frontend | `src/shared/format.ts` | UTC → local time formatting (dayjs) |
| Frontend | `src/domain/` | Entities (`task.ts`/`note.ts` + validators), repository interfaces (`repository.ts`) |
| Frontend | `src/application/tasks.ts`, `notes.ts` | Pinia store factories: all list/CRUD use-case logic |
| Frontend | `src/south/http.ts` | axios instance, port discovery, envelope unwrap, `ApiError` |
| Frontend | `src/south/task-repository.ts`, `note-repository.ts` | HTTP adapters implementing domain ports |
| Frontend | `src/north/` | Router, `App.vue`, views, components (thin, call stores) |
| Backend (Composition) | `src-tauri/src/lib.rs` | Boot: tracing → config → DB → wire repos/services → Axum → Tauri |
| Backend (Shared) | `src-tauri/src/shared/` | `config.rs` (`APP_*` env), `time.rs` (UTC RFC3339), `error.rs` (`AppError`) |
| Backend (Domain) | `src-tauri/src/domain/` | Entities (`task.rs`/`note.rs`), south ports (`repository.rs`) + `TaskQuery`/`TaskList`, `DomainError`/`RepoError` |
| Backend (Application) | `src-tauri/src/application/` | `TaskService`/`NoteService` (use cases), north ports (`ports.rs`), DTOs, `ServiceError` |
| Backend (South) | `src-tauri/src/south/db/` | Pool init (WAL) + `sqlx::migrate!`, `SqlxTaskRepository`/`SqlxNoteRepository` |
| Backend (North) | `src-tauri/src/north/` | `AppState { tasks: Arc<dyn TaskUseCase>, notes: Arc<dyn NoteUseCase>, config }`, handlers, routes, `ApiError`/`ApiResponse`/`JsonBody` |
| Migrations | `src-tauri/migrations/` | Versioned SQL, tracked by `_sqlx_migrations` |
| Config | `package.json`, `vite.config.ts` | Frontend scripts, `@` alias, vendor chunks |
| Config | `src-tauri/Cargo.toml`, `tauri.conf.json` | Rust & Tauri config |

### Important Patterns

- **Layering**: north → application → domain, with south implementing domain ports. Handlers never write SQL; repos never build HTTP responses; entities hold invariants.
- **DI**: services hold `Arc<dyn TaskRepository>` / `Arc<dyn NoteRepository>` (south) trait objects; the north gateway holds `Arc<dyn TaskUseCase>` / `Arc<dyn NoteUseCase>` (north). Tests inject the same sqlx adapters against a temp DB.
- **Validation**: domain entities enforce invariants in `new`/`update`; `JsonBody` maps JSON parse failures to 400 + envelope.
- **CORS**: `tower_http::cors` allows any origin — safe, everything binds to `127.0.0.1` only.
- **Request logging**: `TraceLayer` records method, URI, status, latency per request.
- **Timeouts**: `TimeoutLayer` applies `APP_REQUEST_TIMEOUT_SECS` to every request.
- **Port resolution**: frontend caches the `get_api_port` promise to avoid repeated Tauri calls.
- **Idempotency**: entities generate UUID ids at construction; SQLite `RETURNING`-style read-back is replaced by constructing the entity up front and persisting it (full-row write).
- **Migrations**: `sqlx::migrate!` embeds `migrations/`; already-applied scripts are skipped.

### SQLite Database

- File: `./data.db` (WAL journal mode), created relative to launch dir.
- Tables: `tasks(id, title, completed, created_at)`, `notes(id, task_id, content, created_at)`.
- Production note: move the DB path to Tauri's app-data dir via `APP_DATABASE_URL`.
