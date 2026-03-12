# AI Secretary — Feature Vision & Use Cases

A sidecar AI assistant that runs alongside the user's existing email apps
(Outlook, Gmail, etc.). It reads from email, understands the user's full
agenda across all accounts, and writes events and tasks back to the
provider's native calendar. Zero friction — the user never has to switch
email clients.

---

## Design Principles

- **Sidecar, not replacement** — the user keeps their existing email apps.
  The secretary reads emails as input and writes to native calendars as
  output. It does not present its own inbox or calendar UI.
- **Provider-agnostic** — connects to Gmail, Outlook, Google Calendar,
  Microsoft Calendar (and extensible to more).
- **Full schedule awareness** — aggregates all connected calendars and
  inboxes into an internal model of the user's state, commitments,
  and availability. Uses this to make informed decisions.
- **Assistive, not autonomous** — suggests and prepares — but the user
  approves before anything is committed to their calendar.
- **Privacy-conscious** — users control which accounts, folders, and labels
  are processed; data is not persisted beyond what is necessary.
- **Minimum friction** — no workflow changes required. The app does its job
  in the background and surfaces results when relevant.

---

## Feature Areas

### 1. Email-Aware Context Engine

Read emails across all connected accounts to build a continuous
understanding of the user's world — without replacing the email client.

**Capabilities:**

- **Continuous ingestion** — monitor new emails across all accounts in the
  background. Extract scheduling-relevant signals: meeting requests,
  deadlines mentioned, tasks assigned, follow-ups expected.
- **Action item extraction** — pull out to-dos, deadlines, and commitments
  from email bodies. Surface them as actionable items the user can schedule.
- **Thread context** — understand conversation history so that a reply
  saying "Tuesday works" is linked back to the original proposal.
- **Recurring pattern detection** — learn patterns (weekly reports, monthly
  invoices, regular meeting requests) and anticipate them.
- **Cross-account deduplication** — if the same meeting request arrives in
  both work Outlook and personal Gmail, treat it as one item.

**Use cases:**

- Your manager emails "Can you prepare the Q3 budget review by next
  Friday?" — the secretary extracts the task and deadline and offers to
  schedule a focus block for it.
- A client CCs your work and personal email on a meeting request — the
  secretary recognises the duplicate and handles it once.
- Every Monday your team lead sends a status request. The secretary
  recognises the pattern and reminds you on Sunday evening.

---

### 2. Smart Event Extraction & Calendar Sync

Detect scheduling information in emails and create events on the correct
provider's calendar.

**Capabilities:**

- **Date/time/location parsing** — understands natural language like "let's
  meet next Thursday afternoon at the usual place".
- **Participant resolution** — maps email addresses to contacts and calendar
  identities across providers.
- **Correct calendar routing** — if a meeting was proposed in a work email,
  create it on the work calendar. If it's a personal dinner invite, put it
  on the personal calendar.
- **Cross-calendar conflict detection** — before creating an event, checks
  all connected calendars (work + personal + any others) for conflicts.
- **Timezone intelligence** — infers and normalises timezones from sender
  location, email headers, and explicit mentions.

**Use cases:**

- A vendor emails your work account "Can we do a call March 20th at 2pm
  PST?" — the secretary creates the event on your work Microsoft Calendar
  and confirms no conflict with your personal Google Calendar.
- A friend emails a dinner invite to your personal Gmail while you have a
  late work meeting — the secretary flags the conflict and suggests
  alternative times.
- Someone proposes a meeting across timezones. The secretary normalises
  "3pm London time" to your local timezone and creates the event correctly.

---

### 3. Proactive Scheduling & Time Management

Actively help the user plan and protect their time, using full awareness
of all calendars.

**Capabilities:**

- **Time slot suggestion** — given a task description and estimated duration,
  suggest optimal time slots based on all calendars, existing commitments,
  and user preferences (e.g., "I prefer deep work in the mornings").
- **Task-aware scheduling** — understand task dependencies, deadlines, and
  priorities to suggest when to work on what.
- **Buffer & travel time** — automatically account for travel time between
  in-person meetings, buffer time between calls, and break time.
- **Focus time protection** — reserve blocks for deep work and defend them
  from meeting creep.
- **Meeting time negotiation** — when scheduling with others, find mutually
  available slots across all participants (using free/busy data from
  calendar APIs).
- **Workload balancing** — warn when a day or week is overloaded and suggest
  redistributing tasks.
- **Daily/weekly briefing** — proactively summarise the upcoming schedule,
  highlight conflicts, and suggest adjustments.

**Use cases:**

- You say "I need 3 hours to prepare the quarterly report, due Friday" —
  the secretary finds a 3-hour block on Wednesday morning (your preferred
  deep-work time), avoids conflicting with any calendar, and creates a
  focus block on the appropriate calendar.
- You have 6 meetings on Tuesday. The secretary warns you on Monday and
  suggests rescheduling the two lowest-priority ones.
- You need to meet with 3 colleagues. The secretary checks free/busy across
  all their calendars and proposes the top 3 available slots.
- Every morning at 8am you get a briefing: "You have 4 meetings today,
  a 2-hour gap at 1pm good for the report, and a deadline tomorrow for
  the contract review."

---

### 4. Attachment & Document Intelligence

Extract scheduling-relevant information from email attachments.

**Capabilities:**

- **Deadline extraction from documents** — detect signing deadlines,
  submission dates, and expiry dates buried in attached contracts or forms.
- **Key date surfacing** — scan attachments for dates that should become
  calendar events or reminders.
- **Follow-up tracking** — if a document requires action (signature, review,
  approval), track it and create a reminder on the calendar.

**Use cases:**

- Legal sends a 40-page contract with a signing deadline on page 12 — the
  secretary surfaces "Contract requires signature by March 25th" and
  creates a calendar reminder 3 days before on your work calendar.
- An event organiser attaches a PDF agenda with session times — the
  secretary extracts the relevant sessions and offers to block them on
  your calendar.

---

### 5. User Preferences & Learning

Adapt to the user over time to make better scheduling and prioritisation
decisions.

**Capabilities:**

- **Preference configuration** — preferred working hours, meeting-free days,
  deep-work windows, default meeting durations, travel time defaults,
  which calendar to use for which type of event.
- **Account-aware context** — understand that a work account has different
  priorities and norms than a personal account.
- **Behavioral learning** — observe how the user accepts, declines, or
  reschedules suggestions, then adapt accordingly.
- **Custom rules** — user-defined rules like "never schedule meetings before
  10am" or "always leave 15 minutes between back-to-back calls".
- **Feedback loop** — when the user overrides a suggestion, learn from it.

**Use cases:**

- You consistently move suggested focus blocks from morning to afternoon.
  The secretary learns your actual preference and adjusts.
- You mark certain senders as high-priority. Emails from them get
  immediate attention for scheduling.
- You set a rule: "Fridays are meeting-free." The secretary respects this
  and pushes back on Friday meeting proposals.

---

## How It Fits Into the User's Workflow

```
┌─────────────────────────────────────────────────────────┐
│                    User's existing apps                  │
│  ┌───────────┐  ┌──────────┐  ┌───────────────────────┐ │
│  │  Outlook   │  │  Gmail   │  │  Calendar apps        │ │
│  │  (email)   │  │  (email) │  │  (Google, Microsoft)  │ │
│  └─────┬─────┘  └────┬─────┘  └───────────┬───────────┘ │
└────────┼──────────────┼────────────────────┼────────────┘
         │ read         │ read               ▲ write events
         ▼              ▼                    │
┌─────────────────────────────────────────────────────────┐
│                    AI Secretary                          │
│                                                         │
│  ┌─────────────┐  ┌──────────┐  ┌────────────────────┐  │
│  │ Context     │  │ Schedule │  │ User Preferences   │  │
│  │ Engine      │→ │ Planner  │← │ & Learning         │  │
│  │ (reads mail)│  │          │  │                    │  │
│  └─────────────┘  └──────────┘  └────────────────────┘  │
│                        │                                 │
│                   suggestions                            │
│                        ▼                                 │
│                  ┌───────────┐                           │
│                  │ User      │                           │
│                  │ approves  │                           │
│                  └───────────┘                           │
└─────────────────────────────────────────────────────────┘
```

The user never leaves Outlook or Gmail. The secretary works in the
background and only surfaces when it has something actionable:
a meeting to create, a conflict to resolve, or a schedule to optimise.

---

## Out of Scope (for now)

- **Unified inbox / email client UI** — users keep their existing email apps.
- **Smart reply & draft assistance** — auto-drafting email replies.
- **Chat / messaging platforms** (Slack, Teams) — potential future addition.
- **Phone / voice integration** — text-based only for now.
- **CRM integration** — may be added later for sales/BD use cases.

---

## Open Questions

- What is the right UX surface for the secretary itself? Menubar app?
  Web dashboard? Notification-based (push only)? A combination?
- How should the user authenticate multiple accounts securely?
- What is the data retention policy — should the secretary maintain a
  persistent memory of past emails, or re-fetch on demand?
- How to handle shared/delegated calendars?
- Should the app have a conversational interface (chat with your secretary)
  or a structured task-based UI?
