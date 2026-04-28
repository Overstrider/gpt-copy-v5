PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: c5cfd5995f917f1db1d2984fe4f7493ce68cf323438847f0f1989e595473dc8e
PROJECT_RULES_READ: yes

# gpt-copy-v5 QA Plan

## Backend Focus
- Health route returns JSON `ok: true`.
- Invalid conversation/message payloads return structured JSON errors and 400/422-class statuses.
- SQLite persistence creates conversations, stores user/assistant messages, and reloads them in stable order.
- Mock OpenRouter client proves send-chat saves user input plus mocked assistant output without network calls.
- Streaming route validates payloads and emits assistant content without exposing secrets.

## Frontend Focus
- Main chat shell renders sidebar, transcript, composer, loading and error states.
- Sending a message posts to the API and renders assistant response.
- API client validates zod schemas and reports structured backend errors.
- Playwright smoke test mocks backend routes, sends one message, and confirms transcript update.

## Failure Modes To Guard
- Missing `OPENROUTER_API_KEY` should not break tests that use mock clients.
- `.env.example` must contain placeholders only.
- Frontend build must not depend on a running backend.
- Mobile layout must expose conversation navigation through a drawer/menu.

## Verification Commands
- `cd backend && cargo fmt --check`
- `cd backend && cargo test`
- `cd backend && cargo clippy --all-targets -- -D warnings`
- `cd frontend && npm run lint`
- `cd frontend && npm run test`
- `cd frontend && npm run build`
- `cd frontend && npx playwright test`
