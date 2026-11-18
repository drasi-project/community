# Setting up and running the Ignite demo

This guide takes you from nothing to a running presentation and demo, and covers
everyday use. Follow the sections in order the first time. Afterwards you only
need [Start the demo](#start-the-demo) and [Stop the demo](#stop-the-demo).

Everything runs locally on one Apple Silicon Mac. No cloud account, GPU or paid
AI service is needed. Model deployments and capacity are simulated; the requests,
monitoring and Drasi processing are real.

## What you will run

Addresses are this copy's defaults from `demo/ports.env` (see
[Ports and settings](#ports-and-settings)).

| Part | What it is | Address |
|---|---|---|
| Presenter desk and projector window | The desk you present from, and the window it drives on the projector (slides, dashboards, recordings) | http://localhost:19580/presenter, and `/present` (use `localhost`, not `127.0.0.1`: see [Troubleshooting](#troubleshooting)) |
| Grafana | The two dashboards, shown inside the projector window | http://localhost:14000 |
| Langfuse | Traces of individual AI requests (not projected) | http://localhost:14001 |
| Mattermost | The incident channel `#incident-ai-support`, shown inside the projector window: Grafana's alerts and Drasi's incident cards, with the Approve button | http://localhost:19065 |
| Drasi Server | Runs the continuous queries, models and reactions | http://127.0.0.1:19421 |
| AI service | The simulated AI service, its traffic and the demo controls | http://127.0.0.1:19500 |
| Supporting containers | PostgreSQL, Prometheus, OpenTelemetry Collector, Grafana, Langfuse and its storage, Mattermost, and an integration-test probe | Run in Podman |

## First-time setup

Allow about an hour, most of it waiting for the first build.

### 1. Install the tools

```sh
brew install podman docker-compose node python cmake protobuf jq
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

`docker-compose` is only the Compose command that Podman uses; Docker Desktop is
not needed. The demo pins its own Rust version, which is downloaded automatically
on the first build.

Create and start a Podman machine with enough memory for ten containers:

```sh
podman machine init --cpus 5 --memory 8192
podman machine start
```

### 2. Get the code

Put the three repositories side by side in one folder, on these branches:

```sh
mkdir ignite-native && cd ignite-native
git clone -b agentofreality-ignite-native https://github.com/drasi-project/community.git
git clone -b agentofreality-ignite-native https://github.com/drasi-project/drasi-core.git
git clone -b agentofreality-ignite-native https://github.com/drasi-project/drasi-server.git
```

The drasi-core and drasi-server branches carry fixes the demo needs (see
[DEVELOPERS.md](DEVELOPERS.md#dependencies-on-drasi-core-and-server)). Builds
go to `.cargo-target` in this top folder.

All remaining commands run from the demo folder:

```sh
cd community/presentations/2026-11-19_Ignite/demo
```

### 3. Create local passwords

```sh
./ops/observability-stack.sh env
```

This writes random passwords to `demo/.env` once and keeps them on later runs.
Keep `.env` private and never delete it while the containers' data exists.

### 4. Build the test probe and start the containers

```sh
./ops/observability-stack.sh up -d --wait postgres
./ops/observability-build.sh
./ops/observability-stack.sh up -d --wait
```

`observability-build.sh` builds the integration-test probe image
(`localhost/ignite-native-probe:phase1`, named after the project in
`ports.env`). The first run downloads the container images; afterwards they
work offline.

Starting Grafana also creates the Grafana token that Drasi's annotation
reaction uses: `up` runs `ops/observability-env.py grafana-token`, which creates
the `drasi-annotations` service account and writes `IGNITE_GRAFANA_TOKEN` to
`.env` (it keeps a token that still works). It prints only whether it wrote one.

Starting Mattermost also sets it up: `up` creates its database in this copy's
PostgreSQL, then runs `ops/mattermost.py bootstrap`, which creates what is
missing: users `admin` and `oncall` (the on-call engineer, who you act as), team
`ai-platform`, channel `#incident-ai-support`, the `drasi` bot, their access
tokens, and an incoming webhook for Grafana's alerts. Secrets go to `.env`
(never printed); the channel's IDs go to `.native/mattermost.json`. The first
time, it recreates Grafana so its alerts can post to the channel. Mattermost
publishes container images for amd64 only, so compose builds an ARM64 image
(`localhost/ignite-native-mattermost:12.1.0`) from Mattermost's official
release archive the first time; that needs a network connection.

### 5. Build Drasi Server, the plugins and the AI service

```sh
npm --prefix ../../../../drasi-server/ui ci
set -a; . ./.env; set +a
./ops/native-build.sh
```

The first build takes 20–40 minutes; later builds are much faster.

### 6. Create the demo database

```sh
python3 ops/native-runtime.py init
```

Run this once. It is safe to repeat, but never needed for a normal start.

### 7. Build the slides and presenter desk

```sh
cd ui
npm ci
npm run build
cd ..
```

### 8. Add the Ignite template artwork and fonts

The slides follow the Microsoft Ignite session template, but its artwork, the
session survey's QR code and the Segoe UI fonts can't be in this public
repository. Extract them on the presenting machine:

```sh
python3 ops/template-assets.py
```

It reads three things, by default from these places (pass `--template`,
`--survey` or `--fonts` to use others):

| What | Default |
|---|---|
| The Ignite session template | `~/Downloads/Microsoft Ignite FY27_16-9 Event-template_v05.potx` |
| The session survey slides | `~/Downloads/Session Survey Slides.pptx` |
| Segoe UI (`segoeui-*.woff`) | Microsoft PowerPoint's font cache: `/Applications/Microsoft PowerPoint.app/Contents/Resources/sdx/FA000000002/OfflineFiles` |

It writes the walk-in slide, the title background, the survey slide's
background, frame and QR code, Segoe UI and a `manifest.json` to
`ui/public/template/` (git-ignored), which the web app serves at `/template/`
straight away: no rebuild or restart. The walk-in slide is rendered from the
template with LibreOffice and PyMuPDF (`brew install --cask libreoffice`,
`pip install pymupdf`); without them you get its background only, and the script
says so. Run it again whenever the template or the survey slides change.

Without these files the talk still works: slides use the template's plain Dark
Blue background and the system font, the walk-in shows a plain Microsoft Ignite
title, and the survey slide has no QR code. The presenter desk's header then
says **Ignite template assets missing**.

Setup is done. Continue with [Start the demo](#start-the-demo).

## Start the demo

You need three terminal windows, all starting in the `demo` folder.

**Before you start:** make sure Podman is running and the containers are up.

```sh
podman machine start
./ops/observability-stack.sh up -d --wait
```

**Terminal 1: Drasi Server**

```sh
set -a; . ./.env; set +a
python3 ops/native-runtime.py run
```

**Terminal 2: AI service**

```sh
set -a; . ./.env; set +a
python3 ops/workload.py run
```

**Terminal 3: presenter desk and projector window**

```sh
cd ui
npm run preview
```

**Check it is ready.** Open the presenter desk and wait until its header says
**READY**: Drasi's answers are streaming to it. This usually takes under a
minute; terminal 1 first deletes rows from earlier scenario runs, which keeps
startup quick. To check Grafana's side from a terminal, every line of this
should end in `1`:

```sh
curl -s http://127.0.0.1:19423/metrics | grep drasi_reaction_input_ready
```

## Get ready to present

Open the presenter desk at http://localhost:19580/presenter and follow
[PRESENTER.md](PRESENTER.md#set-up-your-screens): it covers the screens, the
checklist, the script, every control and what to do if something goes wrong.

**Sign the projector in to Mattermost** once, in the browser that runs the
desk and projector: open http://localhost:19500/mattermost/sign-in (the AI
service must be running). It signs you in as `oncall` and opens
`#incident-ai-support`; the projector's Mattermost view then works. No password
is needed. The sign-in lasts 30 days.

The projector shows Grafana read-only, without signing in. To use Grafana or
Langfuse yourself (for example to answer a question), sign in privately on the
laptop: Grafana as `admin` with `GRAFANA_ADMIN_PASSWORD`, Langfuse with
`LANGFUSE_ADMIN_EMAIL` and `LANGFUSE_ADMIN_PASSWORD`. To see these values, from
`demo`:

```sh
grep -E '^(GRAFANA_ADMIN_PASSWORD|LANGFUSE_ADMIN_EMAIL|LANGFUSE_ADMIN_PASSWORD)=' .env
```

## Stop the demo

1. On the presenter desk, press **Stop demo traffic** (All demo controls).
2. Press Ctrl-C in terminal 3, then terminal 2, then terminal 1.
3. Stop the containers: `./ops/observability-stack.sh stop`

Your data is kept. Next time, go to [Start the demo](#start-the-demo).

## Reset the scenario

On the presenter desk, open **Before and after the talk** and press **Reset scenario…** then **Confirm reset**, or from `demo`:

```sh
python3 ops/workload.py reset
```

This restores the starting state and cancels waiting requests. It also deletes
the previous run's decision events from Grafana; other Grafana and Langfuse
history is kept. Never reset in the middle of a talk.

## After pulling new code

Stop the demo, then rebuild and start again:

```sh
set -a; . ./.env; set +a
./ops/native-build.sh
(cd ui && npm run build)
```

Only if the change notes say the database changed, also run
`python3 ops/native-runtime.py migrate` before starting.

## Ports and settings

Every port, the container project name, the PostgreSQL publication and slot,
and the browser storage key come from one file, `demo/ports.env`. The scripts,
Compose, Drasi Server's configuration, the AI service, the web app and the tests
all read it. An exported variable of the same name overrides a line, and
`IGNITE_SETTINGS=<file>` selects another file. The full list is in
[DEVELOPERS.md › Ports](DEVELOPERS.md#ports).

After changing a port, rebuild (`./ops/native-build.sh` and `npm run build` in
`ui`), restart the containers, and regenerate Grafana's dashboards
(`python3 observability/grafana/dashboards.py`) if the Langfuse port changed.

### Running beside the original demo

This copy can run on the same Mac as the original demo (the `ignite` folder,
branch `agentofreality-ignite`) at the same time. `ports.env` gives it every
port +1000, its own container project (`ignite-native`, so its own containers
and volumes), its own replication publication and slot (`ignite_rework`) and its
own browser storage key. The original's values are in `ports.default.env`;
never use that file here while the original is running, because it would share
the original's ports, containers and data.

The two copies need separate `.env` files, probe images and builds, which the
steps above already give them.

## Troubleshooting

| Problem | What to do |
|---|---|
| Grafana charts are empty | Start demo traffic on the presenter desk and wait a minute. |
| Grafana tiles say **Unknown** | Drasi's answers haven't loaded. Right after a start, wait for the desk to say READY and run the `curl` check in [Start the demo](#start-the-demo). |
| Everything stays Unknown for a long time | A deployment may have been left failed, so requests kept retrying and history grew very large. On the desk, open **All demo controls** and restore every deployment (**Any deployment** › Restore), then **Before and after the talk** › **Reset scenario…**, then restart terminals 1 and 2. |
| Drasi Server takes minutes to start | The current run has a lot of history. Wait; it will finish (it allows up to 20 minutes). Next time, reset the scenario before restarting. |
| The projector's Mattermost view shows a sign-in page | Open http://localhost:19500/mattermost/sign-in once in the same browser. |
| The Mattermost view shows its sign-in page again, or "session expired" right after signing in | Open the desk at http://localhost:19580/presenter, not `127.0.0.1`. Mattermost's cookies are SameSite=Lax, and `127.0.0.1` and `localhost` are different sites, so a stage on `127.0.0.1` frames the channel without its cookies. The web app moves a `127.0.0.1` address to `localhost` by itself; reopen the projector window from the desk afterwards. |
| Terminal 1 stops with "IGNITE_MATTERMOST_BOT_TOKEN must be exported" or "Mattermost isn't bootstrapped" | Run `./ops/observability-stack.sh up -d --wait mattermost`, which writes the tokens to `.env` and the channel's IDs to `.native/mattermost.json`, then load `.env` again. If it says the bundle changed, run `./ops/native-build.sh`. |
| Terminal 1 stops with "IGNITE_GRAFANA_TOKEN must be exported" | Start Grafana with `./ops/observability-stack.sh up -d --wait grafana`, which writes the token to `.env`, then load `.env` again and restart terminal 1. |
| No decision events in Grafana | Check the token: `python3 ops/observability-env.py grafana-token` replaces it if it no longer works. Restart terminal 1 after it writes a new one. |
| "Run ops/observability-stack.sh env first" | Do [step 3](#3-create-local-passwords). |
| A port is already in use | Something is already running. Find it with `lsof -nP -iTCP:19421 -sTCP:LISTEN` (change the port) and stop it before starting another copy. |
| Startup refuses because files changed | Rebuild: `./ops/native-build.sh`. |
| Containers fail to start | Check Podman is running: `podman machine start`, then `./ops/observability-stack.sh ps`. |
| The projector window shows "Grafana · …" but no dashboard | Grafana isn't running or hasn't loaded yet: check `./ops/observability-stack.sh ps`, then press **Back** and **Next** on the desk. |
| Desk header says "Projector window not opened from this desk" | Press **Open projector window** in the header, and use that window on the projector. |

## For developers

To change or test the demo, see [DEVELOPERS.md](DEVELOPERS.md).
