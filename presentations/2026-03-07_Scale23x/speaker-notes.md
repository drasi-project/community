# Speaker Notes Reference

## Drasi: A New Take on Change-Driven Architectures — SCaLE 23x

---

### Slide 1 — A New Take on Change-Driven Architectures

> PURPOSE: Set context — who you are, what we're covering.
> KEY POINTS: Change-driven architecture is a universal problem. Drasi is a CNCF project that simplifies it.
> SEQUENCE: Brief intro → transition to "The Problem."

---

### Slide 2 — Detecting and Reacting to Change is Harder Than It Should Be

> PURPOSE: Section opener — establish the central premise.
> KEY POINTS: Pause and let the statement resonate. Everyone has lived this.
> SEQUENCE: Hold for 2-3 seconds, then advance.

---

### Slide 3 — A Familiar Scenario

> PURPOSE: Build the problem pattern with progressive reveals.
> KEY POINTS: Data everywhere → changes (or doesn't) → must react. Plant "absence of change" seed early.
> SEQUENCE: Click through each bullet. Land the "Quickly. Precisely. Reliably." callout with weight.

---

### Slide 4 — Example: Proactive Customer Care

> PURPOSE: Concrete example — cross-source correlation under time pressure.
> KEY POINTS: Carrier knows nothing about customers/loyalty. Reaction requires joining across system boundaries.

---

### Slide 5 — Example: Supply Chain Deadlines

> This is "absence of change" — detecting something that DIDN'T happen by a deadline.
> No single system owns this logic. Traditional event-driven systems struggle here
> because there's no event to consume when nothing happens.

---

### Slide 6 — Example: Field Service Dispatch

> PURPOSE: Example of a continuously changing result set across 4 systems.
> KEY POINTS: Not a one-time query — a live view. GPS, tickets, HR, dispatch all change independently.

---

### Slide 7 — Example: AI Support Agent

> PURPOSE: AI/RAG angle — stale vector DB = wrong answers.
> KEY POINTS: Batch-refreshed embeddings miss recent changes. Drasi enables incremental, immediate updates.
> SEQUENCE: Let the scenario land → click the callout as the "fix."

---

### Slide 8 — Sound Familiar?

> PURPOSE: Name the pattern and define it. Anchor slide for the talk.
> KEY POINTS: "Meaningful change" (not every update) + "absence of change" (what didn't happen).
> SEQUENCE: Read the blockquote → click "already solved" → click "more than once" — let audience nod.

---

### Slide 9 — Example: Trading Application

> PURPOSE: Ground the abstract problem in a real system before explaining approaches.
> KEY POINTS: Walk through the feature list. Emphasize "All Dynamic" — everything updates live.

---

### Slide 10 — Example: Trading Application

> PURPOSE: Live demo — show the trading app's dynamism.
> KEY POINTS: Prices updating, portfolio recalculating, gainers shuffling. Keep to 1-2 minutes.
> SEQUENCE: Press Esc to exit interactive mode when done.

---

### Slide 11 — Trading Application: How to ...

> PURPOSE: Reveal the engineering complexity behind the demo. Create the "not again" moment.
> KEY POINTS: Each bullet = real engineering work. Six hard questions, zero easy answers.
> SEQUENCE: Click to reveal questions → pause → "Oh no" → "not again" — play for knowing laughs.

---

### Slide 12 — How Teams Do It Today

> PURPOSE: Section opener — transition from "what's the problem" to "how do people solve it."
> KEY POINTS: Three approaches, each with trade-offs. Not dismissing any — showing shared limitations.

---

### Slide 13 — Three Common Approaches

> PURPOSE: Quick roadmap of the three approaches. Move through quickly.

---

### Slide 14 — Approach 1: Polling

> PURPOSE: Explain polling — the simplest approach.
> KEY POINTS: Intuitive, no new infra. Diagram shows the loop. Fridge analogy = humor moment.
> SEQUENCE: Describe the flow → pause on the fridge line → let it land.

---

### Slide 15 — Approach 2: Analytics Platforms

> PURPOSE: Explain analytics approach — powerful but retrospective.
> KEY POINTS: Rich queries across all data, but you're analyzing stale copies. ETL = delay.

---

### Slide 16 — Approach 3: Event Stream Processing

> PURPOSE: Explain CDC/event streaming — powerful but heavy.
> KEY POINTS: Battle-tested, not dismissing it. But event-driven ≠ change-driven. Heavy infra, consumer owns logic.

---

### Slide 17 — Event Stream

> PURPOSE: Consolidate all three approaches' problems into one visual.
> KEY POINTS: Cards appear together — walk through each briefly, don't linger.
> SEQUENCE: Click → all three appear → scan Polling/Analytics/Event Stream → land the callout punchline.

---

### Slide 18 — There has to be an easier way !

> PURPOSE: Pivot from problem to solution. Set up exactly what Drasi delivers.
> KEY POINTS: Purpose-built, simple, unified, extensible, open source. SCaLE audience cares about lock-in.
> SEQUENCE: Click to reveal → read conversationally → emphasize the open source callout.

---

### Slide 19 — Introducing

> PURPOSE: Big reveal moment — introduce Drasi by name.
> KEY POINTS: Let the logo speak. Brief pause before advancing.

---

### Slide 20 — What is Drasi?

> PURPOSE: Define Drasi — what it is and isn't.
> KEY POINTS: "Data change processing" — distinct from event streaming or stream processing. CNCF Sandbox, Apache 2.0.

---

### Slide 21 — Core Concepts

> PURPOSE: Introduce the three-concept mental model. This is the slide to pause on.
> KEY POINTS: Source → Continuous Query → Reaction. Click to show concrete examples.
> SEQUENCE: Show simple diagram → click → concrete examples appear. Let simplicity sink in.

---

### Slide 22 — Sources

> PURPOSE: Explain what Sources do internally.
> KEY POINTS: Connect → listen for changes → transform to graph nodes/relations → emit Source Change Events (Insert/Update/Delete).
> Source handles all CDC complexity — WAL, offsets, mapping. Query engine sees clean typed events.

---

### Slide 23 — Source Ecosystem

> PURPOSE: Show breadth of source connectors. Animated rows stream in by category.
> KEY POINTS: Relational (WAL) → Graph/Doc (change feeds) → Stream (event-based) → Custom (extensible).
> No code changes to existing apps. "Custom" row signals the extensibility model.

---

### Slide 24 — Continuous Query

> PURPOSE: Explain what happens inside a Continuous Query.
> KEY POINTS: Subscribe to Sources → openCypher/GQL queries → maintain live result set →
> emit Result Change Notifications (Added/Updated/Deleted). Three internal indexes: Element, Aggregation, Result.

---

### Slide 25 — Example: Continuous Query

> PURPOSE: Simplest query example — show it looks like a normal query but runs live.
> KEY POINTS: MATCH all stock prices, compute change %. Table shows sample output.
> Emphasize: this isn't a one-time SELECT — it's perpetual. Drasi emits diffs on every price change.

---

### Slide 26 — Example: Continuous Query

> PURPOSE: Show relationship traversal + filtering — stocks entering/leaving the result set.
> KEY POINTS: MATCH with relationship + WHERE filter. Only gainers shown. As prices move, membership changes automatically.
> Note the graph syntax — MATCH pattern, not SQL JOINs.

---

### Slide 27 — Example: Continuous Query

> PURPOSE: Show live aggregation — sector rollups that recalculate on every price change.
> KEY POINTS: count, avg, sum, min, max — all maintained incrementally via aggregation indexes.
> This is the kind of derived view that's painful to build and maintain manually.

---

### Slide 28 — Reactions

> PURPOSE: Explain what Reactions do — subscribe, receive, act.
> KEY POINTS: Typed Result Change Notifications (Added/Updated/Deleted). Diagram shows flow from CQ to downstream system.

---

### Slide 29 — Reaction Ecosystem

> PURPOSE: Show breadth of reaction connectors.
> KEY POINTS: Push (real-time delivery) → Write (back to databases) → Integrate (event systems) → Custom.
> Vendor-neutral first (Webhooks, SSE, gRPC) since this is SCaLE. One query → many reactions.

---

### Slide 30 — Change-driven UI

> PURPOSE: Quick overview of advanced capabilities — don't dwell, hit highlights.
> KEY POINTS: Extensibility model, middleware transforms, cross-database joins,
> absence-of-change detection (trueFor/trueLater/trueUntil), temporal history, React/Vue components.
> SEQUENCE: Click through cards — 1-2 sentences each, move on.

---

### Slide 31 — Drasi Server

> PURPOSE: Show deployment flexibility — meet users where they are.
> KEY POINTS: K8s (scaling), drasi-lib (embedded Rust), Drasi Server (Docker, simplest start).
> Same queries work across all three. Start on laptop → scale to production.

---

### Slide 32 — Demo: Drasi Trading

> PURPOSE: Transition to demo — show architecture before going live.
> KEY POINTS: Walk through the diagram: sources (left) → Drasi (middle) → trading app (right).
> SEQUENCE: Explain the architecture → click to reveal diagram → then advance to live demo.

---

### Slide 33 — Demo: Drasi Trading

> PURPOSE: Live demo — the climax of the talk. Show Drasi powering the trading app.
> KEY POINTS: Show Sources, Queries, Reactions in the UI. Make a data change → watch it propagate. Highlight speed.
> SEQUENCE: 3-5 minutes. Press Esc to exit interactive mode.

---

### Slide 34 — Why Drasi

> PURPOSE: Closing argument — connect Drasi back to every pain point discussed.
> KEY POINTS: Purpose-built, declarative, cross-source, absence of change, runs anywhere, open source.
> Don't read bullets — reference the talk: "Remember polling? Gone. ETL? Not needed. Consumer logic? It's a query."

---

### Slide 35 — Getting Started

> PURPOSE: Give the audience next steps and the QR code.
> KEY POINTS: drasi.io for docs/tutorials, GitHub for code, Docker for quick start, CNCF/Apache 2.0.
> SEQUENCE: Point to QR code. Thank the audience. Open for questions.
