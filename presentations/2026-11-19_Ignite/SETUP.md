# Setting up and running the Ignite demo

This guide takes you from nothing to a running presentation and demo, and covers
everyday use. Follow the sections in order the first time. Afterwards you only
need [Start the demo](#start-the-demo) and [Stop the demo](#stop-the-demo).

Everything runs locally on one Apple Silicon Mac. No cloud account, GPU or paid
AI service is needed. Model deployments and capacity are simulated; the requests,
monitoring and Drasi processing are real.

## What you will run

| Part | What it is | Address |
|---|---|---|
| Slides and presenter desk | The three slides plus a private control panel | http://127.0.0.1:18580/present and `/presenter` |
| Grafana | The dashboards you present from | http://localhost:13000 |
| Langfuse | Traces of individual AI requests | http://localhost:13001 |
| Drasi Server | Runs the continuous queries and models | http://127.0.0.1:18421/ui/?instance=ignite |
| Workload service | The simulated AI service and its traffic | http://127.0.0.1:18500 |
| Supporting containers | PostgreSQL, Prometheus, OpenTelemetry Collector, Langfuse storage | Run in Podman |

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
mkdir ignite && cd ignite
git clone -b agentofreality-ignite https://github.com/drasi-project/community.git
git clone -b agentofreality-parallel-computation-graph-recovery https://github.com/drasi-project/drasi-core.git
git clone -b agentofreality-parallel-computation-graph-recovery https://github.com/drasi-project/drasi-server.git
```

This presentation folder is not yet committed to the `community` branch. Until
it is, copy `community/presentations/2026-11-19_Ignite` from a machine that has it.

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

### 4. Start the supporting containers

```sh
./ops/observability-stack.sh up -d --wait postgres
./ops/observability-build.sh
./ops/observability-stack.sh up -d --wait
```

The first run downloads the container images. Afterwards they work offline.

### 5. Build Drasi Server, the plugins and the workload service

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

Setup is done. Continue with [Start the demo](#start-the-demo).

## Start the demo

You need three terminal windows, all starting in the `demo` folder. Start them in
this order.

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

**Terminal 2: workload service** (once terminal 1 says "Drasi Server started successfully")

```sh
set -a; . ./.env; set +a
python3 ops/workload.py run
```

**Terminal 3: slides and presenter desk**

```sh
cd ui
npm run preview
```

**Check it is ready.** In any other terminal, from `demo`:

```sh
python3 ops/workload.py status
```

Wait until it shows `"continuous_context": "ready"`. This usually takes under a
minute, but can take a few minutes if a lot of history has built up.

## Get ready to present

Sign in to the tools in your browser before projecting anything:

- Grafana: user `admin`, password `GRAFANA_ADMIN_PASSWORD`
- Langfuse: `LANGFUSE_ADMIN_EMAIL` and `LANGFUSE_ADMIN_PASSWORD`

To see these values, from `demo`:

```sh
grep -E '^(GRAFANA_ADMIN_PASSWORD|LANGFUSE_ADMIN_EMAIL|LANGFUSE_ADMIN_PASSWORD)=' .env
```

Then follow [PRESENTER.md](PRESENTER.md#before-you-present): it covers the
screens, the checklist, the script, every control and what to do if something
goes wrong.

## Stop the demo

1. Stop demo traffic on the presenter desk.
2. Press Ctrl-C in terminal 3, then terminal 2, then terminal 1.
3. Stop the containers: `./ops/observability-stack.sh stop`

Your data is kept. Next time, go to [Start the demo](#start-the-demo).

## Reset the scenario

Use **Reset scenario…** on the presenter desk, or from `demo`:

```sh
python3 ops/workload.py reset
```

This restores the starting state and cancels waiting requests. Grafana and
Langfuse history is kept. Never reset in the middle of a talk.

## After pulling new code

Stop the demo, then rebuild and start again:

```sh
set -a; . ./.env; set +a
./ops/native-build.sh
(cd ui && npm run build)
```

Only if the change notes say the database changed, also run
`python3 ops/native-runtime.py migrate` before starting.

## Troubleshooting

| Problem | What to do |
|---|---|
| Grafana charts are empty | Start demo traffic on the presenter desk and wait a minute. |
| Grafana tiles say **Unknown** | Run `python3 ops/workload.py status`. Right after a start, wait for `ready`. |
| Everything stays Unknown for a long time | A deployment may have been left failed or throttled, so requests kept retrying and history grew very large. On the desk, **restore** every deployment, use **Reset scenario…**, then restart terminals 1 and 2. |
| Drasi Server takes minutes to start | It is reading accumulated history. Wait; it will finish. |
| "Run ops/observability-stack.sh env first" | Do [step 3](#3-create-local-passwords). |
| A port is already in use | Something is already running. Find it with `lsof -nP -iTCP:18421 -sTCP:LISTEN` (change the port) and stop it before starting another copy. |
| Startup refuses because files changed | Rebuild: `./ops/native-build.sh`. |
| Containers fail to start | Check Podman is running: `podman machine start`, then `./ops/observability-stack.sh ps`. |
| Drasi Server UI shows no query text | Known Drasi Server UI issue: it looks up queries in a way the current API no longer accepts. Until it is fixed, skip that drill-down or open the query file in `demo/drasi/`. |
| The query text in Drasi Server is too small to read | Zoom the browser (Cmd +). |

## For developers

To change or test the demo, see [DEVELOPERS.md](DEVELOPERS.md).
