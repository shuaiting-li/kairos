# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Kairos is a **sidecar AI secretary** — a desktop app that runs alongside Outlook/Gmail, reads emails to understand the user's agenda, and writes events/tasks back to native calendars (Google Calendar, Microsoft Calendar). It is not an email client replacement. The user never switches apps.

## Tech Stack

- **Desktop shell:** Tauri v2
- **Backend:** Rust (tokio async runtime, reqwest for HTTP, sqlx/tauri-plugin-sql for SQLite)
- **Frontend:** Svelte 5 + TypeScript (strict) + Tailwind CSS v4
- **AI:** Claude API (Anthropic) for email parsing and scheduling intelligence
- **Auth:** OAuth 2.0 with PKCE for Google and Microsoft, tokens in OS keychain

## Build & Run Commands

```bash
# Prerequisites: Rust (rustup), pnpm, Node.js

# Install dependencies
pnpm install

# Development (launches app with hot reload)
pnpm tauri dev

# Production build (outputs .dmg on macOS)
pnpm tauri build

# Rust checks
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend checks
pnpm run check          # Biome lint + format check
pnpm vitest run         # Run all frontend tests
pnpm vitest run <file>  # Run a single test file
```

## Architecture

```
src/                     Svelte 5 frontend (dashboard UI, approval flows, settings)
src-tauri/src/           Rust backend
  lib.rs                 App setup: system tray, plugin registration, module wiring
  main.rs                Entry point (calls lib::run)
  oauth.rs               OAuth 2.0 flows for Google and Microsoft
  email.rs               Email ingestion (Gmail API, Microsoft Graph API)
  calendar.rs            Calendar read/write (Google Calendar, Microsoft Graph)
  ai.rs                  Claude API integration for email analysis
  db.rs                  SQLite schema, migrations, queries
  scheduler.rs           Background polling and task scheduling
```

**Data flow:** Email poller (Rust) → SQLite → Claude analysis → pending actions → user approves in UI → event written to provider calendar via API.

All API calls, OAuth tokens, and database access live in Rust. The Svelte frontend only renders UI and communicates via Tauri commands/events.

## Key Conventions

- TypeScript: `strict: true` in tsconfig — no `any`, no implicit returns
- Rust: `cargo clippy -- -D warnings` — all warnings are errors
- Linting/formatting: Biome (not ESLint/Prettier)
- Package manager: pnpm (not npm/yarn)
- SQLite via `tauri-plugin-sql` — database lives in the app data directory, not the project directory
- OAuth tokens stored in OS keychain via `keyring` crate — never in SQLite or on disk

## Scope Boundaries

**In scope:** Email context engine, event extraction, proactive scheduling/time-slot suggestion, attachment/document intelligence (deadline extraction), user preferences and learning.

**Out of scope:** Smart reply / email drafting, unified inbox UI, chat platform integration (Slack/Teams), voice, CRM.

## Implementation Status

The project is in Stage 1 (scaffold). See `IMPLEMENTATION_PLAN.md` for the full 10-stage roadmap and `FEATURES.md` for the product vision.
