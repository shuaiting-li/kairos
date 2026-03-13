# Code Review: `copilot/plan-implement-stage-two`

**Reviewer:** Claude (channeling Linus Torvalds energy)
**Branch:** `copilot/plan-implement-stage-two`
**Scope:** Stage 2 — OAuth & Multi-Account Connection
**Verdict: REJECT — significant issues must be addressed before merge.**

---

## Executive Summary

The branch adds OAuth callback handling, SQLite persistence for accounts, and
frontend event wiring. The structural direction is correct, but the
implementation has a critical race condition, hand-rolls functionality that
should use existing crates, ships tests that don't test anything, and silently
swallows errors that will confuse users. Multiple acceptance criteria from
IMPLEMENTATION_PLAN.md are unverified or unmet.

---

## Critical Issues

### 1. Two code paths race for the same PKCE state (BUG)

`callback_server::process_callback` and `commands::handle_oauth_callback` both
call `oauth::take_pending_state()`. They consume the same `PENDING_STATES` map.
If the callback server handles the redirect (which it will, since it binds the
port), then `handle_oauth_callback` can never succeed — the state is already
consumed. Conversely, if some code path invokes the Tauri command first, the
callback server gets `NoPendingState`.

This is not a theoretical race. It's two functions doing the **exact same job**
wired to the **exact same one-time state**. One of them is dead code. Pick one.
Delete the other.

**File:** `callback_server.rs:162-214` vs `commands.rs:39-82`

### 2. Hand-rolled HTTP server from raw TCP

`callback_server.rs` implements an HTTP server by reading raw bytes from a
`TcpStream` with a fixed 4096-byte buffer. This is C-in-1995 territory.

Problems:
- **4096-byte buffer is arbitrary.** OAuth providers can and do send large
  query strings (Microsoft in particular with long authorization codes and
  state values). A truncated read means silently lost parameters and a
  broken flow with zero diagnostics.
- **No read timeout.** A misbehaving client (or a port scanner) holds a
  connection open forever, tying up a tokio task.
- **No handling of partial reads.** TCP does not guarantee a full HTTP
  request arrives in one `read()` call. On a slow connection or with large
  headers, you get a partial parse and a silent failure.
- **No request method validation.** Any HTTP method (POST, PUT, DELETE) is
  treated the same as GET.

Use `hyper`, `axum`, `warp`, or `tiny_http`. The `url` crate is already a
dependency — there's no excuse for hand-rolling URL decoding either (see
Issue 3).

**File:** `callback_server.rs:89-157`

### 3. Hand-rolled `url_decode` when `url` crate is already a dependency

`callback_server.rs:242-264` implements percent-decoding from scratch. The
`url` crate (already used in `oauth.rs` for `Url::parse_with_params`) provides
`url::form_urlencoded::parse()` which handles this correctly, including edge
cases with multi-byte UTF-8 sequences.

The hand-rolled version iterates over `chars()` but percent-encoding operates
on bytes. For ASCII query strings this works by coincidence, but it's
semantically wrong. The fallback path (`bytes.push(b'%')`) will produce
garbled output on malformed input rather than a clean error.

Don't reinvent the wheel — especially not a wheel you already have in your
`Cargo.toml`.

**File:** `callback_server.rs:242-264`

---

## Serious Issues

### 4. Tests that test nothing

Three "tests" in `oauth.rs` are fraud:

```rust
fn token_data_with_empty_refresh_token() {
    let token = TokenData { ... expires_at: 0 };
    assert!(token.refresh_token.is_empty());      // You just set it to ""
    assert!(token.expires_at <= now);              // You just set it to 0
}

fn token_data_is_valid_when_not_expired() {
    let future = now + 3600;
    let token = TokenData { ... expires_at: future };
    assert!(token.expires_at > now + 60);          // You just set it to now + 3600
}

fn token_data_needs_refresh_when_near_expiry() {
    let near_future = now + 30;
    let token = TokenData { ... expires_at: near_future };
    assert!(token.expires_at <= now + 60);         // You just set it to now + 30
}
```

These tests construct a struct, then assert on the values they just put in.
They test Rust's struct initialization — not your code. They test arithmetic —
not your logic. Zero behavioral coverage of `ensure_valid_token`, which is the
actual function that needs testing.

The IMPLEMENTATION_PLAN explicitly requires: **"Unit tests for token refresh
logic and error handling."** These tests do not fulfill that requirement.

**File:** `oauth.rs:604-655` (in the diff)

### 5. Silent error swallowing in the frontend

```typescript
} catch (e) {
    console.warn("Could not persist account to database:", e);
}
addAccount(account);  // Added to in-memory store regardless
```

If SQLite insertion fails, the account appears connected in the UI. On the
next app restart, it's gone — because it was never persisted. The user sees a
"connected" account that silently disappears.

This pattern appears **four times** in the accounts page. Every DB operation
is wrapped in try/catch that logs to console and moves on. This is not
graceful degradation — it's hiding bugs from users.

At minimum: surface a visible warning in the UI. Better: don't add to the
in-memory store if persistence fails.

**File:** `src/routes/accounts/+page.svelte:38-42, 51-54, 83-87`

### 6. No token refresh mutex (risk register item ignored)

The IMPLEMENTATION_PLAN risk register explicitly calls out:

> **"OAuth token refresh race conditions — Users get logged out randomly —
> Implement token refresh mutex in Stage 2."**

There is no mutex, lock, or any concurrency control around
`ensure_valid_token`. If two concurrent requests (e.g., email poller and
calendar sync in Stage 3/4) both detect an expired token, both will attempt
refresh. Depending on the provider, the second refresh may invalidate the
first token, and the first caller gets a 401 on their next request.

This was identified as a known risk. It should have been addressed.

**File:** `oauth.rs:478-505` (`ensure_valid_token`)

### 7. Hardcoded port with no fallback

`REDIRECT_PORT = 17823` is hardcoded. If another process (or another instance
of Kairos) is using that port, the `TcpListener::bind` fails and the OAuth
callback server dies silently. The error is logged but the user sees nothing —
they click "Connect Google" and the flow hangs forever after browser consent.

At minimum: detect bind failure and surface it to the user. Better: try a
range of ports and register the chosen one in the OAuth redirect URI.

**File:** `oauth.rs:REDIRECT_PORT`, `callback_server.rs:90-91`

---

## Minor Issues

### 8. `Mutex<Option<HashMap>>` for no reason

`PENDING_STATES` is `Mutex<Option<HashMap<...>>>`. The `Option` wrapping
serves no purpose except to require an `init()` call. Use
`Mutex<HashMap<...>>` initialized with `Mutex::new(HashMap::new())` and
delete the `init()` ceremony. Or better yet, use `once_cell::sync::Lazy` /
`std::sync::LazyLock`.

**File:** `oauth.rs:152-155`

### 9. Duplicate code between `callback_server` and `commands`

Even if Issue 1 is resolved by picking one code path, the account-creation
logic (build account ID, select scopes, create `Account` struct) is
duplicated verbatim. Extract it into a shared function in `oauth.rs`.

### 10. `Content-Length` correctness for non-ASCII emails

`send_response` uses `body.len()` which is byte length — correct for
`Content-Length`. But the HTML body interpolates user email addresses via
`html_escape(&email)`. If the email contains non-ASCII characters
(internationalized addresses), `body.len()` is still correct, but the HTML
page doesn't declare `<meta charset>` — only the HTTP header specifies
`charset=utf-8`. Minor, but sloppy.

### 11. `connecting` state never clears on browser-close

If the user clicks "Connect Google", the browser opens, but the user closes
the browser tab without completing consent, `connecting` stays set to
`"google"` and the button remains disabled. There's no timeout or recovery
mechanism.

---

## Acceptance Criteria Assessment (IMPLEMENTATION_PLAN.md Stage 2)

| Criterion | Status | Notes |
|---|---|---|
| Google OAuth flow works | **Partial** | Code exists but untested against real APIs. Two competing code paths (Issue 1). |
| Microsoft OAuth flow works | **Partial** | Same as Google. |
| Multiple accounts of same provider | **Met** | Data model supports it; dedup by ID works. |
| Tokens never in SQLite or on disk | **Met** | Keyring-only storage is correctly implemented. |
| Token refresh works without user interaction | **Not verified** | Code exists but no tests exercise actual refresh logic (Issue 4). No mutex (Issue 6). |
| Disconnect removes all stored credentials | **Partial** | Keyring deletion works; SQLite deletion is best-effort with silent failure (Issue 5). |
| Unit tests for token refresh logic | **NOT MET** | The "tests" don't test any logic (Issue 4). |

---

## Summary of Required Actions

1. **Delete one of the two OAuth callback code paths.** This is a real bug.
2. **Replace the hand-rolled HTTP server** with a proper library (axum/hyper).
3. **Replace hand-rolled `url_decode`** with `url::form_urlencoded::parse`.
4. **Write real tests** for `ensure_valid_token` — mock the keyring and HTTP,
   test the actual refresh-or-return logic.
5. **Add a token refresh mutex** as the implementation plan's risk register
   requires.
6. **Surface DB errors to the user** instead of swallowing them.
7. **Handle port binding failure** gracefully.

The bones are there. The architecture is sound. But the execution has too many
shortcuts for a Stage 2 acceptance. Fix the issues above and re-submit.
