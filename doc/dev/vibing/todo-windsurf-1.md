# TODO: Replace unwrap() and `?` operator

This file lists files containing `.unwrap()` calls or the `?` operator, which often indicate areas where error handling could be improved.

## Files with `.unwrap()` calls

- [ ] `examples/playground.rs`
- [ ] `tests/requests/auth.rs`
- [ ] `tests/requests/prepare_data.rs`
- [ ] `tests/models/teams.rs`
- [ ] `tests/models/users.rs`
- [ ] `tests/models/team_memberships.rs`
- [ ] `src/utils/middleware.rs`
- [ ] `src/utils/template.rs`
- [ ] `src/middleware/auth_no_error.rs`
- [ ] `src/app.rs`
- [ ] `src/controllers/users_pages.rs`
- [ ] `src/controllers/auth_pages.rs`
- [ ] `src/controllers/teams_pages.rs`
- *...(Potentially more files - search needed for full list)*

## Files with `?` operator

- [ ] `migration/src/m20220101_000001_users.rs`
- [ ] `src/utils/template.rs`
- [ ] `src/controllers/auth_pages.rs`
- [ ] `src/controllers/home_pages.rs`
- [ ] `src/controllers/auth_api.rs`
- [ ] `src/middleware/auth_no_error.rs`
- *...(Many more files - search needed for full list)*

*Note: This list is based on initial search results and may be incomplete due to the large number of occurrences. A more thorough search within these files is recommended.*
