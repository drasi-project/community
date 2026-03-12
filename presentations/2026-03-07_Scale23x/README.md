# Drasi: A New Take on Change-Driven Architectures

Presentation materials for [SCaLE 23x](https://www.socallinuxexpo.org/scale/23x) (March 7, 2026) — a session introducing [Drasi](https://drasi.io), an open-source Data Change Processing platform (CNCF Sandbox), and the change-driven architecture paradigm.

## About the Presentation

This talk explores the challenges of building systems that detect and react to meaningful changes — or the absence of change — across distributed data sources. It introduces Drasi as a purpose-built platform that replaces polling, ETL pipelines, and hand-coded change-detection logic with declarative continuous queries.

The presentation covers:

1. **The Problem** — Why detecting and reacting to change is harder than it should be, illustrated through real-world scenarios (proactive customer care, field service dispatch, AI/RAG freshness).
2. **Trading Application Example** — A concrete change-driven system with market data, portfolios, order management, and watchlists.
3. **Building Change-Driven Systems Today** — The common approaches (polling, CDC, event-driven) and their trade-offs.
4. **Introducing Drasi** — How Drasi's Sources, Continuous Queries, and Reactions simplify change-driven architectures.
5. **Live Demo** — The Drasi Trading application running end-to-end, embedded directly in the slides.
6. **Getting Started** — Resources, community links, and next steps.

## How the Presentation Works

The slide deck is built with [Reveal.js](https://revealjs.com/) v5.1.0, a JavaScript presentation framework. Key implementation details:

- **`presentation/index.html`** — The single-file slide deck. All slides are defined as nested `<section>` elements within the Reveal.js structure.
- **`presentation/css/custom.css`** — Custom styles layered on top of Reveal.js's built-in `black` theme.
- **`presentation/images/`** — Organized into subdirectories: `backgrounds/`, `diagrams/`, `drasi-logo/`, `icons/`, and `logos/`.
- **Speaker notes** — Embedded in each slide using `<aside class="notes">` elements, visible in the speaker view.
- **Progressive reveals** — Content marked with the `fragment` CSS class appears incrementally on click/advance.
- **Vertical slides** — Sections use nested `<section>` elements to create vertical slide stacks navigable with ↓/↑.
- **Live demo embed** — The demo section uses `data-background-iframe` to embed the running Trading application (at `http://localhost:5273`) directly in a slide, with `data-background-interactive` enabling live interaction.
- **Slide dimensions** — Configured at 1920×1080 with minimal margin (0.02), optimized for full-screen presentation on standard displays.

## Prerequisites

- **[Node.js](https://nodejs.org/)** v18 or later — required to install dependencies and serve the presentation.
- **Running Drasi Server** — the live demo section connects to a Drasi instance. Refer to the [Drasi documentation](https://drasi.io/getting-started/) for setup instructions.
- **Running Trading Example Application** — the demo slide embeds the Trading web app via an iframe at `http://localhost:5273`. The Trading Example must be deployed and connected to the Drasi Server before presenting. See the [Drasi Trading Demo](https://github.com/drasi-project) repository for deployment instructions.

> **Note:** The presentation slides will load and display without the demo prerequisites, but the live demo slide will show a blank/error frame if the Trading application is not running on port 5273.

## Running the Presentation

```bash
./start.sh
```

This script:
1. Changes to the `presentation/` directory
2. Installs npm dependencies (if `node_modules/` doesn't exist)
3. Serves the presentation at **http://localhost:3000**

Alternatively, you can start it manually:

```bash
cd presentation
npm install
npx serve . -l 3000
```

Then open **http://localhost:3000** in your browser.

## Navigating the Presentation

| Key | Action |
|-----|--------|
| `→` or `↓` | Next slide (→ horizontal, ↓ vertical) |
| `←` or `↑` | Previous slide |
| `Space` | Next slide / next fragment |
| `Esc` | Toggle slide overview (scroll to pan) |
| `S` | Open speaker notes in a new window |
| `F` | Toggle fullscreen |
| `Home` (`Fn + ←` on Mac) | Jump to first slide |
| `#/N` in URL | Jump to a specific slide by number |

**During the live demo slide**, the Trading application is interactive — click and scroll within the embedded app. Press `Esc` to exit the interactive mode and return to slide navigation.

## Files

```
README.md               # This file
start.sh                # One-command launcher
presentation-script.md  # Full talk script (AI Generated, for reference only, not used at Scale 23x)
speaker-notes.md        # Speaker notes reference (extracted from presentation/index.html for easy reading)
Drasi_ A New Take on Change-Driven Architectures.pdf  # PDF version of the presentation (for offline viewing)
presentation/           # Reveal.js slide deck
├── index.html          # Slide content
├── package.json        # Node.js dependencies (reveal.js, serve)
├── css/custom.css      # Custom theme styles
└── images/             # Presentation assets
    ├── backgrounds/    # Background images
    ├── diagrams/       # Architecture and concept diagrams (SVG)
    ├── drasi-logo/     # Drasi logos and QR code
    ├── icons/          # Icon SVGs (technology, feature, and community icons)
    └── logos/          # Third-party product logos
```

## Links

- **Drasi:** [drasi.io](https://drasi.io)
- **GitHub:** [github.com/drasi-project](https://github.com/drasi-project)
- **Discord:** [aka.ms/drasidiscord](https://aka.ms/drasidiscord)
- **Talk listing:** [SCaLE 23x — Allen Jones](https://www.socallinuxexpo.org/scale/speaker/allen-jones)
