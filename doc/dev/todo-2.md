# TODO List - Refactor unwrap() and ? operator usage in `*_pages.rs`

This list tracks locations in the Rust page controller files (`src/controllers/*_pages.rs`) where `.unwrap()` or the `?` operator are used. 

According to `endpoints.mdc`, human-facing endpoints (those in `*_pages.rs`) **MUST NOT** handle errors by returning an `Error` type (which the `?` operator often propagates). Instead, they should return an HTML response displaying the error appropriately within the page or fragment.

Similarly, `.unwrap()` calls should be replaced with proper error handling that results in an informative HTML response rather than a panic.

## `.unwrap()` Occurrences

- [ ] `src/controllers/auth_pages.rs:310` (in comment, verify if relevant)
- [x] `src/controllers/teams_pages.rs:31`
- [x] `src/controllers/teams_pages.rs:59`
- [x] `src/controllers/teams_pages.rs:120`
- [x] `src/controllers/teams_pages.rs:251`
- [x] `src/controllers/teams_pages.rs:324`
- [x] `src/controllers/teams_pages.rs:397`
- [x] `src/controllers/teams_pages.rs:446`
- [x] `src/controllers/teams_pages.rs:492`
- [x] `src/controllers/teams_pages.rs:539`
- [x] `src/controllers/teams_pages.rs:585`
- [x] `src/controllers/teams_pages.rs:691`
- [x] `src/controllers/teams_pages.rs:776`
- [x] `src/controllers/teams_pages.rs:886`
- [x] `src/controllers/teams_pages.rs:923`
- [x] `src/controllers/teams_pages.rs:1026`
- [ ] `src/controllers/users_pages.rs:40`
- [ ] `src/controllers/users_pages.rs:99`
- [ ] `src/controllers/users_pages.rs:144`
- [ ] `src/controllers/users_pages.rs:170`
- [ ] `src/controllers/users_pages.rs:205`

## `?` Operator Occurrences

- [ ] `src/controllers/teams_pages.rs:37`
- [ ] `src/controllers/teams_pages.rs:65`
- [ ] `src/controllers/teams_pages.rs:95`
- [ ] `src/controllers/teams_pages.rs:128` (in tracing)
- [ ] `src/controllers/teams_pages.rs:139`
- [ ] `src/controllers/teams_pages.rs:143` (in tracing)
- [ ] `src/controllers/teams_pages.rs:166`
- [ ] `src/controllers/teams_pages.rs:172`
- [ ] `src/controllers/teams_pages.rs:192`
- [ ] `src/controllers/teams_pages.rs:197`
- [ ] `src/controllers/teams_pages.rs:217`
- [ ] `src/controllers/teams_pages.rs:257` (in tracing)
- [ ] `src/controllers/teams_pages.rs:268`
- [ ] `src/controllers/teams_pages.rs:273` (in tracing)
- [ ] `src/controllers/teams_pages.rs:281` (in tracing)
- [ ] `src/controllers/teams_pages.rs:292`
- [ ] `src/controllers/teams_pages.rs:330` (in tracing)
- [ ] `src/controllers/teams_pages.rs:341`
- [ ] `src/controllers/teams_pages.rs:346` (in tracing)
- [ ] `src/controllers/teams_pages.rs:354` (in tracing)
- [ ] `src/controllers/teams_pages.rs:365`
- [ ] `src/controllers/teams_pages.rs:410` (in tracing)
- [ ] `src/controllers/teams_pages.rs:421` (in tracing)
- [ ] `src/controllers/teams_pages.rs:452` (in tracing)
- [ ] `src/controllers/teams_pages.rs:463`
- [ ] `src/controllers/teams_pages.rs:474`
- [ ] `src/controllers/teams_pages.rs:505` (in tracing)
- [ ] `src/controllers/teams_pages.rs:517`
- [ ] `src/controllers/teams_pages.rs:523`
- [ ] `src/controllers/teams_pages.rs:552` (in tracing)
- [ ] `src/controllers/teams_pages.rs:563`
- [ ] `src/controllers/teams_pages.rs:569`
- [ ] `src/controllers/teams_pages.rs:591` (in tracing)
- [ ] `src/controllers/teams_pages.rs:606` (in tracing)
- [ ] `src/controllers/teams_pages.rs:613` (in tracing)
- [ ] `src/controllers/teams_pages.rs:631` (in tracing)
- [ ] `src/controllers/teams_pages.rs:652` (in tracing)
- [ ] `src/controllers/teams_pages.rs:664`
- [ ] `src/controllers/teams_pages.rs:674`
- [ ] `src/controllers/teams_pages.rs:692`
- [ ] `src/controllers/teams_pages.rs:693`
- [ ] `src/controllers/teams_pages.rs:700` (in tracing)
- [ ] `src/controllers/teams_pages.rs:713`
- [ ] `src/controllers/teams_pages.rs:733` (in tracing)
- [ ] `src/controllers/teams_pages.rs:745`
- [ ] `src/controllers/teams_pages.rs:755`
- [ ] `src/controllers/teams_pages.rs:760`
- [ ] `src/controllers/teams_pages.rs:777`
- [ ] `src/controllers/teams_pages.rs:778`
- [ ] `src/controllers/teams_pages.rs:802` (in tracing)
- [ ] `src/controllers/teams_pages.rs:820` (in tracing)
- [ ] `src/controllers/teams_pages.rs:856`
- [ ] `src/controllers/teams_pages.rs:865`
- [ ] `src/controllers/teams_pages.rs:870`
- [ ] `src/controllers/teams_pages.rs:892` (in tracing)
- [ ] `src/controllers/teams_pages.rs:898`
- [ ] `src/controllers/teams_pages.rs:904`
- [ ] `src/controllers/teams_pages.rs:927` (in tracing)
- [ ] `src/controllers/teams_pages.rs:933`
- [ ] `src/controllers/teams_pages.rs:954`
- [ ] `src/controllers/teams_pages.rs:998`
- [ ] `src/controllers/teams_pages.rs:1020` (in tracing)
- [ ] `src/controllers/teams_pages.rs:1041` (in tracing)
- [ ] `src/controllers/teams_pages.rs:1051` (in tracing)
- [ ] `src/controllers/teams_pages.rs:1069`
- [ ] `src/controllers/teams_pages.rs:1081`
- [ ] `src/controllers/teams_pages.rs:1087` (in tracing)
- [ ] `src/controllers/auth_pages.rs:65`
- [ ] `src/controllers/auth_pages.rs:67`
- [ ] `src/controllers/auth_pages.rs:72`
- [ ] `src/controllers/auth_pages.rs:165`
- [ ] `src/controllers/auth_pages.rs:193`
- [ ] `src/controllers/auth_pages.rs:290`
- [ ] `src/controllers/auth_pages.rs:360`
- [ ] `src/controllers/auth_pages.rs:396`
- [ ] `src/controllers/auth_pages.rs:427`
- [ ] `src/controllers/users_pages.rs:46`
- [ ] `src/controllers/users_pages.rs:75`
- [ ] `src/controllers/users_pages.rs:105`
- [ ] `src/controllers/users_pages.rs:109`
- [ ] `src/controllers/users_pages.rs:151`
- [ ] `src/controllers/users_pages.rs:185`
- [ ] `src/controllers/users_pages.rs:190`
- [ ] `src/controllers/users_pages.rs:221`
- [ ] `src/controllers/users_pages.rs:226`
- [ ] `src/controllers/home_pages.rs:24` 