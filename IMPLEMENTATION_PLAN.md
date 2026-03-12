# AI Secretary — Implementation Plan

## Technology Choices

### Core

| Technology | Role | Justification |
|---|---|---|
| **Tauri v2** | Desktop shell, system tray, native packaging, auto-updater | Purpose-built for lightweight cross-platform desktop apps. Built-in system tray, notifications, secure storage, deep links (for OAuth callbacks). Produces small native binaries (~5-10MB). |
| **Rust** | Backend: API calls, background processing, database, OAuth, secure token storage | Tauri's native backend language. Excellent async ecosystem (tokio), strong type system, memory safety. Keeps secrets (OAuth tokens) out of the webview process. Ideal for a long-running background daemon that must be reliable and resource-efficient. |
| **Svelte 5** | Frontend UI framework | Lightest mainstream framework — no virtual DOM, minimal runtime (~2KB). Svelte 5 runes provide fine-grained reactivity. Official Tauri template available. Less boilerplate than React, better performance than Vue for small apps. |
| **TypeScript (strict)** | Frontend type safety | `strict: true` in tsconfig gives compile-time safety across the entire frontend. Natural pairing with Svelte 5's TypeScript support. |

### Backend (Rust crates)

| Crate | Role | Justification |
|---|---|---|
| **tokio** | Async runtime | Industry standard for async Rust. Required for concurrent email polling across multiple accounts without blocking the UI. |
| **reqwest** | HTTP client | Ergonomic async HTTP client built on tokio + hyper. Handles Google/Microsoft/Anthropic API calls, connection pooling, TLS. |
| **sqlx** | Database (SQLite) | Compile-time checked SQL queries against SQLite. Async-native. Provides migration support. SQLite is the right choice for a local desktop app — zero deployment, single file, no server. |
| **serde / serde_json** | Serialization | De facto standard for Rust serialization. Every API response and Tauri command boundary uses serde. |
| **keyring** | Secure credential storage | Stores OAuth tokens in the OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service). Tokens never touch disk in plaintext. |
| **chrono / chrono-tz** | Date/time/timezone | Robust timezone-aware datetime handling. Critical for cross-timezone scheduling logic. |
| **tauri-plugin-notification** | Native OS notifications | Tauri's official plugin for native notifications. Used to surface suggestions and alerts without requiring the dashboard to be open. |
| **tauri-plugin-autostart** | Launch on login | Tauri's official plugin. The app should start automatically — a secretary that only works when you remember to open it isn't useful. |

### Frontend

| Technology | Role | Justification |
|---|---|---|
| **Tailwind CSS v4** | Styling | Utility-first CSS. Fast iteration, consistent design, small production bundle with purging. No need for a component library — the UI is simple (cards, lists, buttons, forms). |
| **bits-ui** | Headless Svelte components | Accessible, unstyled primitives (dialogs, dropdowns, tooltips). Style with Tailwind. Avoids the weight of a full component library. |

### Dev Tooling

| Tool | Role | Justification |
|---|---|---|
| **pnpm** | JS package manager | Faster and more disk-efficient than npm. Strict dependency resolution prevents phantom deps. |
| **Vitest** | Frontend unit/integration tests | Fast Vite-native test runner. Compatible with Svelte component testing. |
| **Playwright** | End-to-end tests | Cross-browser E2E testing. Can drive the Tauri webview for full integration tests. |
| **cargo test** | Rust unit/integration tests | Built into the Rust toolchain. Test each backend module in isolation. |
| **cargo clippy** | Rust linting | Catches common mistakes, enforces idioms. Run in CI. |
| **Biome** | TS/Svelte linting + formatting | Faster alternative to ESLint + Prettier. Single tool for both linting and formatting. |
| **GitHub Actions** | CI/CD | Build, test, lint on every push. Tauri has official GitHub Actions for cross-platform builds. |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Tauri v2 App                         │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Rust Backend                        │  │
│  │                                                       │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐  │  │
│  │  │ Email       │  │ Calendar    │  │ AI Engine    │  │  │
│  │  │ Poller      │  │ Service     │  │ (Claude API) │  │  │
│  │  │             │  │             │  │              │  │  │
│  │  │ • Gmail API │  │ • GCal API  │  │ • Parse      │  │  │
│  │  │ • Graph API │  │ • Graph API │  │ • Extract    │  │  │
│  │  └──────┬──────┘  └──────┬──────┘  │ • Schedule   │  │  │
│  │         │                │         └──────┬───────┘  │  │
│  │         ▼                ▼                ▼          │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │              SQLite (sqlx)                      │ │  │
│  │  │  emails · events · tasks · preferences · state  │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  │         │                                            │  │
│  │  ┌──────┴──────┐  ┌──────────────┐                   │  │
│  │  │ OAuth +     │  │ System Tray  │                   │  │
│  │  │ Keyring     │  │ + Notifs     │                   │  │
│  │  └─────────────┘  └──────────────┘                   │  │
│  └────────────────────────┬──────────────────────────────┘  │
│                           │ Tauri commands + events          │
│  ┌────────────────────────▼──────────────────────────────┐  │
│  │                 Svelte 5 Frontend                     │  │
│  │                                                       │  │
│  │  ┌──────────┐  ┌──────────┐  ┌─────────────────────┐ │  │
│  │  │ Pending  │  │ Schedule │  │ Settings &          │ │  │
│  │  │ Actions  │  │ Overview │  │ Account Management  │ │  │
│  │  │ (approve │  │ (today/  │  │ (connect accounts,  │ │  │
│  │  │  /decline│  │  week)   │  │  preferences, rules)│ │  │
│  │  └──────────┘  └──────────┘  └─────────────────────┘ │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Data flow:**
1. Rust email poller fetches new emails on a schedule (configurable, e.g. every 2 min)
2. New emails are stored in SQLite and sent to Claude for analysis
3. Claude returns structured data: detected events, tasks, deadlines
4. Results are stored as "pending actions" in SQLite
5. User is notified (system tray notification or badge)
6. User opens dashboard, reviews pending actions, approves/declines
7. Approved events are written to the correct calendar via the provider's API

---

## Implementation Stages

### Stage 1: Project Scaffold & Dev Environment

**Goal:** A working Tauri v2 app that builds, runs, and shows a hello-world
window with system tray icon. CI pipeline is green.

**Work:**
- Initialise Tauri v2 project with Svelte 5 + TypeScript template
- Configure TypeScript `strict: true`
- Set up Tailwind CSS v4
- Set up Rust workspace with module stubs: `oauth`, `email`, `calendar`,
  `ai`, `db`, `scheduler`
- Configure SQLite with sqlx + initial migration (empty schema)
- Add system tray with placeholder menu
- Set up CI: GitHub Actions with `cargo clippy`, `cargo test`, `biome check`,
  `vitest run`, `tauri build`

**Verification:**
- `pnpm tauri dev` launches the app with a window and tray icon
- `cargo clippy -- -D warnings` passes with zero warnings
- `cargo test` passes
- `pnpm run check` (Svelte check + Biome) passes
- GitHub Actions CI builds successfully on macOS (at minimum)

**Acceptance criteria:**
- [x] App launches on macOS with system tray icon
- [x] Frontend renders a placeholder page
- [x] SQLite database is created on first launch
- [x] CI pipeline runs all checks and builds succeed
- [x] All configs committed: tsconfig (strict), clippy (deny warnings),
      biome, tailwind

---

### Stage 2: OAuth & Multi-Account Connection

**Goal:** Users can connect their Google and Microsoft accounts. Tokens are
stored securely. Token refresh works silently.

**Work:**
- Implement OAuth 2.0 Authorization Code flow with PKCE for Google
  (Gmail + Calendar scopes)
- Implement OAuth 2.0 Authorization Code flow with PKCE for Microsoft
  (Graph API: Mail.Read + Calendars.ReadWrite)
- Use Tauri deep links or localhost redirect for OAuth callbacks
- Store access/refresh tokens in OS keychain via `keyring`
- Implement silent token refresh (intercept 401, refresh, retry)
- Build "Accounts" settings page in Svelte: connect/disconnect accounts,
  show connection status
- SQLite schema: `accounts` table (provider, email, scopes, connected_at)

**Verification:**
- Connect a Google account → token stored in macOS Keychain, visible in
  Keychain Access
- Connect a Microsoft account → same
- Kill and restart the app → accounts still connected, no re-auth needed
- Wait for token expiry (or simulate) → token refreshes silently
- Disconnect an account → token removed from keychain, row removed from DB

**Acceptance criteria:**
- [x] Google OAuth flow works: user clicks "Connect Google" → browser
      opens → consent → redirect back → account shown as connected
- [x] Microsoft OAuth flow works: same flow
- [x] Multiple accounts of the same provider can be connected
- [x] Tokens are never stored in SQLite or on disk in plaintext
- [x] Token refresh works without user interaction
- [x] Disconnect removes all stored credentials
- [x] Unit tests for token refresh logic and error handling

---

### Stage 3: Email Ingestion

**Goal:** The app reads emails from all connected accounts in the background.
Emails are stored locally for processing. The user sees nothing yet — this
is the invisible engine.

**Work:**
- Implement Gmail API client (Rust + reqwest): list messages, get message
  detail, handle pagination
- Implement Microsoft Graph API client (Rust + reqwest): same
- Define a provider-agnostic `Email` struct (from, to, cc, subject, body,
  date, thread_id, attachments metadata, provider, account_id)
- Build a provider trait/interface so Gmail and Graph clients are
  interchangeable
- Background poller using tokio: poll each account on a configurable
  interval (default 2 min)
- Track last-synced state per account (Gmail: historyId, Graph: deltaLink)
  so we only fetch new emails
- SQLite schema: `emails` table, `sync_state` table
- Deduplication logic: detect same email arriving in multiple accounts

**Verification:**
- Connect a Gmail account → after polling interval, new emails appear in
  SQLite (inspect with `sqlite3` CLI)
- Connect an Outlook account → same
- Send a test email → it appears in the DB within the polling interval
- Restart the app → only new emails since last sync are fetched (no
  full re-sync)
- Send same email to both accounts → stored once, marked as deduplicated

**Acceptance criteria:**
- [x] Gmail client fetches emails correctly (subject, body, sender, date)
- [x] Microsoft Graph client fetches emails correctly
- [x] Background polling runs without blocking the UI
- [x] Incremental sync: only new emails are fetched after first run
- [x] Cross-account deduplication works by Message-ID header matching
- [x] Provider trait allows adding new providers without modifying core logic
- [x] Unit tests for email parsing, deduplication, sync state tracking
- [x] Integration tests against live APIs with test accounts

---

### Stage 4: Calendar Integration (Read + Write)

**Goal:** The app reads all events from connected calendars and can write
new events back. Cross-calendar conflict detection works.

**Work:**
- Implement Google Calendar API client: list calendars, list events (with
  time range), create event, update event
- Implement Microsoft Graph Calendar client: same operations
- Provider-agnostic `CalendarEvent` struct (title, start, end, location,
  participants, calendar_id, provider, account_id, timezone)
- Calendar sync: fetch upcoming events (rolling 2-week window) on interval
- Conflict detection: given a proposed time range, check all calendars
  across all accounts for overlaps
- Calendar routing logic: determine which calendar an event should be
  created on based on the source account/context
- SQLite schema: `calendars` table, `events` table

**Verification:**
- Connect accounts → events from Google Calendar and Microsoft Calendar
  appear in SQLite
- Create a test event via the app → it appears in the correct provider's
  calendar (verify in Google Calendar / Outlook web)
- Create two events at the same time on different calendars → conflict
  detection reports the overlap
- Event in BST is correctly displayed/stored for a user in a different
  timezone

**Acceptance criteria:**
- [x] Google Calendar events are read correctly (title, time, timezone,
      participants, location)
- [x] Microsoft Calendar events are read correctly
- [x] Events can be created on the correct provider's calendar
- [x] Conflict detection works across providers (Google event conflicts
      with Microsoft event)
- [x] Timezone handling: events are stored in UTC internally, displayed in
      user's local timezone
- [x] Calendar sync only fetches the rolling window, not entire history
- [x] Unit tests for conflict detection algorithm
- [x] Integration tests: create event → verify via API → delete

---

### Stage 5: AI Context Engine (Claude Integration)

**Goal:** New emails are analysed by Claude to extract events, tasks,
deadlines, and scheduling signals. Results are stored as structured
"pending actions."

**Work:**
- Define structured output schemas for Claude responses:
  - `DetectedEvent` (title, proposed_time, location, participants, source)
  - `DetectedTask` (description, deadline, estimated_duration, priority)
  - `DetectedDeadline` (description, date, source_document)
- Build prompt templates that provide email content + user's current
  schedule context to Claude
- Use Claude API with tool use / structured output to get reliable
  JSON responses
- Implement a processing pipeline: new email → Claude analysis → store
  pending actions in SQLite
- Rate limiting and cost control: batch emails, use Haiku for simple
  classification, Sonnet for complex scheduling reasoning
- Handle edge cases: emails that are clearly not scheduling-relevant
  (newsletters, spam) should be filtered cheaply before hitting Claude

**Verification:**
- Send a test email "Let's meet Thursday 3pm at Costa Coffee on Strand" →
  a `DetectedEvent` is created with correct title, time, location
- Send an email "Please submit the report by March 28th" → a
  `DetectedTask` is created with correct deadline
- Send a newsletter → no pending action is created (filtered out)
- Check Claude API usage dashboard → token usage is reasonable (not sending
  entire email histories)

**Acceptance criteria:**
- [x] Emails containing meeting proposals produce `DetectedEvent` with
      correct fields
- [x] Emails containing tasks/deadlines produce `DetectedTask` with
      correct deadline and estimated duration
- [x] Non-scheduling emails (newsletters, notifications, ads) are filtered
      without calling Claude (or with a cheap Haiku classification call)
- [x] Structured output is validated — malformed Claude responses are
      handled gracefully (retry or skip, never crash)
- [x] Thread context: a reply "Tuesday works" is linked to the original
      email proposing Tuesday
- [x] Cost control: average cost per email processed is <$0.01 for Haiku
      classification, <$0.05 for Sonnet analysis
- [x] Unit tests with fixture emails covering: meeting proposals, task
      assignments, deadline mentions, false positives
- [x] Pending actions are stored in SQLite with source email reference

---

### Stage 6: Dashboard UI & Approval Flow

**Goal:** The user can see pending actions, review them, approve or decline,
and see a lightweight schedule overview. System tray notifications alert
when new actions need attention.

**Work:**
- **Pending Actions view** — list of detected events/tasks awaiting approval.
  Each card shows: what was detected, source email snippet, proposed
  calendar, detected time/deadline. Actions: approve, decline, edit before
  approving.
- **Schedule Overview** — today/this-week view showing all calendar events
  across all accounts (read from local SQLite cache). Highlights conflicts
  and gaps.
- **System tray integration:**
  - Badge/count of pending actions
  - Click tray icon → open/focus the dashboard window
  - Native notification when a new actionable item is detected
- Tauri event bridge: Rust emits events (`new-pending-action`,
  `schedule-updated`) → Svelte listens and updates reactively
- Approval flow: user approves → Rust writes event to provider calendar →
  status updated in SQLite → UI reflects change

**Verification:**
- Send a meeting-proposal email → notification appears → click opens
  dashboard → pending action card is shown with correct details
- Click "Approve" → event appears on the correct calendar (verify in
  provider's web UI)
- Click "Decline" → action is dismissed, not written to any calendar
- Click "Edit" → modify time/title → approve → event created with
  modifications
- Schedule overview shows events from all connected calendars

**Acceptance criteria:**
- [x] Pending actions are displayed with source context (email snippet,
      sender, detected fields)
- [x] Approve writes to the correct provider's calendar within 2 seconds
- [x] Decline dismisses without side effects
- [x] Edit-before-approve allows changing time, title, calendar, duration
- [x] System tray shows pending action count
- [x] Native OS notification fires for new pending actions
- [x] Schedule overview renders events from all providers, correctly
      colour-coded by account
- [x] UI is responsive — no hang or freeze during API calls (all async)
- [x] E2E test: email → detection → notification → approve → event created

---

### Stage 7: Proactive Scheduling Intelligence

**Goal:** The app can suggest optimal time slots for tasks, protect focus
time, warn about overloaded days, and produce daily briefings.

**Work:**
- **Scheduling algorithm:** given a task (duration, deadline, priority) and
  the user's full calendar state, score candidate time slots based on:
  - No conflicts across any calendar
  - User preference alignment (working hours, deep-work windows)
  - Buffer time around existing meetings
  - Proximity to deadline (urgency weighting)
  - Workload distribution across the week
- **Focus time protection:** user configures focus blocks → scheduler
  treats them as soft-occupied, warns if a meeting tries to override
- **Daily briefing:** generate a morning summary via Claude:
  "4 meetings today, 2-hour gap at 1pm, report deadline tomorrow"
  Deliver as a notification and/or dashboard card.
- **Overload warning:** if a day exceeds N hours of meetings or M total
  tasks, surface a warning with rebalancing suggestions
- Add a "Schedule this" action to pending tasks: user clicks → app
  suggests top 3 time slots → user picks one → event created

**Verification:**
- Create a task "Prepare presentation, 2 hours, due Friday" → app
  suggests 3 time slots, all within working hours, none conflicting
- Configure "no meetings before 11am" → suggested slots respect this
- Fill Tuesday with 7 hours of meetings → app warns about overload on
  Monday evening
- Enable daily briefing → notification fires at configured time with
  accurate summary

**Acceptance criteria:**
- [x] Time slot suggestions avoid all conflicts across all calendars
- [x] Suggestions respect user preference configuration (working hours,
      meeting-free days, deep-work windows)
- [x] Buffer time is applied: no suggestion starts immediately after an
      existing meeting
- [x] At least 3 candidate slots are offered, ranked by suitability
- [x] Overload warning triggers when daily meeting load exceeds user's
      configured threshold
- [x] Daily briefing is accurate: correct event count, correct conflict
      detection, correct deadline surfacing
- [x] Unit tests for the scheduling algorithm with various calendar states
- [x] Focus time blocks are not overridden by suggestions

---

### Stage 8: User Preferences, Rules & Learning

**Goal:** Users can configure their scheduling preferences, define custom
rules, and the app learns from their behaviour over time.

**Work:**
- **Settings UI:**
  - Working hours per day of week
  - Meeting-free days
  - Deep-work / focus time windows
  - Default meeting duration
  - Buffer time between meetings
  - Default calendar per account type (work/personal)
  - Daily briefing time and toggle
  - Notification preferences
- **Custom rules engine:** simple condition → action rules stored in SQLite.
  Examples: "If sender contains @university.ac.uk → route to academic
  calendar", "If subject contains 'standup' → auto-approve"
- **Behavioural learning:**
  - Track approve/decline/edit patterns per action type
  - If user consistently edits a field (e.g., always changes suggested
    duration from 1h to 30min), adapt the default
  - If user always declines a certain type of suggestion, reduce its
    priority or suppress it
- SQLite schema: `preferences` table, `rules` table, `feedback_log` table

**Verification:**
- Set "no meetings before 11am" → all suggestions respect this
- Create rule "auto-approve events from alice@work.com" → events from
  Alice skip the pending queue and are created directly
- Decline 5 newsletter-triggered suggestions in a row → app stops
  surfacing that type (or reduces frequency)
- Edit suggested duration from 1h to 30min three times → next suggestion
  for similar tasks defaults to 30min

**Acceptance criteria:**
- [x] All preference fields are persisted across app restarts
- [x] Rules engine supports: sender matching, subject matching, account
      matching, keyword matching, with actions: auto-approve, auto-decline,
      route to specific calendar, override duration
- [x] Behavioural adaptation is measurable: before/after comparison of
      suggestion accuracy over 20+ interactions
- [x] User can reset learned preferences to defaults
- [x] Settings UI validates input (e.g., working hours end > start)
- [x] Unit tests for rules engine matching logic
- [x] Integration test: set preference → trigger relevant email → verify
      suggestion respects preference

---

### Stage 9: Attachment & Document Intelligence

**Goal:** The app extracts scheduling-relevant information from email
attachments (PDFs, documents) and surfaces them as pending actions.

**Work:**
- Download attachments from Gmail API and Graph API (respect size limits,
  e.g., skip attachments >25MB)
- Extract text from common formats: PDF (via `pdf-extract` or `lopdf`
  crate), DOCX (via `docx-rs`), plain text
- Send extracted text to Claude with a focused prompt: "Extract any dates,
  deadlines, or scheduling-relevant information from this document"
- Create `DetectedDeadline` actions from extracted dates
- Link back to source email so user has context

**Verification:**
- Email with a PDF attachment containing "submission deadline: April 15th"
  → a pending action surfaces with that deadline
- Email with a .docx meeting agenda containing session times → sessions
  are offered as calendar events
- Email with a large image attachment → skipped, no processing attempted
- Email with no scheduling-relevant attachment content → no false positives

**Acceptance criteria:**
- [x] PDF text extraction works for standard (non-scanned) PDFs
- [x] DOCX text extraction works
- [x] Dates/deadlines mentioned in attachments are detected with >80%
      accuracy on test corpus
- [x] Attachments >25MB are skipped gracefully
- [x] Non-text attachments (images, videos) are skipped
- [x] Pending actions from attachments include a reference to the source
      document and email
- [x] Cost control: attachment text is truncated to a reasonable limit
      before sending to Claude
- [x] Unit tests with sample PDFs/DOCXs containing various date formats

---

### Stage 10: Packaging, Distribution & Polish

**Goal:** The app is distributable as a native installer for macOS (and
optionally Windows). Auto-update works. The experience is polished.

**Work:**
- **macOS:**
  - `tauri build` → `.dmg` with drag-to-Applications installer
  - Code signing with Apple Developer certificate
  - Notarisation with Apple
  - Auto-start on login (tauri-plugin-autostart)
  - Auto-update via Tauri's built-in updater (GitHub Releases as backend)
- **Windows (stretch):**
  - `tauri build` → `.msi` installer
  - Code signing (optional at this stage)
- **Polish:**
  - App icon and tray icon design
  - Onboarding flow: first launch walks through account connection +
    preference setup
  - Error states: graceful handling of network failures, expired tokens,
    API rate limits
  - Loading states: skeleton UI while data loads
  - Empty states: helpful messaging when no accounts connected or no
    pending actions
- **Performance:**
  - Profile and optimise background polling (memory and CPU)
  - Ensure <50MB RAM usage during idle polling
  - App startup time <2 seconds

**Verification:**
- Build `.dmg` → install on a clean Mac → launch → onboarding flow →
  connect accounts → emails are processed → suggestions appear
- Quit and reopen → app remembers state, accounts still connected
- Check Activity Monitor → idle RAM <50MB, CPU ~0% when not actively
  polling
- Trigger auto-update → new version installs and restarts seamlessly
- Disconnect from internet → app degrades gracefully (shows cached
  schedule, queues actions for later)

**Acceptance criteria:**
- [x] macOS `.dmg` installs and runs on a clean machine (no dev tools
      required)
- [x] App is code-signed and notarised (no Gatekeeper warnings)
- [x] Auto-update downloads and installs new versions
- [x] Auto-start on login works
- [x] Onboarding flow guides new users through setup in <2 minutes
- [x] Idle memory <50MB, startup <2 seconds
- [x] Graceful offline behaviour: cached data shown, actions queued
- [x] No crashes in 48-hour continuous run test
- [x] E2E smoke test: fresh install → connect accounts → receive email →
      approve event → verify on calendar

---

## Stage Dependency Graph

```
Stage 1 (Scaffold)
  │
  ▼
Stage 2 (OAuth)
  │
  ├──────────────────┐
  ▼                  ▼
Stage 3 (Email)    Stage 4 (Calendar)
  │                  │
  └────────┬─────────┘
           ▼
         Stage 5 (AI Engine)
           │
           ▼
         Stage 6 (Dashboard & Approval)
           │
           ├──────────────────┐
           ▼                  ▼
         Stage 7 (Scheduling) Stage 9 (Attachments)
           │
           ▼
         Stage 8 (Preferences & Learning)
           │
           ▼
         Stage 10 (Packaging & Distribution)
```

Stages 3 and 4 can be developed in parallel after Stage 2.
Stages 7 and 9 can be developed in parallel after Stage 6.

---

## Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| Google OAuth verification for Gmail scope is slow (weeks) | Blocks external testers | Start the verification process in Stage 2. Use test-mode (100 users) during development. |
| Claude API costs scale unexpectedly | Budget overrun | Implement Haiku-first classification in Stage 5. Add per-account daily cost caps. Monitor in dashboard. |
| Tauri v2 system tray has edge cases on Linux/Windows | Cross-platform issues | Focus on macOS first. Test Windows/Linux in Stage 10. |
| OAuth token refresh race conditions | Users get logged out randomly | Implement token refresh mutex in Stage 2. Extensive integration testing. |
| Email providers rate-limit the app | Missed emails, degraded experience | Respect rate limits with exponential backoff. Use incremental sync (delta/history) to minimise API calls. |
| Claude structured output occasionally malformed | Crash or missed actions | Validate all Claude responses against schema. Fall back to retry or skip with logging. Never crash on bad AI output. |
