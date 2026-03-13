# Stage 2 Code Review: `copilot/plan-implement-stage-two`

**Reviewer:** Code review in the style of Linus Torvalds
**Date:** 2026-03-13
**Verdict: REJECT — significant issues must be addressed before merge.**

---

## Executive Summary

This branch implements OAuth 2.0 with PKCE for Google and Microsoft, a localhost callback server, keyring-based token storage, an accounts settings page, and SQLite persistence. The *shape* of the work is correct — it covers the right surface area for Stage 2. But the implementation has real problems ranging from architectural to security to correctness. Several acceptance criteria from `IMPLEMENTATION_PLAN.md` are unverifiable or unmet.

---

## Acceptance Criteria Audit

| Criteria | Status | Notes |
|---|---|---|
| Google OAuth flow works end-to-end | **UNVERIFIABLE** | No integration test. Manual-only verification. Code looks structurally correct but has never been proven against a real provider in this codebase. |
| Microsoft OAuth flow works end-to-end | **UNVERIFIABLE** | Same. |
| Multiple accounts of same provider | **PARTIAL** | The `INSERT OR REPLACE` in `db.ts` and duplicate-check in the store support this, but the account ID scheme `{provider}-{email}` means reconnecting the same email silently overwrites. No test for this edge case. |
| Tokens never stored in SQLite or on disk | **MET** | Tokens go to keyring. Good. |
| Token refresh works without user interaction | **PARTIAL** | `ensure_valid_token` exists and is exposed as a Tauri command, but **nothing calls it automatically**. There is no interceptor, no middleware, no 401-retry logic. The implementation plan says "intercept 401, refresh, retry." That does not exist. |
| Disconnect removes all stored credentials | **MET** | Keyring delete + SQLite delete + store removal. |
| Unit tests for token refresh logic and error handling | **WEAK** | Tests exist but are shallow — see detailed section below. |

**Score: 3/7 criteria fully met. 2 partially met. 2 unverifiable.**

---

## Critical Issues

### 1. Duplicate OAuth callback handling — code processes the same callback TWICE

`callback_server.rs:process_callback()` and `commands.rs:handle_oauth_callback()` do **the same thing**: take pending state, exchange code, fetch email, store token. The callback server runs automatically on every HTTP request to `/callback`. The Tauri command `handle_oauth_callback` is also registered and exposed to the frontend.

The callback server calls `take_pending_state()` which **consumes** the state atomically. So if both paths race, one will fail with `NoPendingState`. But the real issue is: **why do both exist?** This is a confused architecture. Either the localhost server handles it (and emits an event to the frontend), OR the frontend captures the params and calls a Tauri command. Not both. Pick one and delete the other.

The `handle_oauth_callback` Tauri command is dead code in the current flow — the callback server handles everything. But `api.ts` exports `handleOAuthCallback()` as if the frontend should call it. A maintainer will be confused about which path is live.

### 2. Hand-rolled HTTP server with no Content-Length validation

`callback_server.rs` reads exactly 4096 bytes from the TCP stream and parses the HTTP request by hand. This is a minimal HTTP parser that:

- Has a fixed 4096-byte buffer with no handling for requests larger than that
- Does not validate the HTTP method (accepts POST, PUT, DELETE — anything with `/callback` in the path)
- Does not handle chunked transfer encoding, keep-alive, or any HTTP/1.1 semantics
- Has no timeout on the read — a slow-loris connection will hold the task forever
- Accepts connections in a loop with no rate limiting

For an OAuth callback that receives exactly one GET with short query params, this is *tolerable* but not good. Use `axum` or `tiny_http` or even `hyper` — they are already in the dependency tree via `reqwest`. Rolling your own HTTP parser is the kind of thing that bites you six months later.

### 3. Global mutable state via `static Mutex<Option<HashMap>>>`

`oauth.rs` uses a `static Mutex<Option<HashMap<String, (Provider, PkceChallenge)>>>` for PKCE state. This is:

- **Global mutable state** — the worst kind of state management in Rust
- Initialized lazily via `init()` which must be called before any other function — but there is no compile-time enforcement
- Using `unwrap()` on the mutex lock — will panic (and crash the app) on poisoning
- Not bounded — if `build_auth_url` is called repeatedly without callbacks completing, the map grows forever

This should be `Mutex<HashMap<...>>` (drop the `Option`) with `::new()` as the initializer. Or better: use Tauri's managed state (`app.manage()`) so the lifecycle is tied to the app.

### 4. `url_decode` is subtly broken for multi-byte UTF-8

```rust
} else {
    let mut buf = [0u8; 4];
    let encoded = c.encode_utf8(&mut buf);
    bytes.extend_from_slice(encoded.as_bytes());
}
```

This iterates by `char`, so a multi-byte character like `é` (U+00E9) is encoded as 2 bytes. But percent-encoded UTF-8 like `%C3%A9` would be decoded byte-by-byte correctly. The issue is that the function mixes byte-level percent decoding with char-level passthrough — if the input contains raw multi-byte UTF-8 (not percent-encoded), it works, but if a percent-encoded sequence produces invalid UTF-8 bytes, `String::from_utf8` falls back to the original string silently. This is a correctness bug waiting to happen.

Just use the `urlencoding` crate or `percent_encoding` from the `url` crate (which is already a dependency).

### 5. No 401-retry / token refresh interceptor

The implementation plan explicitly says:

> "Implement silent token refresh (intercept 401, refresh, retry)"

This does not exist. `ensure_valid_token` is exposed as a standalone Tauri command, but there is no mechanism to automatically intercept a 401 from Gmail/Graph API, refresh the token, and retry the request. Every future API call in Stages 3-4 will need to manually call `ensure_valid_token` before every request, or implement its own retry logic. This is a fundamental piece of the OAuth infrastructure that was supposed to be built in Stage 2.

### 6. Client secrets in environment variables with `unwrap_or_default`

```rust
let google_client_id = std::env::var("KAIROS_GOOGLE_CLIENT_ID").unwrap_or_default();
```

When these are empty (which they will be for any developer who doesn't set them), the app starts with `warn!()` messages but proceeds to register OAuth commands that will silently fail with empty credentials. The OAuth flow will break at the provider's token endpoint with a cryptic HTTP error, not a clear "you haven't configured OAuth credentials" message.

At minimum, the `get_auth_url` command should return a meaningful error when credentials are not configured, not construct a URL with an empty `client_id`.

---

## Moderate Issues

### 7. The `handle_oauth_callback` Tauri command is dead code

As noted in #1, the callback server handles the entire flow and emits a Tauri event. The `handle_oauth_callback` command in `commands.rs` and `handleOAuthCallback` in `api.ts` are never called by any code path. Dead code should not be shipped.

### 8. Tests are shallow — they test serialization, not behavior

The Rust tests are:
- PKCE generation (valid)
- URL building (valid)
- Provider round-trip serialization (trivial)
- Pending state store/take (valid)
- Token serialization (trivial — tests serde, not your code)
- `ensure_valid_token` returns error when no token exists (trivial)
- Token expiry checks that **don't actually call `ensure_valid_token`** — they just check fields on a struct manually

The test `token_data_with_empty_refresh_token` is particularly useless:
```rust
fn token_data_with_empty_refresh_token() {
    let token = TokenData { ... expires_at: 0 };
    assert!(token.refresh_token.is_empty());
    assert!(token.expires_at <= chrono::Utc::now().timestamp());
}
```
This tests that `""` is empty and `0` is less than now. It tests nothing about the actual refresh logic.

The frontend tests (`types.test.ts`) test that JSON serialization works and that a string union contains "google" and "microsoft." These are type-system tautologies, not behavioral tests.

Missing tests:
- Token exchange with mocked HTTP (use `mockito` or `wiremock`)
- Token refresh flow end-to-end (mock the provider, verify the new token is stored)
- Callback server parsing with malformed/adversarial inputs
- Concurrent PKCE state access (the whole point of the Mutex)
- Frontend: Svelte component rendering, event listener behavior

### 9. Hardcoded port 17823 with no fallback

`REDIRECT_PORT: u16 = 17823` — if another instance of the app (or anything else) is already bound to this port, the callback server will fail to start and OAuth will be completely broken. There is no port-probing, no fallback, and no error surfaced to the user.

### 10. `reqwest::Client::new()` created on every request

Both `exchange_code` and `refresh_access_token` create a new `reqwest::Client` per call. Each `Client::new()` allocates a connection pool, TLS context, etc. The client should be created once and reused (put it in Tauri managed state alongside `OAuthConfig`).

### 11. SQL injection is not possible but the pattern is fragile

`db.ts` uses parameterized queries (`?` placeholders) — good. But `INSERT OR REPLACE` is a SQLite-specific shortcut that deletes-then-inserts, which means any `ON DELETE` triggers or foreign key cascades will fire. Use `INSERT ... ON CONFLICT(id) DO UPDATE SET ...` (UPSERT) instead.

### 12. Frontend accounts page duplicates provider sections

The Google section and Microsoft section in `accounts/+page.svelte` are nearly identical — ~50 lines of duplicated markup. This should be a `{#each}` over providers or a component. When you add a third provider (or need to change the card layout), you'll need to change it in two places.

---

## Minor Issues

### 13. `html_escape` does not escape single quotes

The `html_escape` function escapes `&`, `<`, `>`, `"` but not `'` (apostrophe). In an HTML attribute context with single quotes, this is an XSS vector. Use `&#x27;` or `&apos;`.

### 14. No `Content-Security-Policy` or `X-Frame-Options` on callback response

The localhost callback server returns HTML responses without any security headers. While the blast radius is limited (localhost only), it's bad practice.

### 15. GitHub Actions workflows deleted

The diff shows `.github/workflows/claude-code-review.yml` and `.github/workflows/claude.yml` were deleted (44 + 50 lines removed). These may be copilot-specific workflows, but deleting CI config in a feature branch is suspicious. Was this intentional?

### 16. `Account.id` format is a convention, not enforced

The account ID is `format!("{}-{}", provider, email)` — but this is done ad-hoc in both `callback_server.rs` and `commands.rs`. There should be a single `Account::new()` or `Account::id_for()` method. If the email contains a `-`, the ID becomes ambiguous.

### 17. No `#[must_use]` on functions that return `Result`

Functions like `store_token`, `delete_token`, `load_token` return `Result` but are not marked `#[must_use]`. Callers can silently ignore errors.

---

## What's Actually Good

- **Token storage in OS keyring, not SQLite** — correct security decision, properly implemented.
- **PKCE implementation** — the challenge/verifier generation follows RFC 7636 correctly. S256 method is used.
- **Svelte 5 runes-based reactive store** — `accounts.svelte.ts` using `$state` and `$derived` is idiomatic Svelte 5.
- **SQL parameterized queries** — no injection vectors in `db.ts`.
- **Error types** — `OAuthError` with `thiserror` is well-structured with proper Display/Serialize.
- **HTML escaping on callback response** — prevents basic XSS in the success/error pages.
- **Biome + svelte-check pass** — the code is properly formatted and type-checks.

---

## Verdict

The code has the right *shape* for Stage 2 but fails on execution quality. The duplicate callback handling (#1) is an architectural confusion that needs resolving. The missing 401-retry interceptor (#5) is a gap that will create technical debt in every subsequent stage. The tests (#8) look like they exist to satisfy a checkbox, not to catch bugs.

**Required before merge:**
1. Remove either the callback server's `process_callback` or the `handle_oauth_callback` command. One path, not two.
2. Implement the 401-intercept-refresh-retry mechanism, or explicitly document that it's deferred (and remove the acceptance criteria checkmark).
3. Replace hand-rolled `url_decode` with the `url` crate's `percent_encoding` module.
4. Add at least one test with a mocked HTTP server for the token exchange flow.
5. Fix the global mutable state — drop the `Option` wrapper, use `Mutex<HashMap>` directly.

**Recommended before merge:**
6. Reuse `reqwest::Client` via Tauri managed state.
7. Extract duplicate Svelte markup into a provider-agnostic component.
8. Add port-binding error handling with a user-visible message.
9. Return a clear error from `get_auth_url` when credentials are not configured.
10. Delete the dead `handle_oauth_callback` command and `handleOAuthCallback` API export.

---

*"Talk is cheap. Show me the code." — and the code showed me duplicated logic, missing infrastructure, and tests that test nothing. Fix it.*
