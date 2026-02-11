# Repository Guidelines

## Project Structure & Module Organization
- This repository is currently planning-first and contains two core docs:
  - `plan.md`: target architecture, API contracts, reliability rules, and deployment model.
  - `steps.md`: stepwise delivery plan (Step-01 to Step-14), Definition of Done, and iteration rules.
- When implementation begins, keep modules aligned with the plan:
  - `frontend/` for React + Vite + Tailwind UI.
  - `backend/` for Axum services (`api`, `rag`, `provider`, `indexer`, `config`, `observability`).
  - `deploy/`, `tests/`, `scripts/`, and `docs/` for runtime, validation, automation, and runbooks.

## Build, Test, and Development Commands
- No runnable application scaffold exists in the current snapshot; contributions are document-centric.
- Useful commands now:
  - `rg --files` to inspect repository contents quickly.
  - `sed -n '1,200p' plan.md` and `sed -n '1,220p' steps.md` to review context before edits.
  - `wc -w AGENTS.md plan.md steps.md` to keep documentation concise.
- Runtime constraint: do not rely on Docker/Docker Compose for implementation or validation unless explicitly requested; run services directly on the host by default.
- After Step-01 scaffolding, standardize host-run commands in `README` (for example `cargo run`, `npm run dev`).

## Coding Style & Naming Conventions
- Markdown conventions: ATX headings (`##`), short sections, and actionable bullet points.
- Write requirements as testable statements (clear inputs, outputs, and failure behavior).
- Naming rules from `steps.md`:
  - Branch: `feature/engineqa-rag-mvp`.
  - Commit: `feat(step-XX): <module capability>`.
- For future code modules, use formatter defaults (`rustfmt` for Rust, project formatter/linter for frontend) to avoid style drift.

## Testing Guidelines
- Every step must be runnable, verifiable, and reversible (`git revert`).
- Include a 5-10 minute smoke check for each completed step.
- Add at least one automated test or reproducible script per delivered module.
- Use Step-13 as the acceptance baseline: functional tests, load tests, and security checks.

## Commit & Pull Request Guidelines
- The current workspace does not expose `.git` history; follow `steps.md` conventions as the canonical standard.
- PRs should include step number, scope, validation evidence, and rollback notes.
- If behavior or configuration changes, document affected env vars and API fields in the PR description.

## Security & Configuration Tips
- Never commit secrets (for example `INTERNAL_API_TOKEN`); keep them in environment variables.
- Ensure logs and shared artifacts do not leak tokens or sensitive request payloads.
