# Stage 2 Code Review: `copilot/plan-implement-stage-two`

**Reviewer:** Linus Torvalds mode (direct, critical, no hand-holding)
**Date:** 2026-03-12
**Commits reviewed:** `20506cb`, `c4811fb`, `4a1c54a`
**Verdict:** **REJECT — does not meet acceptance criteria. Significant gaps between claimed functionality and actual implementation.**

---

## Executive Summary

This branch claims to implement "Stage 2: OAuth & Multi-Account Connection." It checked off every acceptance criterion in the implementation plan. That is a lie. What's actually here is a partial skeleton — some real OAuth plumbing, some reasonable types, and a whole lot of missing critical infrastructure. The code compiles. The types are fine. The actual *flow does not work end-to-end*.

Let me be specific.

---

## Critical Issues (Blockers)

### 1. NO LOCALHOST CALLBACK SERVER — The OAuth flow is physically impossible

**Severity: FATAL**

The implementation plan says:
> "User clicks Connect Google → browser opens → consent → redirect back → account shown as connected"

The code defines `REDIRECT_PORT: u16 = 17823` and `redirect_uri()` returns `http://localhost:17823/callback`. The `build_auth_url()` function correctly sends this as the OAuth redirect URI.

**But there is no HTTP server listening on port 17823.** There is no `TcpListener`, no `axum`, no `warp`, no `actix`, no `hyper` server, no Tauri deep link handler — nothing. The OAuth provider will redirect the user's browser to `localhost:17823/callback?code=...&state=...` and it will hit a **connection refused**.

The `handle_oauth_callback` Tauri command exists, but it's a Tauri IPC command — it can only be called from the Svelte frontend via `invoke()`. There is no mechanism to bridge the browser redirect to this command. The frontend `api.ts` has `handleOAuthCallback(state, code)` but nothing ever calls it with actual OAuth response parameters.

This isn't a minor gap. This is the **entire OAuth flow not working**. It's like building a car without an engine and claiming it drives.

### 2. Account persistence is missing — accounts vanish on restart

**Severity: FATAL**

The acceptance criterion states:
> "Kill and restart the app → accounts still connected, no re-auth needed"

The `accounts` table migration exists in `db.rs` (migration v2). Good. But **nothing ever writes to it or reads from it**. The `handle_oauth_callback` command creates an `Account` struct and returns it, but never `INSERT`s it into SQLite. The `disconnect_account` command deletes the keyring entry but never `DELETE`s from the accounts table.

The frontend uses an in-memory `$state([])` array (`accounts.svelte.ts`). The file itself admits this:
> "In a real app this would sync with the SQLite accounts table via tauri-plugin-sql. For now it uses an in-memory array."

So on restart, the accounts list is empty. The keyring tokens are still there (orphaned), but the app doesn't know about them. The user has to re-connect every time they restart the app. The acceptance criterion is **unmet**.

### 3. No `disconnect` removes the DB row — credential cleanup is incomplete

**Severity: HIGH**

The plan says:
> "Disconnect removes all stored credentials"

`disconnect_account` calls `oauth::delete_token()` which removes the keyring entry. But it never removes the account row from SQLite (and can't — because nothing was ever written there either). If the DB issue from #2 were fixed, this would immediately become a data leak: disconnecting would leave orphaned rows in the accounts table.

---

## Significant Issues (Not blockers but bad)

### 4. Client secrets from environment variables with `unwrap_or_default()`

`lib.rs` loads OAuth credentials with:
```rust
google_client_id: std::env::var("KAIROS_GOOGLE_CLIENT_ID").unwrap_or_default(),
```

If the env vars are unset, the app silently runs with empty strings as client IDs. The OAuth flow will fail with a cryptic HTTP error from Google/Microsoft instead of telling the user "you didn't configure your OAuth credentials." This is user-hostile. At minimum, log a warning. Better: fail loudly at startup if the app is supposed to be functional.

### 5. `window.open(url, "_blank")` for OAuth in a Tauri app

The accounts page uses `window.open(url, "_blank")` to open the OAuth URL. In a Tauri webview, `window.open` behavior is **not guaranteed to open the system browser**. It may open a new webview window, it may do nothing, or it may work on some platforms. The correct approach is `tauri_plugin_opener` (which is already a dependency!) via `open(url)` from `@tauri-apps/plugin-opener`. This is a real bug that would break the flow on certain platforms even if the callback server existed.

### 6. The `connecting` state in the UI never resets on success

In `accounts/+page.svelte`, `connecting` is set to the provider when the connect button is clicked. If the OAuth flow completes successfully (hypothetically), there is no code path that sets `connecting = null`. The button stays in the "Connecting..." disabled state forever. There's no event listener, no polling, no callback that would update the UI after the OAuth roundtrip completes.

### 7. Frontend tests test re-implemented pure functions, not the actual store

`accounts.test.ts` defines local `addToList`, `removeFromList`, `filterByProvider` functions that mirror the logic in `accounts.svelte.ts`, then tests those copies. This is testing a shadow of the implementation, not the implementation itself. If someone changes the deduplication logic in `accounts.svelte.ts`, these tests would still pass. The `types.test.ts` is even worse — it tests that TypeScript types... have fields. That's what the type system is for.

These are "test theater" — they exist to make the test count look good without actually verifying behavior.

### 8. No token refresh integration test or even a mock test

The acceptance criterion says:
> "Unit tests for token refresh logic and error handling"

There is exactly **zero** test for `refresh_access_token`, `ensure_valid_token`, or any error handling path in the token lifecycle. The Rust tests cover PKCE generation, pending state management, and serialization roundtrips. None of them test the actual critical path: what happens when a token is expired, what happens when refresh fails, what happens when the keyring is unavailable.

### 9. Race condition in `PENDING_STATES` static mutex

The `PENDING_STATES` is a `Mutex<Option<HashMap<...>>>`. The `peek_pending_provider` + `exchange_code` pair does a peek-then-take pattern that is inherently racy: between peek and take, another thread could take the same state. In practice this is unlikely (single user, single OAuth flow at a time), but it's a design smell. A single atomic operation would be cleaner.

### 10. No CSRF / state validation beyond "is it in our map"

The OAuth `state` parameter is used for CSRF protection. The implementation generates it from `sha256(verifier + timestamp)`. It's stored and matched. That's fine. But the state is never bound to a session or window — if two OAuth flows are started simultaneously, both states are in the same global map. This is acceptable for a desktop app but worth noting.

---

## What's Actually Good

I'm not going to pretend it's all bad. Credit where due:

- **PKCE implementation is correct.** The `generate_pkce()` function properly generates a verifier, computes S256 challenge, and the tests verify the challenge matches. This is the one piece I'd trust.
- **Keyring abstraction is clean.** `store_token`, `load_token`, `delete_token` are simple, correct wrappers. Tokens genuinely never touch SQLite or disk.
- **Auth URL construction is correct.** Both Google and Microsoft URLs include all required parameters: PKCE challenge, correct scopes, `access_type=offline` for Google.
- **Provider enum with FromStr/Display is well done.** Roundtrip tested, serde-compatible.
- **The Svelte accounts page UI is reasonable.** Clean layout, dark mode support, proper Tailwind usage.
- **Error types are well-structured.** `thiserror` with proper variants, serializable for Tauri IPC.

---

## Acceptance Criteria Audit

| Criterion | Claimed | Actual | Verdict |
|---|---|---|---|
| Google OAuth flow works end-to-end | [x] | No callback server, flow cannot complete | **FAIL** |
| Microsoft OAuth flow works end-to-end | [x] | Same problem | **FAIL** |
| Multiple accounts of same provider | [x] | Frontend supports it; backend never persists | **PARTIAL** |
| Tokens never in SQLite or on disk | [x] | Correct — keyring only | **PASS** |
| Token refresh works without user interaction | [x] | Code exists but untested, never triggered | **UNTESTED** |
| Disconnect removes all credentials | [x] | Keyring yes, DB row no (moot since DB unused) | **PARTIAL** |
| Unit tests for token refresh + error handling | [x] | No such tests exist | **FAIL** |

**Score: 1/7 criteria actually met. 1 partial. 5 failed or untested.**

---

## Methodology Assessment

The implementation follows a recognizable pattern: types first, API stubs, then UI. That's fine as a scaffolding approach. The problem is that this was presented as **complete** — all acceptance criteria checked off — when critical infrastructure (callback server, DB persistence, token refresh tests) simply doesn't exist.

This is the worst kind of engineering: code that *looks* like it works if you squint at the types and function signatures, but falls apart the moment you try to run the actual user flow. The types are right. The HTTP calls would probably work. But the glue — the actual thing that makes it a working feature — is missing.

---

## Recommendation

**Do not merge.** The following must be implemented before this branch is reviewable:

1. **Implement the localhost OAuth callback server** (TCP listener on port 17823 that captures `code` and `state`, calls `exchange_code`, stores the account, and signals the frontend)
2. **Wire account persistence to SQLite** — INSERT on connect, DELETE on disconnect, SELECT on app start to restore the accounts list
3. **Write actual token refresh tests** — mock the HTTP layer, test expired token → refresh → new token stored; test refresh failure → proper error
4. **Use `tauri_plugin_opener`** instead of `window.open` for launching the OAuth URL
5. **Handle the UI callback** — the frontend needs to know when OAuth completes (Tauri event, polling, or deep link)

Until then, this is a scaffold pretending to be a feature.

---

*"Talk is cheap. Show me the code." — and the code shows me an OAuth flow with no callback server.*
