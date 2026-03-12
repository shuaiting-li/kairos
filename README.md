# Kairos

A sidecar AI secretary that runs alongside your existing email apps (Outlook, Gmail), understands your agenda across all accounts, and writes events and tasks back to your native calendars (Google Calendar, Microsoft Calendar).

You never switch apps. Kairos works in the background — reading emails, extracting scheduling signals, and surfacing actionable suggestions. You approve, it executes.

## How It Works

```
  Outlook / Gmail              Google Calendar / Microsoft Calendar
       │ read                              ▲ write events
       ▼                                   │
┌─────────────────────────────────────────────────┐
│                    Kairos                        │
│                                                  │
│  Email Context Engine → Claude AI Analysis       │
│       → Pending Actions → User Approves          │
│            → Event Created on Calendar           │
└─────────────────────────────────────────────────┘
```

1. **Reads** emails from all connected accounts in the background
2. **Analyzes** them with Claude to extract meetings, deadlines, tasks, and scheduling signals
3. **Suggests** calendar events, focus blocks, and reminders
4. **Waits** for your approval — nothing is committed without your say-so
5. **Writes** approved events to the correct provider calendar

## Features

- **Email-aware context engine** — continuous ingestion across Gmail and Outlook, action item extraction, thread context, recurring pattern detection, cross-account deduplication
- **Smart event extraction** — natural language date/time parsing, timezone intelligence, correct calendar routing (work email → work calendar), cross-calendar conflict detection
- **Proactive scheduling** — time slot suggestions based on preferences, focus time protection, buffer/travel time, workload balancing, daily briefings
- **Document intelligence** — extract deadlines and key dates from email attachments (contracts, agendas, forms)
- **Learning** — adapts to your preferences over time: working hours, meeting-free days, deep-work windows, priority senders

## Design Principles

- **Sidecar, not replacement** — you keep your email apps; Kairos reads from them and writes to your calendars
- **Assistive, not autonomous** — suggests and prepares, but you approve before anything touches your calendar
- **Provider-agnostic** — connects to Gmail, Outlook, Google Calendar, Microsoft Calendar
- **Privacy-conscious** — data stays local (SQLite), OAuth tokens in OS keychain, you control which accounts and folders are processed

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 |
| Backend | Rust (tokio, reqwest, tauri-plugin-sql/SQLite) |
| Frontend | SvelteKit (Svelte 5) + TypeScript (strict) + Tailwind CSS v4 |
| AI | Claude API (Anthropic) |
| Auth | OAuth 2.0 with PKCE, tokens in OS keychain |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (via `rustup` or `brew install rustup`)
- [Node.js](https://nodejs.org/) (v22+)
- [pnpm](https://pnpm.io/) (`brew install pnpm`)

### Development

```bash
pnpm install
pnpm tauri dev
```

### Build

```bash
pnpm tauri build     # outputs .dmg on macOS
```

### Checks

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm run check       # Biome + svelte-check
pnpm vitest run      # frontend tests
```

## Status

The project is in **Stage 1** (scaffold) of a 10-stage implementation plan. See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for the full roadmap and [FEATURES.md](FEATURES.md) for the detailed product vision.

## License

MIT
