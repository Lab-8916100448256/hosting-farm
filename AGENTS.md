# AGENTS
## Build, Lint & Test
- Build: `cargo build`, Check: `cargo check`, Test all: `cargo test`, Test single: `cargo test <test_name>`
- Format: `cargo fmt --all -- --check`, Lint: `cargo clippy -- -D warnings`
- E2E: `npx playwright test` / `--ui`, Codegen: `npx playwright codegen http://localhost:5151`


## Code Style Guidelines
- rustfmt max_width=100 (/.rustfmt.toml); clippy per CI (/.github/workflows/ci.yaml)
- import order: std, external crates, local modules
- snake_case for fns/vars, CamelCase for types
- use `Result`/`?`; HTTP errors via status codes or HTML error pages

## Project Rules
- See `.cursor/rules/rust.mdc` and `.cursor/rules/endpoints.mdc`

