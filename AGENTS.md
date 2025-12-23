# Repository Guidelines

## Project Structure & Module Organization
- `Cargo.toml` defines the `rightcode-floatingball` crate (Rust edition 2024).
- `src/main.rs` is the binary entrypoint; keep it thin (just wiring into `app::run()`).
- `src/app.rs` owns the iced state machine (window sizing, settings view, timers).
- `src/ball.rs` renders the floating ball (Canvas) + handles input (drag, right-click refresh, wheel switch, resize).
- `src/api.rs` wraps the RightCode API call and subscription selection logic.
- `src/config.rs` loads/saves local `config.toml` (token/cookie/user-agent/refresh interval).
- Build artifacts land in `target/` and are ignored via `.gitignore`.

## Build, Test, and Development Commands
- `cargo run` — build and run locally.
- `cargo check` — fast compile/type-check during development.
- `cargo build [--release]` — produce debug or optimized binaries.
- `cargo test` — run the test suite.
- `cargo fmt --all` — format code (required before PR).
- `cargo clippy --all-targets --all-features -- -D warnings` — lint and treat warnings as errors.

## Coding Style & Naming Conventions
- Follow `rustfmt` as the source of truth; do not hand-format.
- Naming: modules/functions `snake_case`, types/traits `CamelCase`, constants `SCREAMING_SNAKE_CASE`.
- Prefer explicit errors (`Result<T, E>`) over panics for recoverable failures.

## Testing Guidelines
- There are currently no tests; add them when changing parsing/selection/config logic.
- Unit tests: colocate in the module (`src/**`) under `#[cfg(test)]`.
- Integration tests: add files under `tests/` (e.g., `tests/smoke.rs`) to exercise public behavior.
- Name tests by behavior (`parses_empty_input`, `rejects_invalid_state`).

## Commit & Pull Request Guidelines
- Git history is currently empty; use a consistent convention going forward (recommended: Conventional Commits).
  - Examples: `feat: add draggable floating ball`, `fix: prevent crash on empty config`, `docs: update usage`.
- PRs should include: problem/solution summary, how you tested (`cargo test`), and screenshots of the floating ball/settings if UI changes.

## Security & Configuration Tips
- Never commit real `Authorization` tokens or `cf_clearance` cookies; use the Settings UI to store them in your local `config.toml`.
