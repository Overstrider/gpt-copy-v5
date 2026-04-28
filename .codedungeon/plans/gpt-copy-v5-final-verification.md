PROJECT_RULES_STATUS: approved
PROJECT_RULES_DIGEST: 3ae6ef49673724749fb1b0e05bccedc6da1cdeee8c0694d95384ec2b72b91b15
PROJECT_RULES_READ: yes

# Final Verification Set

- `cd backend; cargo fmt --check`
- `cd backend; cargo test`
- `cd backend; cargo clippy --all-targets -- -D warnings`
- `cd frontend; npm run lint`
- `cd frontend; npm run test`
- `cd frontend; npm run build`
- `cd frontend; npx playwright test`
- `git status --short`
- `./.codex/bin/codedungeon rules status --human`
- `./.codex/bin/codedungeon rules lint --human`
- `./.codex/bin/codedungeon git verify --human`

Notes:
- Frontend Playwright test mocks backend network calls and does not require OpenRouter credentials.
- Backend tests use an injected mock OpenRouter client and do not require live provider calls.
