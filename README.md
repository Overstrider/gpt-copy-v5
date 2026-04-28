# gpt-copy-v5

PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: c5cfd5995f917f1db1d2984fe4f7493ce68cf323438847f0f1989e595473dc8e
PROJECT_RULES_READ: yes

gpt-copy-v5 is a ChatGPT-style monorepo with a Rust/Axum backend and a Next.js App Router frontend. The backend owns SQLite persistence and all OpenRouter calls, so browser code never receives `OPENROUTER_API_KEY`.

## Requirements

- Rust 1.85+ with Cargo and edition 2024 support
- Node.js 20+ and npm
- An OpenRouter API key for real model calls

## Setup

```powershell
Copy-Item .env.example .env
```

Edit `.env` locally and set `OPENROUTER_API_KEY`. Keep `.env` untracked.

The default non-secret model is:

```text
nvidia/nemotron-3-super-120b-a12b:free
```

## Backend

```powershell
cd backend
cargo run
```

The backend defaults to `http://127.0.0.1:3001` and provides:

- `GET /health`
- `GET /api/conversations`
- `POST /api/conversations`
- `GET /api/conversations/{id}/messages`
- `POST /api/conversations/{id}/messages`
- `POST /api/conversations/{id}/stream`

The API is intended for trusted local development. Startup refuses non-loopback binds unless `ALLOW_UNAUTHENTICATED_PUBLIC_BIND=true` is set explicitly.

Run backend checks:

```powershell
cd backend
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

## Frontend

```powershell
cd frontend
npm install
npm run dev
```

The frontend defaults to `http://localhost:3000` and calls the backend through `NEXT_PUBLIC_API_BASE_URL`, defaulting to `http://localhost:3001`.

Run frontend checks:

```powershell
cd frontend
npm run lint
npm run test
npm run build
npx playwright test
```

## Local Development

Use two terminals:

```powershell
cd backend
cargo run
```

```powershell
cd frontend
npm run dev
```

Open `http://localhost:3000`.

## Troubleshooting

- `OPENROUTER_API_KEY` missing: backend chat endpoints return a structured JSON error for real model calls. Tests use mocks and do not need a key.
- CORS errors: confirm `FRONTEND_ORIGIN=http://localhost:3000` matches the frontend dev server.
- Public bind errors: keep `BACKEND_HOST=127.0.0.1` for local use, or add real authentication before acknowledging `ALLOW_UNAUTHENTICATED_PUBLIC_BIND=true`.
- SQLite errors: confirm the `backend/` directory exists and `DATABASE_URL` points to a writable local SQLite file.
- Frontend API errors: confirm `NEXT_PUBLIC_API_BASE_URL=http://localhost:3001` and the backend is running.
