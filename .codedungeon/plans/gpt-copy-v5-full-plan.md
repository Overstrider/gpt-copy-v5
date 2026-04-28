PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: c5cfd5995f917f1db1d2984fe4f7493ce68cf323438847f0f1989e595473dc8e
PROJECT_RULES_READ: yes

# gpt-copy-v5 Full Plan

## Scope
- Create a monorepo with `backend/` Rust 2024 Axum API and `frontend/` Next.js App Router UI.
- Keep OpenRouter access server-side in the backend using `OPENROUTER_API_KEY` and `OPENROUTER_MODEL`.
- Persist conversations and messages in SQLite through `sqlx`.
- Provide docs, placeholder-only env examples, focused tests, and CodeDungeon verification evidence.

## Backend Architecture
- `backend/src/main.rs`: process entrypoint, tracing, env loading, Axum listener.
- `backend/src/lib.rs`: app factory and shared state for tests.
- `backend/src/config.rs`: non-secret defaults including `nvidia/nemotron-3-super-120b-a12b:free`.
- `backend/src/db.rs`: SQLite pool setup, schema creation, conversation/message queries.
- `backend/src/error.rs`: structured JSON errors and status mapping.
- `backend/src/models.rs`: request/response DTOs with validation helpers.
- `backend/src/openrouter.rs`: trait-backed OpenRouter client, production HTTP client, test mock.
- `backend/src/routes.rs`: `/health`, conversation CRUD/read, send chat, stream chat.

## Frontend Architecture
- Next.js App Router TypeScript app in `frontend/`.
- Client-side chat workspace using TanStack Query for conversations/messages and streaming `fetch` for chat streaming.
- Safe assistant markdown rendering with `react-markdown` and `remark-gfm`.
- Tailwind responsive layout with desktop sidebar and mobile drawer.
- Zod schemas for backend API responses and structured errors.

## API Surface
- `GET /health`
- `GET /api/conversations`
- `POST /api/conversations`
- `GET /api/conversations/:id/messages`
- `POST /api/conversations/:id/messages`
- `POST /api/conversations/:id/stream`

## Verification Targets
- Backend: `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- Frontend: `npm run lint`, `npm run test`, `npm run build`, `npx playwright test`.
- Root docs: README commands match generated project scripts.

## Risks
- Streaming behavior must not expose OpenRouter credentials or require real provider calls in tests.
- SQLite schema setup must be deterministic for in-memory test databases.
- Playwright smoke test should mock backend endpoints to avoid requiring backend or OpenRouter secrets.
