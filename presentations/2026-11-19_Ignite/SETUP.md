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
| Presenter desk and projector window | The desk you present from, and the window it drives on the projector (slides, dashboards, recordings) | http://127.0.0.1:18580/presenter, and `/present` |
| Grafana | The two dashboards, shown inside the projector window | http://localhost:13000 |
| Langfuse | Traces of individual AI requests (not projected) | http://localhost:13001 |
| Drasi Server | Runs the continuous queries and models | http://127.0.0.1:18421 |
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

**Terminal 3: presenter desk and projector window**

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

Open the presenter desk at http://127.0.0.1:18580/presenter and follow
[PRESENTER.md](PRESENTER.md#set-up-your-screens): it covers the screens, the
checklist, the script, every control and what to do if something goes wrong.

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
| Everything stays Unknown for a long time | A deployment may have been left failed, so requests kept retrying and history grew very large. On the desk, open **All demo controls** and restore every deployment (**Any deployment** › Restore), then **Before and after the talk** › **Reset scenario…**, then restart terminals 1 and 2. |
| Drasi Server takes minutes to start | It is reading accumulated history. Wait; it will finish. |
| "Run ops/observability-stack.sh env first" | Do [step 3](#3-create-local-passwords). |
| A port is already in use | Something is already running. Find it with `lsof -nP -iTCP:18421 -sTCP:LISTEN` (change the port) and stop it before starting another copy. |
| Startup refuses because files changed | Rebuild: `./ops/native-build.sh`. |
| Containers fail to start | Check Podman is running: `podman machine start`, then `./ops/observability-stack.sh ps`. |
| The projector window shows "Grafana · …" but no dashboard | Grafana isn't running or hasn't loaded yet: check `./ops/observability-stack.sh ps`, then press **Back** and **Next** on the desk. |
| Desk header says "Projector window not opened from this desk" | Press **Open projector window** in the header, and use that window on the projector. |

## For developers

To change or test the demo, see [DEVELOPERS.md](DEVELOPERS.md).
