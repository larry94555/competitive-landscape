# Landscape — Architecture Explained

> Companion to [ARCHITECTURE.md](ARCHITECTURE.md). That document states *what* the stack is.
> This one explains *why*, for every named technology, and what we gave up by choosing it.

## How to read this document

Every entry follows the same four-part shape:

- **What it is** — a plain-language definition, assuming no familiarity with the tool.
- **Alternatives** — the most popular and/or most respected options we could have picked
  instead. Not an exhaustive survey; the two or three a competent reviewer would actually
  propose.
- **Why this choice** — the specific reasoning for *this* product.
- **Trade-off** — what the choice costs, what the alternative would have bought, and why
  the balance favors the choice made.

Depth is proportional to consequence. A decision that would take a week to reverse
(the database, the inference runtime, the report schema) gets full treatment. A decision
that would take an hour (a diffing crate) gets a row in a table. Entries marked
**⚠ Genuinely arguable** are ones where a reasonable senior engineer could pick differently;
those are collected in §11 with the conditions that should trigger a re-think.

A recurring theme, stated once here rather than repeated forty times: **this is a solo
product with a fixed-cost inference budget and no operations team.** That fact decides most
of the close calls. A tool that is 10% better but adds a second runtime, a monthly bill, or
a new failure mode usually loses.

---

## 1. Frontend

### 1.1 Vite (build tool)

- **What it is** — The program that turns source files into what a browser loads. In
  development it serves modules natively so edits appear in milliseconds; for production it
  bundles, minifies, and splits code into cacheable chunks.
- **Alternatives** — **Next.js** (a full React framework that includes a build system and a
  Node server); **Create React App** (the old default, now unmaintained); **webpack** or
  **Rspack** configured by hand; **Parcel**.
- **Why this choice** — We need a static single-page app served by Caddy next to the Rust
  binary, and nothing more. Vite does exactly that with near-zero configuration, has become
  the default for non-framework React, and its dev server keeps the feedback loop tight
  during the many small UI iterations this product needs.
- **Trade-off** — Vite gives us no server-side rendering. Next.js would, and SEO matters
  here (shared reports and ~200 programmatic comparison pages, per Phase 6). We deliberately
  take the SSR responsibility into the Rust backend instead — see §1.11, which is the real
  decision. Vite alone is simply the smallest thing that satisfies the client half.

### 1.2 TypeScript with `strict: true` and `noUncheckedIndexedAccess`

- **What it is** — TypeScript adds a type system to JavaScript, checked at build time and
  erased at runtime. `strict` turns on the full set of safety checks (most importantly, null
  and undefined must be handled explicitly). `noUncheckedIndexedAccess` additionally makes
  `array[0]` typed as *possibly undefined*, because indexing past the end of an array is a
  runtime reality the default types pretend away.
- **Alternatives** — Plain JavaScript with JSDoc annotations; TypeScript with `strict: false`;
  runtime-only validation (Zod everywhere, no static types).
- **Why this choice** — The report schema *is* the product. A seven-section structure with
  nested claims, optional payloads, and three "not found" states is exactly the shape where
  a loose type system lets a rendering bug ship. Because the types are generated from the
  Rust schema (§1.14), strict mode turns a backend schema change into a compile error rather
  than a blank section in production.
- **Trade-off** — Strict mode costs real time: more explicit narrowing, more `undefined`
  handling, occasional fights with third-party types. `noUncheckedIndexedAccess` in
  particular is unpopular because it makes ordinary array access verbose. We accept that
  friction specifically because array and record access over model-generated data is where
  silent failures live. Non-strict TS would be faster to write and would find perhaps half
  the bugs.

### 1.3 React 19 (function components and hooks)

- **What it is** — The library that renders UI as a function of state. Version 19 adds the
  `use()` hook for reading promises and context during render, improved Suspense behavior,
  and Actions for form/async transitions.
- **Alternatives** — **Vue**, **Svelte**, and **SolidJS** are the credible competitors;
  Svelte and Solid are both meaningfully faster and smaller at runtime.
- **Why this choice** — React is a stated constraint of the project. It is also, independent
  of the constraint, the pragmatic pick: the largest ecosystem, the deepest hiring and
  agent-assistance pool (coding agents produce markedly better React than SolidJS), and the
  libraries we want — TanStack Query, Radix, Playwright integration — all treat React as the
  first-class target.
- **Trade-off** — React carries more runtime overhead than Svelte or Solid and its rendering
  model requires more care (memoization, effect dependencies) to keep fast. For an app whose
  performance bottleneck is a language model on a server, that overhead is irrelevant. We are
  paying a small runtime cost for a large ecosystem and iteration-speed benefit.

### 1.4 TanStack Router  ⚠ Genuinely arguable

- **What it is** — A routing library that maps URLs to components. Its distinguishing
  feature is *type-safe routing*: route paths, path parameters, and search-query parameters
  are all typed, so `/a/:analysisId` produces a typed `analysisId` and a typo in a link is a
  compile error.
- **Alternatives** — **React Router v7** (the incumbent, now merged with Remix, by far the
  most widely used); **Next.js file-based routing** (not applicable without Next);
  **Wouter** (minimal, ~2KB).
- **Why this choice** — Search-parameter type safety is worth more here than usual, because
  the app leans on URL state instead of client state (see §1.6) — share links, PDF variant
  selection, KB search queries and tag filters all live in the URL. TanStack Router validates
  and types those parameters natively.
- **Trade-off** — React Router is the safer institutional choice: more tutorials, more
  StackOverflow answers, better coding-agent familiarity, and a much longer track record.
  TanStack Router is younger and its type-heavy API can produce dense inference errors. For a
  twelve-route application the practical difference is small either way, which is precisely
  why this is listed as arguable — if early implementation friction shows up, switching to
  React Router is a day of work, not a rewrite.

### 1.5 TanStack Query (server state)

- **What it is** — A library for data that lives on the server: it fetches, caches,
  deduplicates concurrent requests for the same key, retries failures, revalidates stale
  data, and invalidates caches after mutations. It removes the `useEffect` + `useState` +
  loading-flag pattern that every React app otherwise reinvents badly.
- **Alternatives** — **SWR** (smaller, simpler, from Vercel); **RTK Query** (excellent, but
  arrives attached to Redux Toolkit); hand-rolled fetching in effects; **Apollo Client** (if
  we were using GraphQL, which we are not).
- **Why this choice** — The non-streaming half of the app — watchlists, KB threads, billing
  state, usage counters, analysis history — is entirely conventional
  fetch/cache/mutate/invalidate work, and this is the best-in-class tool for it. It also
  gives us request deduplication for free, which matters when several components read the
  same usage counter.
- **Trade-off** — It is a real dependency with its own concepts (query keys, staleness,
  invalidation) to learn. SWR would be lighter and cover perhaps 80% of the need. We choose
  the heavier tool because mutation invalidation (upgrade a plan → usage meter, plan cards,
  and watch limits must all refresh consistently) is where SWR gets hand-rolled and buggy.
  Note that the *streamed analysis* does not use TanStack Query at all — it is an SSE
  subscription (§1.12), which is a different problem.

### 1.6 Zustand (client state)

- **What it is** — A minimal state container: create a store, read it with a hook, update it
  by calling a function. Roughly one kilobyte, no providers, no reducers, no action types.
- **Alternatives** — **Redux Toolkit** (the industry standard, with excellent devtools);
  **Jotai** or **Recoil** (atom-based); **React Context + `useReducer`** (no dependency at
  all); **MobX**.
- **Why this choice** — There is very little true client state in this product: the composer
  draft, the streaming buffer for the in-flight analysis, and some UI toggles. Everything
  else is server state (TanStack Query) or URL state (the router). Zustand is the smallest
  tool that handles that without ceremony, and critically it lets the SSE stream write into a
  store outside React's render cycle without triggering re-render storms.
- **Trade-off** — Redux Toolkit would give better time-travel debugging and a more
  disciplined structure for a large team. It would also mean boilerplate for three pieces of
  state and a mental model that outweighs the problem. Plain Context would avoid the
  dependency but re-renders every consumer on every token delta during streaming — which is
  the specific reason Context is wrong here and Zustand is right.

### 1.7 Tailwind CSS

- **What it is** — A CSS framework of small single-purpose utility classes
  (`flex`, `gap-4`, `text-sm`) composed directly in markup, with unused classes stripped at
  build time.
- **Alternatives** — **CSS Modules** (scoped plain CSS, no dependency); **styled-components**
  or **Emotion** (CSS-in-JS); a component library with styling included such as
  **MUI**, **Chakra**, or **Mantine**; **shadcn/ui** (which is Tailwind + Radix, copied into
  your repo rather than installed).
- **Why this choice** — A solo founder iterating daily on a small, opinionated interface
  benefits most from editing layout in the same file as the markup, with no naming decisions
  and no separate stylesheet to keep in sync. The output CSS is small because unused
  utilities are purged.
- **Trade-off** — Markup becomes visually noisy, and Tailwind is genuinely divisive for that
  reason. A component library like MUI would deliver a polished look faster on day one — but
  it would also make the product look like every other MUI app, and this product's
  credibility depends on a distinctive, calm, typography-led report design. We take verbose
  markup in exchange for full design control and no framework fighting.

### 1.8 Radix UI (unstyled accessible primitives)

- **What it is** — A library of interactive component behaviors — dialog, popover, tooltip,
  dropdown, toast — with correct keyboard handling, focus trapping, and ARIA semantics, and
  *no* visual styling. You bring your own CSS.
- **Alternatives** — Building these from scratch; **Headless UI** (similar idea, smaller
  set); **React Aria** from Adobe (the most rigorous accessibility implementation available);
  a styled library like MUI or Mantine that includes both behavior and appearance.
- **Why this choice** — Accessible modal and popover behavior is deceptively hard — focus
  restoration, scroll locking, escape handling, screen-reader semantics — and getting it
  wrong is both an exclusion problem and a bug source. Radix supplies exactly that layer and
  nothing else, which composes cleanly with Tailwind.
- **Trade-off** — React Aria is more thorough and better documented on the accessibility
  details; it is also a larger API surface to learn. Radix is the smaller, faster path for
  the six primitives this app actually needs (dialog, popover, tooltip, toast, sheet, tabs).
  We are trading some rigor for velocity, on a component set small enough to audit manually.

### 1.9 Zod (runtime validation)

- **What it is** — A schema library for describing data shapes in TypeScript and validating
  values against them *at runtime*, inferring the static type from the schema so the two can
  never disagree.
- **Alternatives** — **Valibot** (smaller bundle, similar API); **Yup** (older, weaker type
  inference); **io-ts** (more rigorous, harder to read); **ArkType**; or trusting the API and
  validating nothing.
- **Why this choice** — TypeScript types vanish at runtime, so a malformed API response is a
  silent failure. Zod is the pragmatic guard at the boundary — form input and any response
  we do not fully control — and its inference means one declaration produces both the check
  and the type.
- **Trade-off** — Validating every response duplicates work the generated types already
  describe (§1.14) and adds bundle size. We therefore use Zod narrowly: at form boundaries
  and on untrusted input, not as a second copy of the report schema. The alternative of
  skipping runtime validation entirely is cheaper but converts backend contract drift into
  an unhelpful runtime crash.

### 1.10 Vitest, Testing Library, Playwright

- **What they are** — **Vitest** runs unit tests, sharing Vite's transform pipeline so tests
  and app build identically. **Testing Library** renders components and queries them the way
  a user perceives them (by role and text, not by CSS class), which makes tests survive
  refactors. **Playwright** drives a real browser end to end.
- **Alternatives** — **Jest** instead of Vitest (the long-time standard, slower here because
  it needs its own transform config); **Cypress** instead of Playwright (friendlier debugger,
  weaker multi-browser and parallelism story); **Enzyme** instead of Testing Library
  (deprecated, tested implementation details).
- **Why this choice** — Vitest is the natural pairing with Vite and is markedly faster.
  Playwright covers the three flows whose breakage would be fatal — first analysis to PDF,
  signup and upgrade, watch creation to alert — across browsers, headlessly, in CI.
- **Trade-off** — Jest has more ecosystem history and more agent-generated examples;
  Cypress is nicer to debug interactively. Both are acceptable. We favor CI speed and
  cross-browser coverage over interactive ergonomics, because a solo founder runs the suite
  far more often than they debug it.

### 1.11 Client-rendered SPA with server-rendered public pages  ⚠ Genuinely arguable

- **What it is** — The app ships as a JavaScript bundle that renders in the browser, but
  three categories of page — shared reports, `/compare/:a-vs-:b` comparison pages, and KB
  threads — are additionally rendered to HTML by the Rust backend so that crawlers and
  first-time visitors get content without executing JavaScript.
- **Alternatives** — **Next.js** with server components (the conventional answer, requiring a
  Node.js process alongside the Rust backend); **Remix / React Router v7 framework mode**;
  a **prerender step** in CI that snapshots pages to static HTML; **client-only rendering**
  with no SEO story at all.
- **Why this choice** — Introducing Next.js means a second language runtime, a second deploy
  artifact, a second set of security updates, and a Node process competing for RAM on a box
  whose entire memory budget is planned around a quantized model. The pages that need SSR are
  read-only renderings of a JSON document we already have server-side, which is a small
  templating job in Rust — not a reason to adopt a framework.
- **Trade-off** — This is a real cost. We give up React Server Components, streaming SSR,
  and the enormous body of Next.js documentation and agent familiarity, and we take on
  maintaining two render paths for those pages (Rust for the crawler, React for the app),
  with the risk that they drift. The alternative cost is an entire additional runtime on the
  critical box. If the SSR surface grows beyond those three page types, this decision should
  be revisited honestly — but the current surface is small and static, and the Rust side is
  perhaps 300 lines of templating.

### 1.12 Server-Sent Events instead of WebSockets

- **What it is** — SSE is a plain HTTP response with `Content-Type: text/event-stream` that
  stays open while the server pushes newline-delimited events. Browsers consume it with the
  built-in `EventSource` API, which reconnects automatically and replays from the last event
  id it received.
- **Alternatives** — **WebSockets** (full duplex, the general-purpose answer); **long
  polling** (repeatedly request until something changes); **HTTP chunked streaming** parsed
  by hand with `fetch` and a `ReadableStream`.
- **Why this choice** — The traffic is entirely one-directional: the server emits stage
  changes, source cards, token deltas, and completion. SSE is ordinary HTTP, so it passes
  through Caddy, Cloudflare, and corporate proxies without special configuration; it
  reconnects and resumes for free via `Last-Event-ID`, which is exactly what a phone locking
  mid-analysis needs; and it keeps the Rust backend to a single protocol.
- **Trade-off** — SSE cannot send data from client to server on the same connection (we do
  not need to), cannot set custom request headers via `EventSource` (we authenticate with
  cookies, so this is fine), and is limited to six concurrent connections per origin on
  HTTP/1.1 — a real constraint that disappears under HTTP/2, which Caddy serves by default.
  WebSockets would remove those limits and add connection-state management, heartbeats,
  reconnection logic, and proxy configuration we would then own. For a one-way stream that is
  a bad trade.

### 1.13 Discriminated unions with exhaustive matching

- **What it is** — A TypeScript pattern where every event carries a literal `type` field, so
  narrowing on that field tells the compiler exactly which other fields exist. Combined with
  a `never` check in the default branch, adding a new event type without handling it becomes
  a compile error.
- **Alternatives** — A loose `{ type: string; data: any }` shape; class hierarchies; runtime
  `switch` with no type narrowing.
- **Why this choice** — The SSE stream is the most protocol-like part of the frontend, and
  it will grow (queue position, partial-failure notices, retry hints). Exhaustive matching
  guarantees that a new server event cannot be silently dropped by the UI.
- **Trade-off** — Essentially none; this is a free property of TypeScript once the events
  are typed. It is listed because it is a deliberate design decision, not an accident.

### 1.14 `serde` + `schemars` → JSON Schema → TypeScript types and GBNF grammar

- **What it is** — `serde` is Rust's serialization framework; `schemars` generates a JSON
  Schema document from the same annotated Rust structs. That one schema is then compiled into
  (a) TypeScript interfaces via `json-schema-to-typescript`, and (b) a GBNF grammar that
  constrains the language model's output (§3.3). CI regenerates and fails on drift.
- **Alternatives** — Maintaining the types by hand in both languages; **OpenAPI** as the
  source of truth with generators on both sides (`utoipa` or `aide` in Rust); **ts-rs**
  (Rust → TypeScript directly, but produces no JSON Schema and so cannot drive the grammar);
  **protobuf** or **gRPC-web**; **specta**.
- **Why this choice** — This is described in ARCHITECTURE.md as the highest-leverage decision
  in the codebase, and the reason is the third consumer. Hand-maintained types drift. `ts-rs`
  would solve the TypeScript half but leaves the model's output constraints as a separate,
  hand-written artifact that can silently disagree with the API — which is precisely the
  failure that would let an unrenderable report reach a user. One schema means the model
  *cannot* emit a shape the UI cannot render.
- **Trade-off** — JSON Schema is a verbose intermediate format, the generated TypeScript is
  uglier than hand-written types, and there is a codegen step to keep working in CI. OpenAPI
  would additionally document the HTTP API for the Phase 7 public API, which is a genuine
  advantage — and the two can coexist later, since `utoipa` can consume the same structs.
  We accept generated-code ugliness for a guarantee that three representations of the
  product's core data structure cannot diverge.

### 1.15 No charting library

- **What it is** — A decision not to add **Recharts**, **Chart.js**, **visx**, or **D3**.
- **Why this choice** — Reports are prose, tables, and citations. There is no time series and
  no quantitative data worth plotting; a chart would be decoration implying rigor the data
  does not have, which cuts directly against the product's honesty posture.
- **Trade-off** — Charts make screenshots look impressive, and screenshots drive launch-day
  signups. We forgo that. If a genuinely quantitative feature arrives (pricing history across
  watched competitors is the plausible one), a small library can be added then — and by then
  the data will justify it.

---

## 2. Backend — Rust

### 2.1 Rust as the backend language

- **What it is** — A compiled systems language with memory safety enforced at compile time
  and no garbage collector, meaning no unpredictable pause times and a small, stable memory
  footprint.
- **Alternatives** — **Go** (the usual choice for network services: simpler, fast enough,
  much quicker to learn); **Node.js/TypeScript** (one language across the stack); **Python
  with FastAPI** (the default for anything AI-adjacent, and by far the largest ecosystem for
  model tooling).
- **Why this choice** — It is a stated constraint, and it also fits: the backend is
  I/O-heavy fan-out (ten-plus concurrent polite fetches per analysis, hundreds of scheduled
  change checks) sharing one machine with an inference engine that wants every spare core and
  gigabyte. No GC and a small idle footprint mean the application tier stays out of the
  model's way. Deployment is a single static binary.
- **Trade-off** — Rust costs development speed, particularly around async lifetimes, and
  the AI/ML ecosystem is thinner than Python's. Python would have made model experimentation
  faster; Go would have made the CRUD faster to write. The counter-argument specific to this
  product is that the model runs *out of process* in C++ regardless (§3.1), so the Python
  ecosystem advantage largely evaporates — what remains is HTTP orchestration, scraping, and
  scheduling under tight memory constraints, which is Rust's strength. The learning-curve
  cost is also the most agent-mitigable part of the project (see ROADMAP §4).

### 2.2 `tokio` (async runtime)

- **What it is** — Rust has no built-in async runtime; you choose one. Tokio is the scheduler
  and I/O reactor that drives async tasks, timers, and sockets. It is the de facto standard.
- **Alternatives** — **`async-std`** (largely dormant); **`smol`** (smaller, simpler);
  **`glommio`** (thread-per-core with io_uring, excellent for storage-bound work); or
  synchronous threads with no runtime at all.
- **Why this choice** — Effectively everything we depend on — axum, sqlx, reqwest, redis —
  is built on Tokio. Choosing anything else means fighting the ecosystem.
- **Trade-off** — Tokio is a large dependency with a work-stealing multithreaded scheduler
  that requires `Send` bounds on most futures, which adds friction. A thread-per-core runtime
  could be faster for some workloads. Neither matters when the bottleneck is a language model
  in another process; ecosystem alignment is worth far more here than scheduler nuance.

### 2.3 `axum` with `tower` / `tower-http`

- **What it is** — `axum` is an HTTP framework: it routes requests to handler functions and
  extracts typed values (path parameters, JSON bodies, cookies) from them. `tower` is a
  generic middleware abstraction, and `tower-http` supplies ready-made layers for tracing,
  timeouts, compression, CORS, and rate limiting.
- **Alternatives** — **Actix Web** (the perennial benchmark leader, mature, its own actor
  heritage); **Rocket** (the most ergonomic, historically slower to track async Rust);
  **Poem**; **warp** (axum's spiritual predecessor, less active); raw **hyper**.
- **Why this choice** — axum is maintained by the Tokio team, so it composes natively with
  `tokio`, `tracing`, and `hyper` with no impedance mismatch. Tower middleware means the
  cross-cutting concerns this app needs — request tracing, per-route timeouts, compression,
  concurrency limiting — are configuration rather than code. Its SSE support
  (`axum::response::Sse`) is first-class, which matters given §1.12.
- **Trade-off** — Actix Web benchmarks slightly faster and has been production-proven
  longer. That performance difference is invisible in a system whose latency is dominated by
  token generation. axum's extractor system can produce intimidating trait-bound errors when
  handler signatures are wrong — a real ergonomic cost we accept in exchange for ecosystem
  cohesion.

### 2.4 `sqlx` (database access)  ⚠ Genuinely arguable

- **What it is** — An async SQL toolkit. You write SQL; `sqlx` checks it *at compile time*
  against a live database, verifying that the query parses, the tables and columns exist, and
  the result types match the Rust structs you are decoding into.
- **Alternatives** — **Diesel** (the mature Rust ORM, with a type-safe query DSL instead of
  raw SQL; synchronous by default, with `diesel-async` available); **SeaORM** (a full async
  ORM built on top of sqlx); **tokio-postgres** (the raw driver, no checking, no
  abstraction).
- **Why this choice** — Two reasons. First, the queries this product needs are not
  ORM-shaped: job-queue claiming with `FOR UPDATE SKIP LOCKED`, full-text search ranking with
  `tsvector`, window functions over usage counters. In an ORM these become escape hatches
  anyway. Second, compile-time verification catches the class of error an ORM prevents by
  construction — misspelled columns, wrong types — without hiding the SQL.
- **Trade-off** — The compile-time checking has a real operational cost: it requires either a
  reachable database at build time or a checked-in `.sqlx` offline metadata directory that
  must be regenerated (`cargo sqlx prepare`) whenever a query changes, or CI breaks in a way
  that is confusing the first few times. Diesel would give a composable query DSL and
  stronger migration tooling; SeaORM would give relations and lazy loading. We choose visible
  SQL and accept the offline-metadata chore, because the hardest queries here are precisely
  the ones an ORM would obscure. This is listed as arguable because a reviewer who prefers
  Diesel is not wrong — they are optimizing for a different query mix.

### 2.5 PostgreSQL 16

- **What it is** — A relational database. Beyond standard SQL it provides `jsonb` (indexed
  JSON columns), native full-text search, `SKIP LOCKED` for queue semantics, advisory locks,
  and a large extension ecosystem.
- **Alternatives** — **SQLite** (in-process, zero administration, genuinely excellent for
  single-machine deployments, especially with **Litestream** for streaming backups);
  **MySQL/MariaDB**; **MongoDB**; managed offerings like **Supabase** or **Neon**.
- **Why this choice** — Postgres lets one component do four jobs that would otherwise be four
  components: relational storage, the job queue (§2.7), full-text search for the knowledge
  base (§2.16), and semi-structured storage of report payloads via `jsonb`. For a solo
  operator, collapsing four moving parts into one is the dominant consideration.
- **Trade-off** — SQLite deserves a serious hearing here: the architecture is explicitly
  single-machine, SQLite has no separate process to run, back up, or tune, and it would be
  faster for most of these queries. It loses on three specifics — `SKIP LOCKED` queue
  semantics with multiple workers, full-text search quality (FTS5 is good but Postgres
  ranking plus `pg_trgm` fuzzy fallback is better for a Q&A corpus), and the scale-out path
  in ROADMAP Phase 7, where worker and API processes move to separate hosts and need a shared
  database over the network. Postgres costs us a daemon to run and back up; it buys the
  ability to split hosts without a migration.

### 2.6 Redis

- **What it is** — An in-memory key-value store with atomic operations and expiring keys.
  Used here for rate-limit counters, anonymous quota tracking, and hot cache metadata.
- **Alternatives** — Doing it in Postgres (one fewer service); an in-process cache such as
  **`moka`** (fastest, but not shared across processes); **Valkey** (the open-source Redis
  fork after Redis's 2024 licence change); **Memcached**.
- **Why this choice** — Rate limiting requires cheap atomic increments with TTLs at higher
  frequency than the rest of the workload, and pushing that into Postgres means write
  amplification on the database that also serves the job queue and search. Redis is the
  standard tool and costs about 30MB of RAM at our volume.
- **Trade-off** — It is an additional daemon in a design that otherwise prides itself on
  having few. An honest simplification would be to start with in-process `moka` counters
  while the deployment is genuinely one process, and introduce Redis when the worker splits
  off. The reason to include it from the start is that the anonymous-quota and admission-control
  logic must be *correct* across api and worker roles the moment they separate, and retrofitting
  shared state is more error-prone than running one extra daemon. Note also the licensing
  context: Redis relicensed in 2024, which is why Valkey exists; either works, and the client
  API is identical.

### 2.7 Postgres-backed job queue using `FOR UPDATE SKIP LOCKED`

- **What it is** — A `jobs` table plus roughly 200 lines of SQL and Rust. Workers claim work
  with `SELECT ... FOR UPDATE SKIP LOCKED LIMIT n`, which atomically locks rows that no other
  worker has locked and skips over contended ones, giving correct competing-consumer
  semantics without a broker.
- **Alternatives** — **`apalis`** (a Rust background-job framework with Redis/Postgres/SQS
  backends); **RabbitMQ** or **NATS** (real message brokers); **Redis-based queues** in the
  Sidekiq tradition; **AWS SQS** (managed); **`pgmq`** (a Postgres extension implementing
  queue semantics).
- **Why this choice** — Two properties matter more than features. First, **transactionality**:
  enqueuing a job and writing the business row that justifies it happen in one transaction,
  so we can never bill a user and lose their analysis job, or create a watch whose first check
  never gets scheduled. No external broker can offer that. Second, **operational surface**: it
  is a table, so it is backed up, inspectable with `SELECT`, and survives restarts with no
  extra daemon.
- **Trade-off** — We hand-write retry, backoff, priority, visibility timeout, and dead-letter
  handling that `apalis` provides ready-made, and we own the bugs. Postgres-as-a-queue also
  produces table bloat under very high throughput and needs a vacuum strategy. Neither
  concern binds at this scale: this system will run on the order of thousands of jobs per day,
  where Postgres queues are well understood and comfortable. ARCHITECTURE.md sets the honest
  revisit trigger — more than about fifteen distinct job types, at which point framework
  features start earning their dependency.

### 2.8 `reqwest` with `rustls`

- **What it is** — `reqwest` is Rust's standard high-level HTTP client. `rustls` is a TLS
  implementation written in Rust, used instead of linking against the system's OpenSSL.
- **Alternatives** — Raw **hyper** (what reqwest is built on); **`ureq`** (synchronous,
  simpler); **`isahc`** (libcurl-backed); reqwest with **native-tls**/OpenSSL.
- **Why this choice** — reqwest handles redirects, timeouts, connection pooling, compression,
  and cookies without ceremony. Choosing rustls specifically means the release binary is
  fully static with no OpenSSL system dependency, which is what makes "build in CI, `scp` the
  binary, restart the service" a viable deploy strategy (§8.7).
- **Trade-off** — rustls is stricter than OpenSSL about certificate validity and older TLS
  configurations, so a small number of badly-configured public sites that OpenSSL would
  tolerate will fail to connect. Given that this product *fetches arbitrary public websites*,
  that is a real and recurring cost — it will show up as occasional "unreachable" sources. We
  accept it for static linking and memory safety in the component most exposed to hostile
  input, and the product already has an honest way to report an unreachable source.

### 2.9 `governor` (rate limiting)

- **What it is** — A rate-limiting library implementing the GCRA algorithm (a leaky-bucket
  variant), used here to enforce polite per-host request spacing when fetching.
- **Alternatives** — `tower::limit` (concurrency, not rate); a hand-rolled token bucket;
  `tokio::time::sleep` between requests (crude, serializes everything).
- **Why this choice** — Per-host politeness is a rate constraint (at most one request per
  second *per domain*) with an unbounded and dynamic set of keys. `governor`'s keyed rate
  limiters handle exactly that, lock-free.
- **Trade-off** — Minor: the API is more abstract than a hand-rolled bucket, and per-host
  state lives in memory, so limits are per-process rather than global. When the worker role
  splits to its own host, per-host politeness becomes approximate unless moved to Redis. That
  is an accepted, documented imprecision, and erring toward *more* delay is the safe direction.

### 2.10 `scraper`, Readability-style extraction, `htmd`

- **What they are** — `scraper` parses HTML into a queryable DOM using the same engine as
  Firefox (`html5ever`) and supports CSS selectors. A Readability-style pass (Mozilla's
  algorithm, available in Rust as `readability` or `dom_smoothie`) strips navigation,
  sidebars, and footers to isolate the main content. `htmd` converts the surviving HTML to
  Markdown.
- **Alternatives** — Regex over raw HTML (fragile and a classic mistake); sending raw HTML
  straight to the model; a hosted extraction API such as **Diffbot** or **Firecrawl**;
  **trafilatura** (the best-regarded extractor, but Python).
- **Why this choice** — Markdown is the right intermediate representation for three
  independent consumers: the model reads it (headings and tables survive, and it costs far
  fewer tokens than HTML), the change detector diffs it, and the verifier string-matches
  evidence quotes against it. Doing extraction locally keeps marginal cost at zero and avoids
  a vendor on the critical path.
- **Trade-off** — Local extraction is meaningfully worse than a specialist service on messy
  pages, and pricing tables — the single most important content type for this product — are
  exactly where naive extraction fails. This is why ARCHITECTURE.md §5.3 gates sources on an
  `extraction_quality` score and QUALITY_GUARDRAILS.md §4 notes that most quality failures
  trace to extraction rather than the model. A hosted extraction API remains the most
  defensible place to spend money later; trafilatura via a small Python sidecar is the other
  credible upgrade.

### 2.11 `texting_robots` (robots.txt)

- **What it is** — A parser for `robots.txt` that resolves whether a given user-agent may
  fetch a given path.
- **Alternatives** — Hand-parsing (the format is deceptively irregular); ignoring robots.txt
  entirely.
- **Why this choice** — Correct robots handling is a legal-posture and reputational
  requirement (QUALITY_GUARDRAILS.md §6), not an optimization, and the format has enough edge
  cases — wildcards, longest-match precedence, agent-group inheritance — that hand-parsing is
  a bug farm.
- **Trade-off** — None worth noting. The only decision of substance is *honoring* robots at
  all, which costs us some sources and buys us the ability to state publicly that we do.

### 2.12 `similar` (diffing) and SimHash (near-duplicate detection)

- **What they are** — `similar` computes line- and word-level diffs between two texts, used
  to show what changed on a watched page and to feed the importance-scoring model. **SimHash**
  is a locality-sensitive hash: similar documents produce similar hashes, so a small Hamming
  distance means "essentially the same page."
- **Alternatives** — For diffing: the `diff` crate, or shelling out to `git diff`. For
  near-duplicate detection: exact hashing (too sensitive — one changing timestamp reads as a
  change), full text comparison (too slow at scale), MinHash, or cosine similarity over
  embeddings.
- **Why this choice** — The two work as a pair and address the product's largest
  non-technical risk, notification noise (ROADMAP R4). SimHash is the cheap first gate that
  discards cosmetic churn without invoking the model at all; `similar` then produces a
  human-readable and model-readable diff only for changes that survive. Embedding-based
  similarity would require running an embedding model on every check — precisely the compute
  we are conserving.
- **Trade-off** — SimHash needs a tuned threshold, and tuning is empirical: too tight and
  cosmetic changes leak through, too loose and a genuine one-line price change is suppressed.
  That is exactly why ROADMAP Phase 5 specifies a recorded noise regression suite of
  material/cosmetic change pairs — the threshold is a parameter under test, not a constant.

### 2.13 Authentication: magic links, `argon2`, signed cookies

- **What it is** — Sign-in by emailing a single-use link rather than accepting a password.
  `argon2` (specifically Argon2id) is the password hashing function held in reserve for
  optional passwords. Sessions are opaque ids in signed, `HttpOnly` cookies.
- **Alternatives** — Passwords as the primary method (with bcrypt, scrypt, or PBKDF2 for
  hashing); **OAuth / social login** via Google and GitHub; **passkeys / WebAuthn**; a hosted
  identity provider such as **Auth0**, **Clerk**, or **Supabase Auth**; **JWTs** instead of
  session cookies.
- **Why this choice** — Magic links remove the two highest-friction moments in signup —
  choosing a password and confirming an email — and collapse them into one step, which is
  directly aligned with the product's frictionless mandate. They also eliminate credential
  stuffing, password reset flows, and the support load that comes with both. Argon2id is the
  Password Hashing Competition winner and the current recommended default, resistant to both
  GPU and side-channel attacks; it is specified now so that if passwords are ever added,
  nobody reaches for MD5 or an unsalted SHA. Opaque session cookies are chosen over JWTs
  because they can be revoked instantly by deleting a row, and there is no distributed
  verification problem to solve on one host.
- **Trade-off** — Magic links make login dependent on email deliverability, which turns a
  Postmark outage into an inability to sign in — the reason ARCHITECTURE.md §8 separates
  message streams so alert volume cannot damage magic-link reputation. They are also slower
  for a returning user on a new device (open inbox, click) than typing a saved password, and
  they interact poorly with corporate link-scanning proxies that "click" links in transit —
  a real failure mode mitigated by single-use tokens with a short TTL and a clear
  "link already used" recovery path. Social login would be faster still but hands identity to
  a third party and adds OAuth flows for a per-provider gain. Hosted auth would save a week
  and cost a monthly bill plus a hard dependency on the critical path.

### 2.14 `async-stripe`

- **What it is** — A Rust client for the Stripe API with typed request and response models.
- **Alternatives** — Calling Stripe's REST API directly with `reqwest` and hand-written
  types; **`stripe-rust`** (the older crate this forked from).
- **Why this choice** — Typed models for subscriptions, invoices, and webhook events prevent
  an entire category of field-name and enum-value mistakes in code that handles money.
- **Trade-off** — Stripe is not an official maintainer of any Rust SDK, so the crate lags
  the API and depends on community upkeep. That is a genuine supply risk for the billing
  path. The mitigation is architectural and already in the design: entitlements are derived
  from local tables and never from a live Stripe call (ARCHITECTURE.md §9), so the surface
  area touching the SDK is small — a handful of Checkout, Portal, and webhook calls — and
  could be replaced with raw `reqwest` in a day if the crate stalls.

### 2.15 Observability: `tracing`, OpenTelemetry, Prometheus, Grafana/Tempo

- **What they are** — `tracing` is Rust's structured, span-based instrumentation: rather than
  log lines, you record nested spans with typed fields, so one analysis produces a tree of
  timed operations. OpenTelemetry is the vendor-neutral protocol for shipping that data.
  Prometheus scrapes numeric metrics; Grafana visualizes them; Tempo stores traces.
- **Alternatives** — `log` + `env_logger` (plain lines, no structure); hosted APM such as
  **Datadog**, **Honeycomb**, **Sentry**, or **Axiom**; printing to stdout and grepping.
- **Why this choice** — The central operational question for this product is "where did the
  38 seconds go?" — resolution, fetching, extraction, queue wait, or generation. That is a
  span-tree question, and only tracing answers it without guesswork. Prometheus plus Grafana
  self-hosted keeps the cost at zero, matching the local-inference cost posture.
- **Trade-off** — Self-hosting the observability stack means running Prometheus, Grafana, and
  Tempo on the same box that runs the model, competing for the RAM the whole architecture is
  budgeting carefully. Datadog or Honeycomb would be better tools with zero operational
  burden and a bill that starts small and does not stay small. A pragmatic middle path —
  worth considering during Phase 0 — is `tracing` locally plus **Sentry** for errors only,
  deferring the full metrics stack until there is traffic to look at. The instrumentation
  code is identical either way, which is the point: `tracing` is the decision, and the
  backend behind it is swappable.

### 2.16 Postgres full-text search (`tsvector`, `pg_trgm`) for the knowledge base

- **What it is** — Postgres converts documents to a `tsvector` of normalized lexemes and
  ranks them against a parsed query; `pg_trgm` adds trigram similarity for typo tolerance.
- **Alternatives** — **Elasticsearch** or **OpenSearch** (the heavyweight standard);
  **Meilisearch** or **Typesense** (excellent modern search servers with typo tolerance out
  of the box); a **vector database** with embedding search (**Qdrant**, **pgvector**);
  Algolia (hosted).
- **Why this choice** — The corpus is on the order of hundreds of threads. At that size,
  Postgres FTS is not a compromise; it is the correct tool, and it requires no additional
  service, no sync pipeline, and no separate backup.
- **Trade-off** — Meilisearch would give better relevance and typo handling for maybe 100MB
  of RAM. Semantic (vector) search would match "my report has no prices" to "why does my
  report say not found in public sources," which lexical search will miss — a real quality
  gap in a support KB where users describe symptoms in their own words. Deliberately
  deferred: SUPPORT_SYSTEM.md §2.3 sets the revisit point at roughly 1,000 threads, and notes
  that even then the right move is likely a reranking layer over FTS rather than a
  replacement. Meanwhile zero-result searches are logged and reviewed weekly, which converts
  the weakness into a content backlog.

### 2.17 Supporting crates

| Crate | What it is | Alternative | Why this / trade-off |
|---|---|---|---|
| `serde` | The serialization framework — derives JSON encoding/decoding from struct definitions | `miniserde`, hand-written codecs | Universal in Rust; every other crate speaks it. No real alternative. Compile-time cost from heavy derive macros is the only downside. |
| `schemars` | Generates JSON Schema from `serde` types | `okapi`, hand-written schemas | The linchpin of §1.14 — it is what makes one schema drive three consumers. Its schema output occasionally needs manual attributes for exotic types. |
| `figment` | Layered configuration (TOML file overridden by environment variables) | `config`, `envy`, plain `std::env` | Layering matters because local, staging, and production differ in a handful of values; hand-rolled env parsing gets sloppy about defaults and types. Small learning curve. |
| `thiserror` | Derives error types with structured variants | `anyhow` (opaque errors), `snafu`, hand-written `impl Error` | Library code needs *matchable* errors so the API layer can map them to correct HTTP statuses; `anyhow` erases that distinction. `anyhow` remains appropriate in the binary and in tests, where nobody matches on the error. |
| `tower-cookies` | Cookie extraction and signing middleware for axum | Manual header parsing, `axum-extra`'s cookie support | Signed cookies without hand-rolling HMAC. Minor: cookie signing keys must be managed and rotated deliberately. |
| `wiremock` | An HTTP mock server for tests | `mockito`, recorded fixtures, hitting real sites | Lets the entire fetch and extraction pipeline be tested deterministically against pinned HTML — essential when the input is the live web. Slower than pure unit tests. |
| `sqlx::test` | Per-test transactional databases, rolled back automatically | Shared test DB with manual cleanup | Isolated, parallel-safe database tests with no cleanup code. Requires a running Postgres in CI. |
| `minijinja` | Jinja2-style templating, used for email bodies and SSR pages | `askama` (compile-time checked), `tera`, `handlebars` | Runtime templates mean email copy can change without a recompile. `askama` would catch template errors at compile time — the better choice if templates ever become numerous. |
| `MJML` | An email markup language that compiles to the table-based HTML mail clients require | Hand-written email HTML, **Maizzle**, plain text only | HTML email is a genuinely hostile target (Outlook's rendering engine is Word's). MJML removes that work. It is a Node build-time tool, so it runs in CI and produces templates the Rust side fills in — deliberately not a runtime dependency. |
| `tokio::sync::Semaphore` | A counting permit system; only *N* tasks may hold a permit at once | Channel-based worker pool, unbounded spawning | This is how inference concurrency is capped to the number of llama.cpp slots (§3.6). The single most important line of resource control in the backend. |
| `tokio::sync::broadcast` | A multi-producer, multi-consumer channel where every receiver sees every message | `watch` channel (latest value only), per-client queues | Correct primitive for SSE fan-out: one analysis, potentially several connected tabs. Bounded capacity means a slow client can lag; hence the replay ring buffer. |
| `FuturesUnordered` | Runs many futures concurrently and yields results as they complete | `join_all` (waits for all), spawning a task per item | Lets per-source extractions complete in whatever order they finish, which is what drives sources appearing live in the UI. |
| `cargo audit` / `cargo deny` | Scan dependencies for known vulnerabilities; enforce licence and duplicate-dependency policy | Dependabot alone, manual review | Two CI checks that catch supply-chain problems and accidental GPL ingestion. Occasional false-positive noise from unmaintained transitive crates. |

---

## 3. Local LLM

### 3.1 llama.cpp and the GGUF format

- **What it is** — llama.cpp is a C++ inference engine for transformer models, built around
  the `ggml` tensor library, designed to run quantized models efficiently on ordinary CPUs
  and consumer GPUs. **GGUF** is its model file format, packaging weights, tokenizer, and
  metadata in one file.
- **Alternatives** — **vLLM** (the standard for high-throughput GPU serving, built around
  PagedAttention); **Ollama** (a friendly wrapper around llama.cpp with model management);
  **Hugging Face TGI**; **TensorRT-LLM** (NVIDIA's, fastest on NVIDIA hardware);
  **MLX** (Apple silicon only); **ONNX Runtime**.
- **Why this choice** — It is a stated constraint, and it is also the only option in this
  list that runs *well on CPU*, which is what makes the €60/month launch rung possible.
  vLLM and TensorRT-LLM assume a GPU and would force the GPU purchase into Phase 0. llama.cpp
  also has by far the best quantization support, which is what lets a 14B model fit in 20GB
  of VRAM at acceptable quality.
- **Trade-off** — On a GPU with many concurrent users, vLLM's continuous batching and paged
  KV cache deliver substantially higher throughput than llama.cpp. We are trading peak GPU
  throughput for the ability to run acceptably on CPU and to move up the hardware ladder on a
  metric rather than upfront. If this product ever reaches the traffic where vLLM's throughput
  advantage dominates, that is a very good problem and a well-understood migration — the
  `landscape-llm` crate isolates the client behind one trait for exactly that reason.
  Ollama deserves a specific note: it is easier to operate but abstracts away the slot and
  parallelism controls (`--parallel`, `--cont-batching`, slot reservation) that this design
  depends on for fairness between paid and free traffic. We take the harder-to-operate tool
  because we need those knobs.

### 3.2 `llama-server` as a sidecar process, not in-process Rust bindings

- **What it is** — llama.cpp ships `llama-server`, an HTTP server exposing the model with an
  OpenAI-compatible API, slot management, and continuous batching. We run it as a separate
  systemd-supervised process and call it over localhost.
- **Alternatives** — In-process FFI bindings: **`llama-cpp-2`** (the actively maintained
  binding, from utilityai) or **`llama_cpp_rs`**, linking llama.cpp directly into the Rust
  binary; or a Python sidecar wrapping `llama-cpp-python`.
- **Why this choice** — Four concrete reasons, in order of weight. **Fault isolation**: an
  out-of-memory kill or CUDA fault in the inference engine restarts a supervised process
  instead of terminating the web server and every open SSE connection. **Scheduling for
  free**: continuous batching, slot allocation, and prefix-cache reuse are maintained
  upstream; reimplementing them over raw bindings is where local-LLM projects lose months.
  **Upgrade independence**: swapping models or llama.cpp versions is a process restart, not a
  rebuild and redeploy. **Tracking cost**: llama.cpp changes fast, and following a binary
  release is cheaper than following an FFI crate that must chase upstream API churn.
- **Trade-off** — We pay a localhost HTTP hop and JSON serialization on every call —
  sub-millisecond, against generations measured in seconds — and we give up direct control
  over the KV cache and any possibility of zero-copy token streaming. We also now have two
  processes to supervise and health-check instead of one. That is the correct trade when the
  alternative is owning a scheduler; and ARCHITECTURE.md keeps the escape hatch open by
  isolating everything behind `LlmClient`.

### 3.3 GBNF grammars and constrained decoding

- **What it is** — At each generation step a language model produces a probability
  distribution over its whole vocabulary. Constrained decoding masks out every token that
  would violate a supplied grammar before sampling, so invalid output is not merely unlikely
  but *impossible*. GBNF is llama.cpp's grammar notation (a BNF variant); llama.cpp can also
  derive a grammar from a JSON Schema directly.
- **Alternatives** — Asking for JSON in the prompt and parsing hopefully; parse-and-retry
  loops; "JSON mode" as offered by hosted APIs; external constraint libraries such as
  **Outlines**, **XGrammar**, or **Guidance** (mostly Python, and largely unnecessary since
  llama.cpp has this built in).
- **Why this choice** — This is the mechanism that makes the whole small-model bet work, and
  it does three distinct jobs. It removes parse failures, so no local compute is burned on
  retry loops — a much bigger deal when tokens cost seconds of your own hardware rather than
  fractions of a cent. It makes **citation structurally mandatory**: a `Claim` object cannot
  be emitted without a `source_label` and an `evidence_quote`, because the grammar has no
  path to close the object without them. And it makes **refusal reliable**: an enum
  containing `not_found_in_public_sources` gives the model a well-formed way to decline, which
  is the single most effective anti-hallucination measure available — models invent most when
  the output format offers no legal way to say nothing.
- **Trade-off** — Grammar-constrained generation is somewhat slower per token (the mask must
  be computed each step), grammar compilation is expensive enough that it must be cached at
  startup rather than done per request, and an over-tight grammar can force the model into
  awkward phrasing or, worse, into filling a required field with something plausible rather
  than leaving the object out. That last failure mode is real and is why the grammar allows
  explicit "not found" paths at every level rather than only at the top. Prompt-and-hope
  would be simpler and would fail unpredictably; there is no version of this product where
  that is acceptable.

### 3.4 Model families: Qwen3, and the alternatives

- **What they are** — Open-weight model families distributed in GGUF form. **Qwen3**
  (Alibaba) spans 0.6B to 235B including a 30B mixture-of-experts variant, Apache-2.0
  licensed. **Llama 3.x** (Meta) is the most widely deployed. **Gemma 3** (Google) is strong
  at small sizes with good multilingual coverage. **Mistral Small 3.x** is a well-regarded
  ~24B Apache-2.0 model.
- **Why this choice** — Three criteria, applied in this order. **Licence first**: Apache-2.0
  (Qwen3, Mistral Small) imposes no use restrictions on a commercial SaaS, whereas Llama's
  community licence and Gemma's use policy carry conditions that need legal reading before
  they can be built on — so the licence review precedes the benchmark, not the reverse.
  **Long-context faithfulness over benchmark scores**: the job is "read eight pages and do not
  invent," which correlates poorly with MMLU and is only measurable on our own golden set.
  **Size for the rung**: 4B for high-volume extraction, 8B on CPU or 14B on GPU for synthesis.
- **Trade-off** — Qwen3 is the current recommendation, not a permanent one, and the document
  says so: model choice is a configuration value and the roadmap schedules a quarterly
  re-bake-off because this landscape turns over in months. The specific hedge worth flagging
  is the **mixture-of-experts** option (Qwen3-30B-A3B): MoE models hold all parameters in
  memory but activate only a fraction per token — roughly 3B of 30B here — giving near-14B
  quality at near-4B generation speed, provided you have RAM or VRAM for the full weights.
  On a 20GB GPU that is a very favorable trade and is the most likely Phase 7 upgrade.

### 3.5 Quantization: Q4_K_M and the rest

- **What it is** — Quantization stores weights at reduced precision. A 14B model at 16-bit
  floating point needs ~28GB; at roughly 4.8 bits per weight it needs ~8.5GB, which is the
  difference between "needs a data-center GPU" and "runs on a €180/month box."
  llama.cpp's **K-quants** (`Q4_K_M`, `Q5_K_M`) allocate more precision to the layers that
  matter most rather than quantizing uniformly. **I-quants** (`IQ4_XS`) use an importance
  matrix computed from calibration data to squeeze further at the same quality.
- **Alternatives** — **GPTQ** and **AWQ** (GPU-oriented 4-bit schemes used with vLLM);
  **bitsandbytes** NF4; running unquantized at F16.
- **Why Q4_K_M as default** — It sits at the knee of the quality-versus-size curve: the
  measured quality loss against F16 is small, while the size reduction is roughly 3.3×.
  It is also the most widely tested configuration in the llama.cpp ecosystem, which means
  fewer surprises.
- **Why not lower** — `Q3` and below are rejected outright. Quantization damage does not
  appear as obviously broken text; it appears as *degraded instruction-following and weaker
  factual precision while fluency is preserved* — confident, well-formed, subtly wrong output.
  That is the exact failure mode this product cannot tolerate, so the saving is refused.
- **Trade-off** — `Q5_K_M` would be slightly better and about 20% larger; the design says
  measure rather than assume, and ship the smallest quantization that passes the quality
  gates. GPTQ/AWQ would be marginally faster on GPU but are not the native path for llama.cpp
  and would not run on the CPU rung at all. **KV cache quantization** is called out
  separately in ARCHITECTURE.md §4.3 for a reason: quantizing the attention cache to `q8_0`
  roughly halves its memory and is usually harmless, but the damage it does surfaces
  specifically as degraded long-context faithfulness — which is this product's core
  competence. Hence it stays at F16 until the golden set says otherwise.

### 3.6 Slots, continuous batching, and `--parallel`

- **What it is** — A llama-server "slot" is an independent conversation context. `--parallel N`
  allocates N slots; **continuous batching** lets the server process tokens for several slots
  in the same forward pass, so throughput scales far better than running requests one at a
  time. The trade is memory: each slot needs its own KV cache, so context length per slot
  falls as N rises.
- **Alternatives** — Serial processing (simple, wastes hardware); running multiple
  llama-server processes (multiplies model memory); vLLM's paged KV cache (more efficient,
  GPU-only).
- **Why this choice** — Per-source extraction is naturally parallel — eight to fourteen
  independent small calls per analysis — and continuous batching is what turns that from
  eight sequential waits into something close to one. It is also the mechanism behind the
  fairness guarantee: reserving one slot for interactive traffic is what stops scheduled
  watch checks from starving a live user.
- **Trade-off** — More slots means less context per slot and more memory pressure; too many
  slots on a CPU rung causes thrashing rather than throughput. Four is the starting point in
  the design specifically because it is conservative, and Phase 0 measures throughput at
  1/2/4/8 rather than guessing.

### 3.7 Prefix caching, flash attention, speculative decoding

- **Prefix / KV caching** — Transformer inference caches per-token attention state. If two
  requests share an identical prefix, the cached state can be reused instead of recomputed.
  This is why ARCHITECTURE.md insists the system prompt be *byte-identical* across calls of a
  role with variable content strictly at the end — a formatting discipline that buys whole
  seconds per request on CPU. The alternative (prompts assembled in arbitrary order) costs
  that silently, which is why it is written down as a rule rather than left to taste.
- **Flash attention** (`-fa`) — A reordering of the attention computation that avoids
  materializing the full attention matrix, reducing memory traffic. Mathematically equivalent
  output, meaningfully faster and lighter on GPU. Essentially free; the only reason it is a
  flag rather than a default is hardware compatibility.
- **Speculative decoding** (`-md`) — A small fast "draft" model proposes several tokens; the
  large model verifies them in a single forward pass, accepting the ones it agrees with. Under
  the standard scheme the output distribution is preserved, so quality is unaffected while
  throughput improves by roughly 1.3–2× on predictable text. The costs are real: a second
  model resident in memory, and a benefit that varies with how predictable the output is —
  which is why it is listed as a Phase 7 lever to be measured, not a Phase 0 assumption.

---

## 4. Source discovery and fetching

### 4.1 SearXNG, with Brave Search API as fallback

- **What it is** — SearXNG is a self-hosted metasearch engine: it queries public search
  engines and returns aggregated results through its own API, with no account and no
  per-query cost.
- **Alternatives** — **Brave Search API** (independent index, paid per query);
  **Google Custom Search JSON API** (limited free quota, then paid, with restrictive terms);
  **Bing Web Search**; **SerpAPI** or **Serper** (scraping-as-a-service);
  **Exa** (semantic search built for AI agents).
- **Why this choice** — Zero marginal cost per query is structurally consistent with the rest
  of the architecture: the entire economic argument for local inference is that the marginal
  analysis is free, and a per-query search bill would reintroduce exactly the variable cost we
  removed. Self-hosting also avoids a vendor with terms that could change.
- **Trade-off** — SearXNG is genuinely fragile. It depends on upstream engines that rate-limit
  and block, so results degrade unpredictably and it needs occasional maintenance. This is the
  weakest link in the fetch pipeline and the document is explicit that a paid Brave fallback
  exists for reliability. Worth stating plainly for the reviewer: **search APIs are not LLM
  APIs**, so paying for Brave does not violate the local-inference constraint — if SearXNG
  proves unreliable in practice, moving to Brave as primary is a legitimate and inexpensive
  correction, not a compromise of the product's principles.

### 4.2 Conditional requests (`ETag` / `If-Modified-Since`)

- **What it is** — Standard HTTP caching. The server returns a version token with a
  response; the next request includes it, and an unchanged resource yields a `304 Not
  Modified` with no body.
- **Alternatives** — Refetching everything every time; comparing full-body hashes after
  download (works, but wastes bandwidth and the origin's resources).
- **Why this choice** — Change detection is the highest-frequency operation in the system:
  hundreds of watched pages checked repeatedly, the overwhelming majority unchanged. A `304`
  costs almost nothing for us *and for the site being watched*, which is a politeness
  obligation as much as an optimization. It short-circuits the entire pipeline — no
  extraction, no diff, no model call.
- **Trade-off** — Many sites serve incorrect or absent validators, so the content-hash
  comparison remains as a second gate. Belt and braces, by necessity.

### 4.3 No JavaScript rendering in v1

- **What it is** — We fetch and parse HTML as served. We do not run a browser engine, so
  content injected by client-side JavaScript is invisible to us.
- **Alternatives** — **Playwright** or **Puppeteer** driving headless Chromium; a hosted
  rendering service (**Browserless**, **ScrapingBee**); **chromiumoxide** to drive Chrome
  from Rust.
- **Why this choice** — A headless Chrome instance consumes hundreds of megabytes per page
  and seconds of CPU. On a machine whose memory budget is planned around a quantized model,
  that is the single most expensive thing we could add, and it would compete directly with
  inference for the resource that determines user-visible latency.
- **Trade-off** — This is a real capability gap, and it will be visible: some pricing pages
  are client-rendered and will simply appear empty, producing a "not found in public sources"
  where a browser would have found the answer. The product has an honest way to report that,
  which limits the damage to a missing section rather than a wrong one. ARCHITECTURE.md marks
  headless rendering as a Phase 6+ decision with real cost, and the sensible form is a
  narrow, opt-in, off-peak path for a small set of known-important pages rather than a
  general-purpose renderer.

---

## 5. PDF generation — Typst

- **What it is** — Typst is a modern typesetting system written in Rust, usable as a library.
  Templates are plain text files with a light markup and scripting language; data is injected
  as JSON and rendered to PDF in-process.
- **Alternatives** — **Headless Chrome** printing HTML to PDF (the most common approach, and
  the most faithful to a web design); **wkhtmltopdf** (long-deprecated, unmaintained since
  2023); **LaTeX** via a system install (superb typography, enormous dependency, slow);
  **`printpdf`** or **`genpdf`** (Rust, but low-level — you place text at coordinates);
  client-side **jsPDF** or **react-pdf**.
- **Why this choice** — Four properties that matter here specifically. It is pure Rust and
  in-process, so there is no browser to install, supervise, or memory-cap on the inference
  box. It renders in tens of milliseconds rather than seconds, which is what makes
  pre-warming the executive PDF on completion cheap enough to do for every analysis.
  Its output is deterministic — the same data always produces byte-identical output, which
  makes caching trivial and regressions visible. And its typography is genuinely good by
  default, which matters for a document users forward to their manager.
- **Trade-off** — Typst is a new language to learn for anyone who knows HTML/CSS, its
  ecosystem is small, and it cannot reuse the React components that already render the report
  on screen — so report layout is maintained twice, once in Tailwind and once in `.typ`
  templates. That duplication is the real cost. Headless Chrome would eliminate it by
  rendering the actual page, at the price of several hundred megabytes of resident memory and
  a second supervised process on the most memory-constrained machine in the system. Given the
  PDF is a deliberately different artifact from the web report — one page, executive summary,
  fixed layout — the duplication is smaller in practice than it first appears.

---

## 6. Email — Postmark or Resend

- **What it is** — Transactional email providers with HTTP APIs, per-message delivery
  tracking, and bounce/complaint webhooks. Postmark is the long-standing deliverability
  specialist; Resend is newer with better developer ergonomics.
- **Alternatives** — **Amazon SES** (much cheaper at volume, worse tooling, and you own
  reputation warm-up); **SendGrid** or **Mailgun** (large, general-purpose);
  **self-hosted Postfix** (free, and a reputation disaster waiting to happen).
- **Why this choice** — Magic links mean **deliverability is authentication**: an email that
  lands in spam is a user who cannot sign in. That converts email from a commodity into a
  critical dependency worth paying a specialist for. The separate *message streams* matter as
  much as the provider — alert volume and its inevitable complaint rate must not be able to
  damage the sending reputation that magic links depend on.
- **Trade-off** — SES is roughly an order of magnitude cheaper and is the right answer at
  high volume; it is the wrong answer at low volume, where establishing sender reputation
  from scratch is the hard part and the tooling gap costs founder time. Self-hosting is
  rejected outright: running a mail server that reaches Gmail's inbox reliably is a
  full-time specialty. Related standards worth naming since they appear in ARCHITECTURE.md:
  **SPF**, **DKIM**, and **DMARC** are DNS records that let receivers verify a message
  genuinely came from your domain — non-optional since Google and Yahoo tightened bulk-sender
  rules in 2024 — and **`List-Unsubscribe-Post`** (RFC 8058) is the header that provides
  one-click unsubscribe in the mail client, also now effectively required and, independently,
  the right thing to offer.

---

## 7. Payments — Stripe  ⚠ Genuinely arguable

- **What it is** — A payments platform. **Stripe Checkout** is a hosted purchase page;
  **Stripe Billing Portal** is a hosted page where customers change plans, update cards,
  download invoices, and cancel. Both are Stripe-hosted, so card data never touches our
  servers.
- **Alternatives** — **Paddle** or **Lemon Squeezy** (merchants of record); **Braintree**;
  **Chargebee** layered on top of Stripe.
- **Why this choice** — Stripe has the best API and documentation in the category, and
  Checkout plus Billing Portal removes essentially all of the billing UI that would otherwise
  need building, testing, and supporting. PCI scope stays minimal because card details never
  reach us.
- **Trade-off** — This is the most genuinely debatable choice in the document, and the reason
  is **tax**. A merchant of record such as Paddle or Lemon Squeezy becomes the legal seller
  and takes on VAT, GST, and US sales-tax registration, calculation, and remittance —
  a substantial and growing compliance burden for a solo founder selling software
  internationally. Their fee is higher (roughly 5% + fixed versus Stripe's ~2.9% + 30¢), and
  their APIs are less pleasant. Stripe offers **Stripe Tax** as a paid add-on that calculates
  correctly but still leaves *registration and filing* with us. The recommendation stands
  because Stripe's developer experience and the maturity of `async-stripe` matter for build
  speed, and because at early revenue the tax thresholds in most jurisdictions are not yet
  crossed — but **a reviewer should consciously accept the tax-compliance obligation, or
  choose a merchant of record now**, because migrating billing providers after customers
  exist is genuinely painful.
- **Related decision — webhooks are not the source of truth.** Stripe delivers events by
  webhook, and webhooks get lost, duplicated, and delivered out of order. The design
  therefore verifies signatures, deduplicates on `event.id`, derives entitlements from local
  tables, and reconciles hourly. The alternative — trusting webhook delivery — produces the
  worst possible bug class: a customer who paid and did not get access.

---

## 8. Hosting and operations

### 8.1 Caddy (reverse proxy and TLS)

- **What it is** — A web server that obtains and renews TLS certificates from Let's Encrypt
  automatically, serves static files, and reverse-proxies to the Rust backend. Configuration
  is a handful of lines.
- **Alternatives** — **nginx** (the standard, more configuration, TLS via a separate certbot
  cron); **Traefik** (container-native, service discovery); **HAProxy** (best-in-class load
  balancing); terminating TLS in Rust directly.
- **Why this choice** — Automatic HTTPS eliminates an entire category of 3am outage
  (expired certificate) with no cron job to maintain, and HTTP/2 is on by default — which
  matters because it removes the six-connections-per-origin limit that constrains SSE over
  HTTP/1.1 (§1.12).
- **Trade-off** — nginx has vastly more documentation and operational history, and better
  performance at extreme scale. Neither is decisive at this size. Caddy's Go runtime adds
  perhaps 30MB of RAM — noted only because RAM is the scarce resource on this box.

### 8.2 systemd, not containers  ⚠ Genuinely arguable

- **What it is** — Services are managed as systemd units on the host: automatic restart on
  crash, memory limits, CPU affinity, log capture, dependency ordering.
- **Alternatives** — **Docker Compose** (the common default); **Kubernetes** (k3s for a
  single node); **Nomad**; **Podman** with systemd integration.
- **Why this choice** — For `llama-server` specifically, the controls that matter are
  `Restart=always`, `MemoryMax=`, `CPUAffinity=`, and clean OOM isolation — all first-class
  in systemd. GPU passthrough into containers adds real friction, and the Phase 7 GPU
  migration is a planned event we would rather not complicate. There is no orchestration
  problem to solve on one host, and Kubernetes for a single machine is pure overhead.
- **Trade-off** — Containers give reproducible environments and identical local/production
  images, which systemd does not; dependency drift on the host is a genuine risk that must be
  managed by discipline instead. The design hedges by running Postgres, Redis, and SearXNG
  under Docker Compose (where reproducibility is worth more than resource control) while
  keeping the Rust binary and llama-server on systemd (where resource control is worth more
  than reproducibility). That split is deliberate, and a reviewer who wants everything
  containerized should weigh it against GPU passthrough complexity in Phase 7.

### 8.3 Hetzner dedicated hardware

- **What it is** — A German hosting provider offering dedicated servers — real, unshared
  CPUs — at roughly a quarter of hyperscaler pricing. The AX52 class is a Ryzen 7 with 64GB
  of DDR5; the GEX44 adds an RTX 4000 SFF Ada with 20GB of VRAM.
- **Alternatives** — **AWS/GCP/Azure** (where equivalent GPU capacity costs many times more);
  **Fly.io**, **Railway**, or **Render** (excellent developer experience, priced for
  small workloads, unattractive for sustained inference); **DigitalOcean** or **Linode**
  (shared vCPU at this tier); **Vast.ai** or **RunPod** (cheapest GPU rental, but preemptible
  and unsuitable for a persistent service).
- **Why this choice** — Dedicated CPU is not a preference, it is a requirement: on shared
  vCPU, inference latency varies with neighbouring tenants, which makes a latency SLO
  meaningless and a benchmark unrepeatable. Hetzner is where the price-performance is, and
  the GPU upgrade path (GEX44 at roughly €180/month) is a known, bookable next rung rather
  than a hyperscaler bill.
- **Trade-off** — No managed services, no meaningful SLA, single-region (Germany/Finland),
  and setup is manual. Latency to US users is 90–150ms — acceptable when the product's own
  response time is measured in tens of seconds, and mitigated for static assets by
  Cloudflare. We are trading convenience and geographic reach for the price-performance that
  makes the whole fixed-cost inference model work. This is the one place ARCHITECTURE.md
  explicitly warns against chasing a free tier.

### 8.4 Cloudflare free tier, Backblaze B2, GitHub Actions

| Choice | What it is | Alternatives | Why / trade-off |
|---|---|---|---|
| **Cloudflare** (free) | CDN and DDoS protection in front of the origin | Bunny.net, Fastly, no CDN at all | Absorbs launch-day traffic spikes on static assets and hides the origin IP, at zero cost. Trade-off: an additional dependency in the request path, and SSE requires attention to buffering settings. |
| **Backblaze B2** | Object storage for backups and rendered PDFs, ~$6/TB/month | AWS S3 (~4× the storage price plus egress), **Cloudflare R2** (zero egress fees, S3-compatible) | Cheapest credible option with an S3-compatible API. R2 is arguably the better pick given we already use Cloudflare and it has no egress charges — a fair reviewer substitution. |
| **GitHub Actions** | CI: build, test, lint, cross-compile the Rust binary, run the nightly golden-set eval | GitLab CI, self-hosted runners, no CI | Free for public repositories and generously provisioned for private ones; the code already lives on GitHub. Trade-off: Rust builds are slow on hosted runners and need caching discipline; the nightly evaluation job may eventually need a self-hosted runner with access to the model. |

### 8.5 Deployment: build in CI, copy the binary, restart the service

- **What it is** — GitHub Actions produces a statically linked Linux binary; a script copies
  it to the server and restarts the systemd unit. Roughly twenty lines.
- **Alternatives** — Container images pushed to a registry and pulled by the host;
  blue-green or rolling deploys behind a load balancer; a platform-as-a-service that deploys
  on `git push`; Ansible or similar configuration management.
- **Why this choice** — Rust's static linking makes it this simple, and simple is the point.
  There is nothing to orchestrate on one host.
- **Trade-off** — A restart causes a few seconds of downtime and drops in-flight SSE
  connections — acceptable when the frontend already reconnects and replays via
  `Last-Event-ID`, which is a capability built for phone-lock and network-drop anyway. There
  is no automated rollback; rollback is copying the previous binary back, which is fast but
  manual. Blue-green would remove the downtime and add a load balancer, a health-check
  protocol, and two copies of everything. Not yet worth it; the trigger is paying customers
  noticing deploys.

---

## 9. Security mechanisms

| Mechanism | What it is | Alternative | Why / trade-off |
|---|---|---|---|
| **Argon2id** | Memory-hard password hashing; winner of the Password Hashing Competition | bcrypt (fine, less memory-hard), scrypt, PBKDF2 (weakest of the four), plain SHA (unacceptable) | Current recommended default, resistant to GPU cracking and side channels. Only relevant if optional passwords are ever added — magic links are primary. Parameters must be tuned to the host or verification becomes a denial-of-service vector against yourself. |
| **Single-use magic links, 15-minute TTL, constant-time comparison** | Login tokens that expire quickly, work once, and are compared without leaking timing information | Long-lived links; naive `==` comparison | Short TTL limits the window if an inbox is compromised; single use defeats replay; constant-time comparison defeats timing attacks. Trade-off: corporate email scanners that pre-fetch links will consume the token, so a clear "link already used, request another" path is mandatory. |
| **`HttpOnly`, `Secure`, `SameSite=Lax` cookies** | Session cookies unreadable by JavaScript, sent only over HTTPS, and not sent on cross-site requests | `localStorage` tokens; `SameSite=None` | `HttpOnly` means an XSS bug cannot exfiltrate the session — the reason cookies beat `localStorage` for auth. `Lax` blocks CSRF on unsafe methods while still allowing normal inbound navigation. |
| **CSRF tokens** | A per-session secret required on state-changing form submissions | Relying on `SameSite` alone | Defence in depth; `SameSite` support and browser behaviour vary enough that it should not be the only control. Cost: a token to thread through forms. |
| **Content Security Policy** | An HTTP header restricting which scripts, styles, and connections the page may load | No CSP | Sharply limits the damage of an injection bug — relevant because the KB renders user-submitted Markdown. Cost: it must be maintained as the frontend changes, and a wrong CSP breaks the app loudly. |
| **SSRF protection** | Blocking user-supplied URLs that resolve to private, link-local, or cloud-metadata addresses, re-checking after DNS resolution to defeat rebinding | Trusting user input; a naive string blocklist | **The highest-severity risk in this codebase**, because the core feature is "fetch a URL a stranger typed." Without it, a user can make the server read its own metadata endpoint or internal services. Resolve-then-verify is required; checking the hostname alone is defeated by DNS rebinding. Cost: a small amount of care on every outbound fetch, and occasional false positives on legitimately unusual hosts. |
| **Parameterized queries** (via `sqlx`) | Query parameters sent separately from SQL text, so input cannot alter query structure | String concatenation | Eliminates SQL injection structurally. Free — it is simply how `sqlx` works. |
| **Markdown sanitization** | Rendering user KB posts to a safe HTML subset with no raw HTML or iframes, and `rel="nofollow ugc"` on links | Allowing raw HTML; sanitizing on input instead of output | Public user-generated content is an XSS and SEO-spam vector. Sanitizing at render time is safer than at write time, because it can be tightened retroactively over content already stored. |

---

## 10. Decisions deliberately deferred

Things a reviewer might expect to see, and why they are absent:

| Not chosen | Why not, and when to revisit |
|---|---|
| **Vector database / embeddings** (pgvector, Qdrant) | The KB is hundreds of threads, and source selection is driven by targeted probes and search rather than semantic retrieval. Revisit at ~1,000 KB threads, or if source ranking becomes the quality bottleneck. |
| **Fine-tuning / LoRA** | Prompting, grammar constraints, and retrieval quality are nowhere near exhausted, and fine-tuning adds a training pipeline plus a class of silent regressions. Phase 7+ at the earliest. |
| **GraphQL** | One client, a dozen endpoints, and a streaming path that is not request/response. REST plus SSE is the right size. |
| **Microservices** | One binary with a `--role` flag already provides the only split that matters (API versus worker). |
| **Kubernetes** | No orchestration problem exists on one host. |
| **A hosted LLM API fallback** | Explicitly excluded by the product constraint, and accepting it would quietly undo the cost model and the privacy claim. Worth stating that this is a *product* decision, not merely a technical one. |
| **Multi-region deployment** | Response times are dominated by inference, not network latency. A CDN in front of static assets is sufficient. |

---

## 11. Where a reasonable reviewer could disagree

The choices most worth challenging, with the conditions that should force a re-think:

| Decision | The counter-case | Revisit trigger |
|---|---|---|
| **SPA + Rust SSR rather than Next.js** (§1.11) | Next.js is the conventional answer and would remove a duplicated render path | The SSR surface grows beyond shared reports, comparison pages, and KB threads |
| **Stripe rather than a merchant of record** (§7) | Paddle/Lemon Squeezy absorb international VAT and sales-tax compliance entirely | Before crossing VAT/sales-tax registration thresholds — migration after customers exist is painful |
| **Postgres rather than SQLite** (§2.5) | SQLite plus Litestream is simpler and faster for a genuinely single-machine design | If the Phase 7 host split never happens, Postgres was over-provisioned |
| **Self-hosted observability rather than Sentry/Honeycomb** (§2.15) | Hosted tooling is better and costs founder-hours rather than RAM on the inference box | If Prometheus/Grafana/Tempo memory competes with the model, move errors to Sentry first |
| **SearXNG rather than a paid search API** (§4.1) | Metasearch is fragile and depends on engines that block it | If search reliability causes user-visible failures, promote Brave to primary — it does not violate the local-inference constraint |
| **systemd rather than containers** (§8.2) | Containers give reproducibility that host discipline does not | If host dependency drift causes an incident — but weigh against GPU passthrough friction |
| **TanStack Router rather than React Router** (§1.4) | React Router is the incumbent with far more support material | At the first sign of type-inference friction; it is a day of work to switch |
| **`sqlx` rather than Diesel** (§2.4) | A composable query DSL and stronger migration tooling | If the offline-metadata workflow becomes a persistent CI annoyance |
| **No JS rendering** (§4.3) | A meaningful share of pricing pages are client-rendered and will read as empty | If "not found" rates on pricing sections stay high and trace to client rendering |
| **Local extraction rather than a hosted extractor** (§2.10) | Pricing tables are where naive extraction fails, and pricing is the product's most-used section | If extraction quality, not the model, remains the top failure cause in the daily review sample |
