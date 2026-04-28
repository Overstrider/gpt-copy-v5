PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: c5cfd5995f917f1db1d2984fe4f7493ce68cf323438847f0f1989e595473dc8e
PROJECT_RULES_READ: yes

# gpt-copy-v5 Domain Map

## Repositories And Stacks
- Root repository: documentation, `.env.example`, shared git hygiene.
- `backend/`: Rust 2024, Axum, Tokio, sqlx SQLite, reqwest, tracing, tower-http CORS.
- `frontend/`: Next.js App Router, TypeScript, Tailwind, lucide-react, TanStack Query, zod, react-markdown, remark-gfm, Vitest, Testing Library, Playwright.

## Boundaries
- Backend owns persistence, validation, structured error responses, OpenRouter credential loading, and provider proxying.
- Frontend owns display state, responsive chat layout, client-side response validation, markdown rendering, and browser tests.
- No provider secrets cross into frontend code or tracked files.

## Specialist Roles
- Backend implementer: API routes, database, OpenRouter adapter, Rust tests.
- Frontend implementer: Next.js app, chat UI, client API layer, component and Playwright tests.
- Documentation integrator: root README, `.env.example`, run commands.
- Review personas: spec, security, saboteur, new-hire maintainability, plus validator/classifier aggregation.

## Data Model
- `conversations`: `id`, `title`, `created_at`, `updated_at`.
- `messages`: `id`, `conversation_id`, `role`, `content`, `created_at`.
- Roles limited to `user`, `assistant`, and `system` where applicable.

## Provider Contract
- Production OpenRouter request uses `Authorization: Bearer <OPENROUTER_API_KEY>` on the backend only.
- Model defaults to `OPENROUTER_MODEL` or `nvidia/nemotron-3-super-120b-a12b:free`.
- Tests use a mock client and never call OpenRouter.
