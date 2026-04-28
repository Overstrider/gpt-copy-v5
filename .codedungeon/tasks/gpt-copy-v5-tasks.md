PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: c5cfd5995f917f1db1d2984fe4f7493ce68cf323438847f0f1989e595473dc8e
PROJECT_RULES_READ: yes

# gpt-copy-v5 Tasks

## Task 1: Backend Scaffold And Core API
- Create `backend/Cargo.toml`, Rust 2024 crate, source modules, and tests.
- Implement config, app state, tracing entrypoint, CORS, health route, structured errors.
- Verification: `cd backend && cargo fmt --check`, `cd backend && cargo test`.

## Task 2: Backend Persistence And OpenRouter Proxy
- Add SQLite schema setup and query helpers through `sqlx`.
- Add conversations/messages endpoints, send-message endpoint, streaming endpoint.
- Add trait-backed OpenRouter client with production reqwest implementation and mocked test behavior.
- Verification: backend validation, persistence, and mocked OpenRouter tests pass.

## Task 3: Frontend Scaffold And Chat UI
- Create `frontend/` Next.js App Router TypeScript/Tailwind project using npm.
- Build responsive ChatGPT-style shell with sidebar, transcript, composer, mobile navigation, loading and error states.
- Add lucide icons, zod API parsing, TanStack Query data flow, safe markdown rendering.
- Verification: `cd frontend && npm run lint`, `cd frontend && npm run test`.

## Task 4: Frontend E2E And Docs
- Add Playwright smoke test that mocks backend and sends one message.
- Add root `README.md` and placeholder-only `.env.example`.
- Verification: `cd frontend && npm run build`, `cd frontend && npx playwright test`.

## Task 5: Integration Cleanup
- Run formatting/lint/build/test commands through CodeDungeon QA.
- Refresh project rules only if CodeDungeon marks them stale because new app files exist.
- Generate review evidence, resolve accepted findings, push branch, open PR, post review, finalize.
