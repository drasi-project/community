# Drasi: A New Take on Change-Driven Architectures

## Full Presentation Script — SCaLE 23x | Allen Jones

> This script follows the exact slide order in the deck. Deck interactions are marked with `► CLICK` for fragment advances and `► ADVANCE` for slide transitions. Demo instructions are in `[brackets]`.

---

## TITLE SLIDE

**[On screen: Drasi logo, subtitle, "Presented at SCaLE 23x", contact info, drasi.io links]**

Hey everyone, thanks for being here at SCaLE 23x. My name is Allen Jones. Today we're going to talk about something I think every person in this room has dealt with — building systems that detect and react to changes in data. We'll look at why it's harder than it should be, walk through the common approaches and their trade-offs, and then I'll introduce you to Drasi — a CNCF Sandbox project that takes a fundamentally different approach.

We'll see it in action with a live demo, and by the end you'll have everything you need to try it yourself.

► ADVANCE

---

## SECTION I — THE PROBLEM

---

### The Problem

**[On screen: "Detecting and Reacting to Change is Harder Than It Should Be"]**

"Detecting and reacting to change is harder than it should be."

*[Pause 2-3 seconds. Let it resonate.]*

I want you to sit with that. Because I'm willing to bet most of you have built some version of this. The polling loop. The webhook parser. The glue code connecting system A to system B. It always starts out simple... until it isn't.

► ADVANCE

---

### A Familiar Scenario

**[On screen: Three fragment bullets + "Quickly. Precisely. Reliably." callout]**

Here's the pattern.

► CLICK — You have data spread across multiple systems — databases, APIs, Kubernetes, SaaS platforms. Data is everywhere.

► CLICK — That data changes. Or — and this is important — it *doesn't* change when you expected it to. The absence of change can be just as meaningful.

► CLICK — And your system needs to detect that and react.

► CLICK — Quickly. Precisely. Reliably.

Sounds simple. The implementation? That's where things get messy.

► ADVANCE

---

### Example: Proactive Customer Care

**[On screen: Shipping delay scenario paragraph]**

Let me make this concrete. A shipping carrier marks a delivery as "delayed." Your e-commerce platform needs to identify affected customers, check their loyalty tier, and offer compensation — *before* they complain.

The carrier system knows nothing about your customers. Nothing about loyalty tiers. To react to this one change, you need to join data across system boundaries — carrier, order management, customer database — and do it fast.

► ADVANCE

---

### Example: Field Service Dispatch

**[On screen: Live technician matching across GPS, ticketing, HR, dispatch]**

You need a live set of technicians who are near an open high-priority ticket, certified for the equipment, and not already dispatched. Four systems. GPS, ticketing, HR, dispatch.

The challenge isn't finding qualified technicians *once*. It's maintaining a live view as GPS positions update, tickets change status, and dispatches happen. The answer is different every few seconds.

► ADVANCE

---

### Example: AI Support Agent

**[On screen: Stale embeddings scenario + "What if..." callout fragment]**

A customer calls about a missing order. Your AI support agent searches its knowledge base — but the embeddings were refreshed overnight. It doesn't know the order was rerouted two hours ago. The agent confidently gives the *wrong* answer.

*[Pause — let the scenario land]*

► CLICK — "What if the vector DB updated the moment the order status changed — not on the next batch run?"

That's not just a backend problem. It's an AI infrastructure problem. Your RAG pipeline is only as good as the freshness of your data.

► ADVANCE

---

### Sound Familiar?

**[On screen: Definition blockquote + two progressive callouts]**

These are all examples of Change-Driven Systems.

*[Read the blockquote]* "Systems that take action in response to a meaningful change — or the absence of change — in relevant data."

Two key phrases. "Meaningful change" — not every row update matters. And "absence of change" — detecting what *didn't* happen.

► CLICK — A problem you've probably already "solved"...

► CLICK — ...more than once!

*[Let the audience nod]*

► ADVANCE

---

## SECTION II — EXAMPLE: TRADING APPLICATION

---

### Example: Trading Application

**[On screen: Two-column layout — architecture diagram + feature list with "All Dynamic" callout]**

Before we dive into approaches, let me show you a concrete change-driven system.

This is a trading application. Market data, portfolio, orders, watchlist, sector overviews, gainers and losers — every single one dynamic. Not refreshing every 30 seconds. Updating the moment something changes.

Let me show you what this looks like live.

► ADVANCE

---

### Live Trading App Demo

**[On screen: Embedded iframe — localhost:5273]**

*[Show the trading app for 1-2 minutes. Point out: prices updating, portfolio recalculating, gainers shuffling. Everything is live.]*

*[Press Esc to exit interactive mode]*

► ADVANCE

---

### Trading Application: How to ...

**[On screen: Two-column — expanded diagram + "How" questions + "Oh no" fragments]**

OK, that looks great. But how do you actually build it?

► CLICK — *[Questions appear]* How do you get data from the sources? Integrate across multiple systems? Update all dependencies on change? Push to the UI in real time? Handle concurrency? Recover from failures?

*[Pause — let the weight accumulate]*

► CLICK — Oh no...

► CLICK — ...not again!

*[Play for laughs]* That sinking feeling of realizing you're about to build the same infrastructure *again*.

► ADVANCE

---

## SECTION III — BUILDING CHANGE-DRIVEN SYSTEMS TODAY

---

### Building Change-Driven Systems / How Teams Do It Today

**[On screen: Section header]**

So let's look at how teams solve this today. Three approaches, each with real strengths, each with trade-offs. I'm not dismissing any of them — I'm showing the shared limitations.

► ADVANCE

---

### Three Common Approaches

**[On screen: Numbered list]**

Polling. Analytics Platforms. Event Stream Processing. Let's take them one at a time.

► ADVANCE

---

### Approach 1: Polling

**[On screen: Description + polling diagram + fridge analogy callout]**

Polling. Everyone's first solution. Timer, query, compare, act. Simple mental model, no new infrastructure.

*[Gesture to the callout, deadpan delivery]* "Polling a database is like opening the fridge every five seconds to see if new food appeared."

*[Pause for the laugh]*

► ADVANCE

---

### Approach 2: Analytics Platforms

**[On screen: Description + analytics pipeline diagram]**

Analytics platforms. Centralize all your data — rich queries, dashboards, surface meaningful conditions. Solves the multi-source problem elegantly.

But the model is fundamentally retrospective. You're analyzing a *copy* of data that's already minutes or hours stale. Great for insight. Not great for reaction.

► ADVANCE

---

### Approach 3: Event Stream Processing

**[On screen: Description + CDC pipeline diagram]**

Event stream processing. CDC captures row-level changes, streams them to Kafka or Flink for filtering, joining, transformation.

Genuinely powerful technology. But these are *event-driven* tools — excellent at moving events. Building change-driven *logic* on top still requires substantial engineering.

► ADVANCE

---

### The Problems

**[On screen: Three problem cards appear simultaneously + punchline callout]**

Different tools, shared pain.

► CLICK — *[All three cards appear at once]*

Polling: detection lag equals your poll interval. Most polls return nothing. You own all the state. Every use case needs its own loop. Doesn't scale.

Analytics: cost prohibitive. Stale data. ETL lag. Insight, not reaction.

Event streaming: heavy infrastructure. Consumer owns all the logic. Cross-source joins are painful. Weeks to production.

► CLICK — "I just want to know if something changed!"

Most of your effort goes to plumbing, not business logic.

► ADVANCE

---

### There has to be an easier way!

**[On screen: Wishlist + open source callout]**

► CLICK — What we really need...

► CLICK — *[List appears]* Purpose-built. Simple. Unified detection and action. Works with existing data. Extensible.

*[Emphasize the callout]* Open source, community-driven, no vendor lock-in.

► ADVANCE

---

## SECTION IV — INTRODUCING DRASI

---

### Introducing [Drasi Logo]

**[On screen: "Introducing" + large Drasi logo]**

*[Brief pause — let the logo land]*

We've established the problem. Let me introduce the solution.

► ADVANCE

---

### What is Drasi?

**[On screen: One-liner definition + "Open Source, Apache 2.0, CNCF Sandbox"]**

Drasi is a data change processing platform. Not event streaming. Not stream processing. *Data change processing* — that distinction matters.

Declarative continuous queries to isolate meaningful changes and trigger reactions. Open source. Apache 2.0. CNCF Sandbox since January 2025.

► ADVANCE

---

### Core Concepts

**[On screen: "Three simple concepts" + animated diagram swap]**

Three concepts. That's the entire foundation.

*[Simple diagram visible]* Sources connect to your existing data. Continuous Queries define what changes matter. Reactions act when results change. Source. Query. Reaction.

► CLICK — *[Detailed diagram appears with concrete examples]*

PostgreSQL, Cosmos DB, Kubernetes as Sources. openCypher queries. Webhooks, SSE, gRPC as Reactions.

*[Let the simplicity sink in]*

► ADVANCE

---

### Sources (How They Work)

**[On screen: Four bullets + Source Change Events diagram]**

Sources manage the connection, listen for changes — WAL logs, change feeds, event streams — and transform them into Source Change Events. Insert, Update, Delete.

The source handles all the CDC complexity. By the time data reaches the query engine, it's a clean, typed event.

► ADVANCE

---

### Source Ecosystem

**[On screen: Animated accent rows by category]**

► CLICK — Relational: PostgreSQL, MySQL, SQL Server. Tailing the WAL. No app code changes.

► CLICK — Graph and document: Cosmos DB, Dataverse, Kubernetes.

► CLICK — Streams: Event Hubs, HTTP, gRPC.

► CLICK — Custom: build your own with the SDK.

Drasi connects to what you already run.

► ADVANCE

---

### Continuous Query (How It Works)

**[On screen: Four bullets + internal flow diagram with three indexes]**

Continuous Queries subscribe to Sources, process Source Change Events, and maintain a perpetually accurate result set. Three internal indexes: elements, aggregations, results.

When data changes, the engine diffs the result set and sends Result Change Notifications: Added, Updated with before-and-after, Deleted. Clean, typed, no parsing required.

► ADVANCE

---

### Example: All Stock Prices

**[On screen: Cypher query fragment + result table fragment]**

► CLICK — *[Query appears]* Match all stock prices, compute change percentage. Looks like a normal query.

► CLICK — *[Table appears]* Sample output — green for gains, red for losses.

But this isn't a one-time SELECT. It runs perpetually. Every price update, Drasi emits only the diff.

► ADVANCE

---

### Example: Gainers Only

**[On screen: Cypher query with MATCH relationship + WHERE + result table]**

► CLICK — *[Query]* Relationship traversal plus a filter — only stocks above yesterday's close.

► CLICK — *[Table — only gainers]* As prices move, stocks enter and leave this set automatically. Graph syntax, not SQL JOINs.

► ADVANCE

---

### Example: Sector Aggregation

**[On screen: Cypher query with aggregations + result table]**

► CLICK — *[Query]* Sector rollups: count, average change, total volume, min, max. All live.

► CLICK — *[Table]* Every price change recalculates sector aggregates. Drasi maintains aggregation indexes — no recomputing from scratch.

► ADVANCE

---

### Reactions (How They Work)

**[On screen: Three bullets + Result Change Events diagram]**

Reactions subscribe to queries, receive typed notifications — Added, Updated, Deleted — and act. The diagram shows the flow from Continuous Query to downstream system. Clean and unambiguous.

► ADVANCE

---

### Reaction Ecosystem

**[On screen: Animated accent rows by category]**

► CLICK — Push: Webhooks, SSE, gRPC, SignalR.

► CLICK — Write: PostgreSQL, MySQL, SQL Server, Gremlin, Qdrant.

► CLICK — Integrate: Debezium, Event Grid, Dapr, Grafana.

► CLICK — Custom: build your own.

One query, many reactions. One reaction, many queries.

► ADVANCE

---

### Advanced Capabilities

**[On screen: Six cards in 3x2 grid]**

Quick overview — I won't read every bullet.

► CLICK through cards — Extensibility: everything is pluggable. Middleware: transform data before queries see it. Multi-source: join across databases, no shared schema. Absence of change: trueFor, trueLater, trueUntil. Temporal: compare with historical values. Change-driven UI: React and Vue components.

Each of these is a talk in itself. The point is depth beyond the basics.

► ADVANCE

---

### Deployment Options

**[On screen: Three deployment cards]**

► CLICK — Drasi for Kubernetes: full platform, horizontal scaling, any K8s distribution.

► CLICK — drasi-lib: Rust crate, embed in-process. Edge, IoT, custom pipelines.

► CLICK — Drasi Server: Docker container, built-in web UI. Fastest start.

Same queries across all three. Start on your laptop, scale to production.

► ADVANCE

---

## SECTION V — DEMO

---

### Demo: Drasi Trading

**[On screen: Title + architecture diagram fragment]**

Alright, enough slides. Let's see Drasi powering the trading app.

► CLICK — *[Architecture diagram appears]*

Sources on the left, Drasi in the middle, trading app on the right. Let me show you this live.

► ADVANCE

---

### Live Demo

**[On screen: Embedded iframe — localhost:5273]**

*[Interactive demo — 3-5 minutes]*

*[Walk through:]*
- *Show configured Sources, Continuous Queries, Reactions in the Drasi UI*
- *Make a data change*
- *Watch it propagate: source change -> query result update -> reaction -> UI update*
- *Highlight speed — milliseconds*

*[Press Esc to exit]*

► ADVANCE

---

## SECTION VI — CLOSING

---

### Why Drasi

**[On screen: Subtitle + six differentiator bullets + CNCF callout]**

So why Drasi?

Purpose-built. One query replaces polling loops, ETL pipelines, and consumer logic. Remember all those problems? Gone.

Simple — detect change and take action in one declarative query. Join across databases with no shared schema. Detect what didn't happen. Run anywhere.

*[Gesture to callout]* CNCF Sandbox. Apache 2.0. Community-driven. Extensible. No lock-in.

► ADVANCE

---

### Getting Started

**[On screen: Resource table with icons + QR code]**

Getting started is quick. drasi.io has quickstarts, tutorials, and docs. Code's on GitHub — all open source. Try Drasi Server locally with Docker in minutes. CNCF Sandbox, open governance, Apache 2.0.

*[Point to QR code]* Scan that — goes straight to drasi.io.

Thank you, everyone! I'm Allen Jones. Happy to take questions now, and I'll be around afterward.

---

## DELIVERY TIPS

- **Pause after "harder than it should be."** Let the audience nod.
- **The fridge analogy is your humor moment.** Deadpan delivery, then pause.
- **"Oh no... not again!"** — play it up for knowing laughs.
- **AI Support Agent example** is the most current and relevant. Give it room.
- **Core Concepts is where you slow down.** Three concepts — let simplicity land.
- **Query examples build in complexity.** Simple -> filtered -> aggregated. Don't rush.
- **The demo is the climax.** Keep slides brisk getting there.
- **"Why Drasi" is the payoff.** Connect each bullet back to a pain point.
- **SCaLE audience:** Lead with open source, Apache 2.0, PostgreSQL, Webhooks. Mention CNCF governance. Avoid Azure-first framing.
- **End strong.** QR code, thank you, questions — don't trail off.

---

## TIMING GUIDE

| Section | Slides | Target |
|---|---|---|
| I. The Problem | 5 | ~8 min |
| II. Trading App Example | 3 | ~5 min |
| III. Current Approaches | 6 | ~10 min |
| IV. Introducing Drasi | 13 | ~18 min |
| V. Demo | 2 | ~5 min |
| VI. Closing | 2 | ~4 min |
| **Total** | **31** | **~50 min + Q&A** |
