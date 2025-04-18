# Codex CLI Instructions for loco-rs Projects

These guidelines direct the Codex CLI agent on best practices, conventions, and workflows for maintaining and extending this **loco-rs**-based web application.

## 1. Prerequisites & Environment
- Ensure Rust toolchain (1.60+) with `rustfmt` and `clippy` is available.
- Confirm `cargo-loco` plugin is installed.
- Node.js/npm or equivalent CSS build tool for Tailwind.

## 2. Project Layout
- Follow the classic project structure of loco-res framework. In particular, respect these path:
  ```
  Cargo.toml
  assets/
    i18n/
    static
    views/
  config/
  doc/
  migrations/
  src/
    bin/
      main.rs
    controllers/
      *_api.rs
      *_pages.rs
    fixtures/
    initializers/
    mailers/
    middlewares/
    models/
    tasks/
    views/
    workers/
    utils/
    app.rs
    lib.rs
  tests/
  ```

## 3. Code Generation & CLI Commands
- Use `cargo loco generate` for scaffolds, then refine:
  ```
  cargo loco generate model <Name>
  cargo loco generate migration <desc>
  cargo loco generate controller <Name>
  ...
  cargo loco generate scaffold <Resource>
  ```
- Use minimal targeted edits.
- Always run fmt/clippy/tests before finalizing.
- Do not commit failing code.

## 4. Coding Conventions
- Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` after edits.
- Adhere to Rust 2021 edition style (descriptive naming, small functions, ...).
- Document public APIs with `///`.

## 5. Routing & Controllers
- Controllers in `src/controllers/`; suffix `_api.rs` or `_pages.rs`.
- Define routes via `Routes::new().prefix(...).add(path, method(handler))`.
- Use `{}` for path parameters.

## 6. API Endpoints
- Prefix: `/api/...`.
- JSON-only responses with `format::json()`.
- Standard HTTP methods and status codes.
- Consistent error schema: `{ status, code, message }`.
- No HTML or template rendering.
- Path parameters MUST use curly braces ({}), not a colon (:).
- Examples of API endpoints implementation in the existing code base: `src/controller/auth_api.rs`

## 7. Human-Facing Endpoints and UI considerations
- No `/api/` prefix.
- Always use server side rendering with HTMX and Hyperscript
- Make minimal usage of javascript, primarily for enhancing HTMX functionality. Always prefer using HTMX and Hyperscript over adding custom javascript to the front-end
- Use tailwind CSS for styling
- Render HTML using Tera templates via `format::render().view(...)` or `.fragment(...)`. There are utility functions for that in `src/views/mod.rs`. Use them or create new one if none is applicable.
- Handle failures with error pages, or fragments injected into calling page though HTMX. No `unwrap()` or `?` operator in handlers (can be used in functions called by handlers, if correctly catched by handlers).
- Redirect with utility funtion `redirect(url, headers)` that is in `src/views/mod.rs`.
- For pages that need authentication, apply `auth_no_error` middleware; redirect unauthenticated to login page.
- Path parameters MUST use curly braces ({}), not a colon (:).
- Example of human-facing endpoints and views implementation in the existing code base: `src/controller/teams_pages.rs` and `assets/views/teams/*.html`

## 8. Templates & Assets
- Tera in `assets/views/`, organized by feature.
- Tailwind in `assets/styles/`; compile via build script.

## 9. Models,  Migrations and Controllers 
- Models in `src/models/` (via generator).
- Migrations in `migrations/<version>_<desc>/`.
- To generate a new model, entity and the associated database migration use `cargo loco generate model ...`. 
  - For this to work well you need to pass the new model name and its fileds as parameters. 
  - Example : `cargo loco generate model posts title:string! content:text user:references`. 
  - Refer to the doc for all the data types : https://loco.rs/docs/the-app/models/
- To generate a new database migration to alter an existing model use `cargo loco generate migration <name of migration> [name:type, name:type ...]`
- To generate a new controller use `cargo loco generate controller <name> [...]`

## 10. Background & Async Jobs
- Use loco generators for tasks, workers, mailers, schedulers.
- Queue mailers and background tasks properly.

## 11. Error Handling & Logging
- Define errors with `thiserror`.
- Centralize mapping to HTTP responses.
- Use `tracing` for logs; no sensitive data.

## 12. Testing & CI
- Unit tests in code or `tests/`.
- Dedicated test DB; reset state.
- Commands:
  ```
  cargo fmt --all
  cargo clippy --all-targets -- -D warnings
  cargo test --all
  ```

## 13. Security
- Validate/sanitize inputs.
- CSRF protection.
- CORS policies for API.
- Secrets in application configuration.
- No sensitive data in logs

## 14. Performance
- Cache expensive ops.
- Avoid blocking in handlers.

## 15. Version Control
- Doc comments → `cargo doc`.
- Conventional Commits.

## 16. project dependencies
- Add new dependencies in Cargo.toml only for complex code where adding a new dependencies is really worth it.
- Do not add any new dependency for anything that is already provided by the loco-rs framework. Look in other places of the code and in the alredy defined dependencies to check if there isn't already something available to implement something before adding a new dependency.
- You MUST NEVER change any dependencies that are already in Cargo.toml. Doing so would break many things. 
- Dependencies versions that are set in Cargo.toml MUST NOT be changed! 

## 17. Debugging issues that require human interaction with the application**
- If testing by human interaction is needed to better understand an issue or to verify that a bug fix or new feature implementation works as expected, do not run the application yourself. Ask me to do that and to provide you with the results.
