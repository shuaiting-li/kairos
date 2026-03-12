# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Kairos is a **sidecar AI secretary** — a desktop app that runs alongside Outlook/Gmail, reads emails to understand the user's agenda, and writes events/tasks back to native calendars (Google Calendar, Microsoft Calendar). It is not an email client replacement. The user never switches apps.

## Tech Stack

- **Desktop shell:** Tauri v2
- **Backend:** Rust (tokio async runtime, reqwest for HTTP, tauri-plugin-sql for SQLite)
- **Frontend:** SvelteKit (Svelte 5, adapter-static SPA mode) + TypeScript (strict) + Tailwind CSS v4
- **AI:** Claude API (Anthropic) for email parsing and scheduling intelligence
- **Auth:** OAuth 2.0 with PKCE for Google and Microsoft, tokens in OS keychain

## Build & Run Commands

```bash
# Prerequisites: Rust (rustup via brew), pnpm, Node.js

# Install dependencies
pnpm install

# Development (launches app with hot reload)
pnpm tauri dev

# Production build (outputs .dmg on macOS)
pnpm tauri build

# Rust checks
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend checks (run svelte-kit sync first — generates .svelte-kit/tsconfig.json)
pnpm run check          # Biome CI + svelte-kit sync + svelte-check
pnpm vitest run         # Run all frontend tests
pnpm vitest run <file>  # Run a single test file

# Formatting
pnpm run check:fix      # Biome auto-fix
pnpm run format         # Biome format only
```

## Architecture

The frontend is a SvelteKit SPA (adapter-static, SSR disabled). Routing uses `src/routes/` with `+page.svelte` / `+layout.svelte` conventions. Tailwind v4 is loaded via `@tailwindcss/vite` plugin — no `tailwind.config` file; use `@import "tailwindcss"` in `src/app.css`.

```
src/                     SvelteKit frontend (routes, layouts, components)
src-tauri/src/           Rust backend
  lib.rs                 App setup: system tray, plugin registration, module wiring
  main.rs                Entry point (calls lib::run)
  oauth.rs               OAuth 2.0 flows for Google and Microsoft
  email.rs               Email ingestion (Gmail API, Microsoft Graph API)
  calendar.rs            Calendar read/write (Google Calendar, Microsoft Graph)
  ai.rs                  Claude API integration for email analysis
  db.rs                  SQLite migrations (add new migrations to db::migrations())
  scheduler.rs           Background polling and task scheduling
```

**Data flow:** Email poller (Rust) → SQLite → Claude analysis → pending actions → user approves in UI → event written to provider calendar via API.

All API calls, OAuth tokens, and database access live in Rust. The Svelte frontend only renders UI and communicates via Tauri commands/events.

**Database:** SQLite via `tauri-plugin-sql`. Migrations are registered in `db::migrations()` and run automatically on app start via `tauri_plugin_sql::Builder::add_migrations("sqlite:kairos.db", ...)`.

## Key Conventions

- TypeScript: `strict: true` — no `any`, no implicit returns
- Rust: `cargo clippy -- -D warnings` — all warnings are errors
- Formatting: Biome with **tabs** (indentStyle: tab)
- Biome covers `.ts`, `.js`, `.json` only — `.svelte` files are linted/checked by `svelte-check`
- Package manager: pnpm (not npm/yarn)
- SQLite via `tauri-plugin-sql` — database lives in the app data directory, not the project directory
- OAuth tokens stored in OS keychain via `keyring` crate — never in SQLite or on disk

## Scope Boundaries

**In scope:** Email context engine, event extraction, proactive scheduling/time-slot suggestion, attachment/document intelligence (deadline extraction), user preferences and learning.

**Out of scope:** Smart reply / email drafting, unified inbox UI, chat platform integration (Slack/Teams), voice, CRM.

## Implementation Status

The project is in Stage 1 (scaffold). See `IMPLEMENTATION_PLAN.md` for the full 10-stage roadmap and `FEATURES.md` for the product vision.
