import type { ReactNode } from "react";
import { Icon } from "./Icon";
import "./visuals.css";
import logo from "./assets/drasi-logo-colored-white-tight.svg";
import mark from "./assets/drasi-logomark-tight.svg";
// The CNCF logo, unmodified, from github.com/cncf/artwork (other/cncf/horizontal/white/cncf-white.svg).
import cncf from "./assets/cncf-white.svg";

// The Drasi project's own artwork (community/images), cropped to the artwork. The rule (visuals.css):
// the full logo only where Drasi is introduced or summed up; the logomark, at one size, where Drasi
// is a box or node in a diagram.
export const DrasiMark = () => <img className="drasi-mark" src={mark} alt="" />;

// The new flow's slides (NEW_FLOW.md): diagrams rather than lists. Rough-to-medium polish.
export const visualIds = ["project", "problem", "change-driven", "drasi-1", "drasi-2", "drasi-3", "plugins", "forms", "issues", "wants", "wants-drasi", "scenario-map", "closing", "links"] as const;
export type VisualId = typeof visualIds[number];
export const isVisual = (id: string): id is VisualId => (visualIds as readonly string[]).includes(id);

const nb = (text: string) => text.replace(/\b(EU|US)-([A-C])\b/g, "$1\u2011$2");
const Slide = ({ id, title, children, withLogo = false }: { id: string; title: string; children: ReactNode; withLogo?: boolean }) =>
  <div className={`content-slide v-slide vs-${id}`}><h1 className={title.length > 40 ? "long" : undefined}>{title}</h1>{withLogo && <img className="v-logo" src={logo} alt="Drasi" />}<div className="body">{children}</div></div>;
// Plugins in drasi-core's components folder, grouped.
const pluginSources: [string, string[]][] = [
  ["Databases", ["PostgreSQL", "MySQL", "SQL Server", "Oracle", "SQLite", "Neo4j", "Dataverse"]],
  ["Streams and APIs", ["Kafka", "HTTP", "gRPC", "WebSocket", "OpenTelemetry", "Redis Streams"]],
  ["Platforms", ["Kubernetes", "Your app, in-process"]],
  ["Live feeds", ["Cloudflare Radar", "GTFS-RT", "HERE Traffic", "Open511", "RIS Live", "Hyperliquid", "Sui DeepBook"]],
];
const pluginReactions: [string, string[]][] = [
  ["APIs and agents", ["HTTP", "gRPC", "SSE", "MCP", "A2A"]],
  ["Messaging", ["Event Grid", "AWS SQS", "RabbitMQ", "Azure Storage", "Redis Streams"]],
  ["Databases", ["Stored procedures: PostgreSQL, MySQL, SQL Server"]],
  ["Observe and build", ["Dashboard", "Loki", "Log", "Profiler", "File", "Your app, in-process"]],
];

// From drasi.io's navigation bar.
const social: [string, string][] = [["globe", "drasi.io"], ["github", "github.com/drasi-project"], ["discord", "aka.ms/drasidiscord"],
  ["youtube", "@DrasiProject"], ["bluesky", "drasi-project.bsky.social"], ["x", "@drasi_project"]];
const Arrow = () => <span className="v-arrow" aria-hidden="true" />;

// Sources → Continuous Queries → Reactions, growing over three slides.
type Node = { id: string; col: 0 | 1 | 2; row: number; label: string; step: number; relabel?: [number, string] };
const nodes: Node[] = [
  { id: "pg", col: 0, row: 1, label: "PostgreSQL", step: 1 },
  { id: "otel", col: 0, row: 0, label: "OpenTelemetry", step: 2 },
  { id: "k8s", col: 0, row: 2, label: "Kubernetes", step: 2 },
  { id: "hub", col: 0, row: 3, label: "Event Hubs", step: 3 },
  { id: "q1", col: 1, row: 1, label: "Late orders", step: 1, relabel: [2, "Late orders, unhealthy infra"] },
  { id: "q2", col: 1, row: 0, label: "Unhealthy deployments", step: 3 },
  { id: "q3", col: 1, row: 2, label: "Policy violations", step: 3 },
  { id: "q4", col: 1, row: 3, label: "Stock running low", step: 3 },
  { id: "r1", col: 2, row: 1, label: "Notify the team", step: 1 },
  { id: "r2", col: 2, row: 0, label: "Dashboard", step: 3 },
  { id: "r3", col: 2, row: 2, label: "Start a workflow", step: 3 },
  { id: "r4", col: 2, row: 3, label: "Update a cache", step: 3 },
];
const edges: [string, string, number][] = [["pg", "q1", 1], ["q1", "r1", 1], ["otel", "q1", 2], ["k8s", "q1", 2],
  ["otel", "q2", 3], ["k8s", "q2", 3], ["pg", "q3", 3], ["hub", "q4", 3], ["pg", "q4", 3], ["q2", "r2", 3], ["q1", "r2", 3], ["q3", "r3", 3], ["q4", "r4", 3], ["q3", "r1", 3]];
const colX = [0, 640, 1336], colW = [400, 456, 400], rowY = (row: number) => 96 + row * 150, nodeH = 104;
const tones = ["var(--c-source)", "var(--c-query)", "var(--c-reaction)"], modelTone = "var(--c-model)";
function Grow({ step, active }: { step: number; active: boolean }) {
  const at = (id: string) => nodes.find(n => n.id === id)!;
  const shown = nodes.filter(n => n.step <= step);
  return <svg className={`v-grow${active ? " play" : ""}`} viewBox="0 0 1736 650" width="1736" height="650" role="img" aria-label="Sources, continuous queries and reactions">
    {["Sources", "Continuous Queries", "Reactions"].map((h, i) =>
      <text key={h} x={colX[i] + colW[i] / 2} y="40" textAnchor="middle" className="v-grow-head" style={{ fill: tones[i] }}>{h}</text>)}
    {edges.filter(([, , s]) => s <= step).map(([a, b, s]) => {
      const from = at(a), to = at(b);
      const x1 = colX[from.col] + colW[from.col], y1 = rowY(from.row) + nodeH / 2, x2 = colX[to.col], y2 = rowY(to.row) + nodeH / 2;
      return <path key={a + b} className={s === step && step > 2 ? "new" : undefined} d={`M${x1},${y1} C${x1 + 120},${y1} ${x2 - 120},${y2} ${x2 - 2},${y2}`} markerEnd="url(#grow-head)" />;
    })}
    <defs><marker id="grow-head" viewBox="0 0 10 10" refX="10" refY="5" markerUnits="userSpaceOnUse" markerWidth="18" markerHeight="18" orient="auto"><path d="M0,0 L10,5 L0,10 z" fill="#D9D9D6" /></marker></defs>
    {shown.map(n => {
      const label = n.relabel && step >= n.relabel[0] ? n.relabel[1] : n.label;
      const fresh = n.step === step && step > 2;
      return <g key={n.id} className={fresh ? "new" : undefined}>
        <rect x={colX[n.col]} y={rowY(n.row)} width={colW[n.col]} height={nodeH} rx="10" style={{ fill: tones[n.col] }} />
        <text x={colX[n.col] + colW[n.col] / 2} y={rowY(n.row) + nodeH / 2 + 12} textAnchor="middle" className="v-grow-label">{label}</text>
      </g>;
    })}
  </svg>;
}

// Step 4: queries build on each other's results, as in this demo (simplified).
type GNode = { id: string; x: number; y: number; w: number; label: string; tone: string; fresh?: boolean };
const gnodes: GNode[] = [
  { id: "pg", x: 0, y: 110, w: 300, label: "PostgreSQL", tone: tones[0] },
  { id: "otel", x: 0, y: 420, w: 300, label: "OpenTelemetry", tone: tones[0] },
  { id: "pin", x: 380, y: 110, w: 300, label: "Policy input", tone: tones[1] },
  { id: "dh", x: 380, y: 420, w: 300, label: "Deployment health", tone: tones[1] },
  { id: "pol", x: 780, y: 110, w: 260, label: "Policy (Rego)", tone: modelTone, fresh: true },
  { id: "ws", x: 780, y: 530, w: 260, label: "Workload status", tone: tones[1], fresh: true },
  { id: "plan", x: 1080, y: 300, w: 300, label: "Plan (optimization)", tone: modelTone, fresh: true },
  { id: "chan", x: 1440, y: 110, w: 296, label: "Incident channel", tone: tones[2] },
  { id: "api", x: 1440, y: 300, w: 296, label: "Gateway API", tone: tones[2] },
  { id: "graf", x: 1440, y: 530, w: 296, label: "Grafana", tone: tones[2] },
];
const gedges: [string, string, boolean?][] = [["pg", "pin"], ["otel", "dh"], ["pin", "pol", true], ["pol", "plan", true], ["dh", "plan", true], ["dh", "ws", true],
  ["ws", "graf"], ["plan", "graf"], ["plan", "chan"], ["plan", "api"]];
function QueryGraph({ active }: { active: boolean }) {
  const at = (id: string) => gnodes.find(n => n.id === id)!;
  return <svg className={`v-grow${active ? " play" : ""}`} viewBox="0 0 1736 650" width="1736" height="650" role="img" aria-label="Continuous queries feeding other queries and models">
    {[["Sources", 150, tones[0]], ["Continuous Queries and Models", 860, tones[1]], ["Reactions", 1588, tones[2]]].map(([h, x, fill]) =>
      <text key={h as string} x={x as number} y="40" textAnchor="middle" className="v-grow-head" style={{ fill: fill as string }}>{h}</text>)}
    <defs><marker id="graph-head" viewBox="0 0 10 10" refX="10" refY="5" markerUnits="userSpaceOnUse" markerWidth="18" markerHeight="18" orient="auto"><path d="M0,0 L10,5 L0,10 z" fill="#D9D9D6" /></marker></defs>
    {gedges.map(([a, b, fresh]) => { const f = at(a), t = at(b);
      const x1 = f.x + f.w, y1 = f.y + nodeH / 2, x2 = t.x, y2 = t.y + nodeH / 2, bend = Math.min(90, (x2 - x1) / 2);
      return <path key={a + b} className={fresh ? "new" : undefined} d={`M${x1},${y1} C${x1 + bend},${y1} ${x2 - bend},${y2} ${x2 - 2},${y2}`} markerEnd="url(#graph-head)" />; })}
    {gnodes.map(n => <g key={n.id} className={n.fresh ? "new" : undefined}>
      <rect x={n.x} y={n.y} width={n.w} height={nodeH} rx="10" style={{ fill: n.tone }} />
      <text x={n.x + n.w / 2} y={n.y + nodeH / 2 + 12} textAnchor="middle" className="v-grow-label">{n.label}</text></g>)}
  </svg>;
}

// The usual polling loop: four boxes round a circle.
function Loop() {
  const steps: [number, number, string][] = [[380, 50, "Poll every few seconds"], [575, 230, "Query everything"], [380, 410, "Diff with last time"], [185, 230, "Code to react"]];
  return <svg className="v-loop" viewBox="0 0 760 460" width="760" height="460" aria-hidden="true">
    <circle cx="380" cy="230" r="180" />
    {steps.map(([x, y, t]) => <g key={t}><rect x={x - 180} y={y - 40} width="360" height="80" rx="8" /><text x={x} y={y + 12} textAnchor="middle">{t}</text></g>)}
  </svg>;
}

// Stream processing: the state and logic you build between the stream and the reaction.
function Stream() {
  const steps = ["Consume the event stream", "Keep state: windows, joins, stores", "Handle order and late events", "Code to react"];
  return <svg className="v-loop v-stream" viewBox="0 0 760 460" width="760" height="460" aria-hidden="true">
    {[0, 1, 2].map(i => <path key={i} d={`M380,${94 + i * 116} L380,${130 + i * 116}`} markerEnd="url(#stream-head)" />)}
    <defs><marker id="stream-head" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto"><path d="M0,0 L10,5 L0,10 z" /></marker></defs>
    {steps.map((t, i) => <g key={t}><rect x="40" y={14 + i * 116} width="680" height="80" rx="8" /><text x="380" y={66 + i * 116} textAnchor="middle">{t}</text></g>)}
  </svg>;
}

// The tangle: an on-call engineer between the tools an enterprise typically runs, grouped.
const toolGroups: { name: string; x: number; y: number; w: number; items: string[] }[] = [
  { name: "Telemetry", x: 0, y: 0, w: 450, items: ["Metrics", "Logs", "Traces", "Profiles", "APM", "RUM and synthetics"] },
  { name: "Response", x: 530, y: 0, w: 450, items: ["Alerts and on-call", "Incidents", "ITSM tickets", "ChatOps", "Runbooks"] },
  { name: "Change", x: 0, y: 440, w: 300, items: ["CI/CD and deploys", "Feature flags", "CMDB and catalog"] },
  { name: "AI and security", x: 320, y: 520, w: 320, items: ["LLM observability", "SIEM", "Vulnerabilities"] },
  { name: "Business and risk", x: 660, y: 440, w: 320, items: ["Business KPIs", "Cost (FinOps)", "Policy and compliance"] },
];
function Tangle() {
  const person = { x: 490, y: 340 };
  const centre = (g: typeof toolGroups[number]) => ({ x: g.x + g.w / 2, y: g.y + 90 });
  const cross: [number, number][] = [[0, 2], [0, 4], [1, 3], [1, 2], [2, 4], [3, 0], [4, 1]];
  return <div className="v-tangle" aria-hidden="true">
    <svg viewBox="0 0 980 720" width="980" height="720">
      {cross.map(([a, b]) => { const p = centre(toolGroups[a]), q = centre(toolGroups[b]); return <line key={`${a}-${b}`} className="thin" x1={p.x} y1={p.y} x2={q.x} y2={q.y} />; })}
      {toolGroups.map(g => { const c = centre(g); return <line key={g.name} x1={person.x} y1={person.y} x2={c.x} y2={c.y} />; })}
    </svg>
    {toolGroups.map(g => <div key={g.name} className="v-tools-group" style={{ left: g.x, top: g.y, width: g.w }}>
      <h3>{g.name}</h3><ul>{g.items.map(t => <li key={t}>{t}</li>)}</ul></div>)}
    <div className="who" style={{ left: person.x - 58, top: person.y - 58 }}>?</div>
  </div>;
}

const DeployBox = ({ name, region, detail, pending = false }: { name: string; region: string; detail: string; pending?: boolean }) =>
  <div className={`v-deploy${pending ? " pending-capacity" : ""}`}><b>{nb(name)}</b><span>{region} · 100k tokens/min</span><small>{detail}</small></div>;

export function Visual({ id, active, qr }: { id: VisualId; active: boolean; qr: ReactNode }) {
  switch (id) {
    case "project": return <div className="content-slide v-slide vs-project">
      <div className="v-hero"><h1><img src={logo} alt="Drasi" /></h1>
        <p className="lead">An open-source data processing platform for change-driven systems</p></div>
      <div className="v-project">
        <div className="v-badge"><img className="v-cncf" src={cncf} alt="Cloud Native Computing Foundation" /><div><b>Sandbox project</b><span>since January 2025</span></div></div>
        <ul className="v-tiles">
          <li className="blue"><Icon name="code" size={52} /><b>Open source</b><span>Apache 2.0, on GitHub</span></li>
          <li className="purple"><Icon name="bolt" size={52} /><b>Active development</b><span>New sources, reactions and form factors</span></li>
          <li className="green"><Icon name="user" size={52} /><b>Growing community</b><span>Users and contributors</span></li>
        </ul>
      </div>
      <ul className="v-social">{social.map(([icon, text]) => <li key={text}><Icon name={icon} size={36} />{text}</li>)}</ul>
    </div>;
    case "problem": return <Slide id={id} title="Why Drasi exists">
      <p className="lead">Detecting meaningful change as it happens, and reacting to it, is harder than it should be</p>
      <div className="v-versus">
        <div className="v-col bad"><h2><Icon name="poll" size={40} />Polling</h2><Loop /><p>Slow to notice, wasteful, and changes in between are missed</p></div>
        <div className="v-col bad"><h2><Icon name="otel" size={40} />Stream processing</h2><Stream /><p>Every event arrives; what it means for the current state is code you write and run</p></div>
      </div>
    </Slide>;
    case "change-driven": return <Slide id={id} title="Change-driven computation with Drasi" withLogo>
      <p className="lead">Describe the result you want; Drasi keeps it current</p>
      <ol className="v-chain">
        <li className="role-source"><Icon name="pg" size={64} /><b>Changes flow in</b><span>from the databases, telemetry and systems you already have</span></li>
        <li className="drasi role-query"><DrasiMark /><b>Drasi keeps the result current</b><span>as data changes, and as time passes, across sources</span></li>
        <li className="role-reaction"><Icon name="bolt" size={64} /><b>The reaction acts on the change</b><span>only what changed: added, updated or removed</span></li>
      </ol>
    </Slide>;
    case "drasi-3": return <Slide id={id} title="Simple concepts, powerful change-driven solutions">
      <p className="lead">Queries and models build on each other's results: a computation graph <span className="v-soon">Coming soon</span></p>
      <QueryGraph active={active} />
    </Slide>;
    case "drasi-1": case "drasi-2": {
      const step = Number(id.slice(-1)) + 1;
      return <Slide id={id} title="Simple concepts, powerful change-driven solutions">
        <p className="lead">{["", "Sources bring in every change; one query joins them; a reaction acts on the answer", "Many sources, queries and reactions; every answer kept current"][step - 1]}</p>
        <Grow step={step} active={active} />
      </Slide>;
    }
    case "plugins": return <Slide id={id} title="Everything is a plugin">
      <div className="v-hub">
        <div className="v-plug role-source">{pluginSources.map(([group, items]) => <div className="v-group" key={group}><h3>{group}</h3><ul>{items.map(t => <li key={t}>{t}</li>)}</ul></div>)}</div>
        <Arrow />
        <div className="v-core"><b><DrasiMark />Drasi</b>
          <span className="role-query">Continuous Queries<em className="v-langs-q"><i>openCypher</i><i>GQL</i></em></span>
          <span className="role-model">Continuous Models<em className="v-models">{["Policy", "Optimization", "View", "Arithmetic", "Geometry"].map(m => <i key={m}>{m}</i>)}</em></span></div>
        <Arrow />
        <div className="v-plug role-reaction">{pluginReactions.map(([group, items]) => <div className="v-group" key={group}><h3>{group}</h3><ul>{items.map(t => <li key={t}>{t}</li>)}</ul></div>)}</div>
      </div>
    </Slide>;
    case "forms": return <Slide id={id} title="Run Drasi your way">
      <div className="v-forms">
        <div className="v-col gray"><h2>In your app</h2>
          <div className="v-sketch"><div className="v-box app"><small>Your app</small><div className="v-box lib"><DrasiMark />Drasi library</div></div></div>
          <ul className="v-langs">{["Rust", "Node.js", "Python", ".NET"].map(l => <li key={l}>{l}</li>)}</ul>
          <p>Build change-driven applications</p></div>
        <div className="v-col gray"><h2>As a service</h2>
          <div className="v-sketch row"><div className="v-box src">Data</div><Arrow /><div className="v-box lib"><DrasiMark />Drasi Server</div><Arrow /><div className="v-box src">Tools</div></div>
          <ul className="v-langs"><li>Process</li><li>Container</li></ul>
          <p>Configure it in YAML</p></div>
        <div className="v-col gray"><h2>On Kubernetes</h2>
          <div className="v-sketch"><div className="v-box cluster"><small><DrasiMark /> Drasi on a cluster</small><div className="v-pods"><span>Sources</span><span>Queries</span><span>Reactions</span></div></div></div>
          <ul className="v-langs"><li>drasi CLI</li><li>Resources</li></ul>
          <p>Each part scales on its own</p></div>
      </div>
    </Slide>;
    case "issues": return <Slide id={id} title="Observability today">
      <div className="v-split"><Tangle />
        <ul className="v-issues">
          <li><b>Too much data</b><span>and the cost of keeping it</span></li>
          <li><b>Too many tools</b><span>one request, many screens</span></li>
          <li><b>Too much noise</b><span>alerts that don't matter</span></li>
          <li><b>Symptoms, not answers</b><span>who's affected? what to do?</span></li>
          <li><b>Context lives elsewhere</b><span>policy, owners, business data</span></li>
        </ul></div>
    </Slide>;
    case "wants": case "wants-drasi": return <Slide id={id} title="What organizations want">
      <div className="v-clear">
        <ul className="v-inputs">{["Telemetry", "Application data", "Policy", "Changes and approvals"].map(t => <li key={t}>{t}</li>)}</ul>
        <Arrow />
        <div className={`v-context${id === "wants-drasi" ? " with-drasi" : ""}`}>{id === "wants-drasi" && <DrasiMark />}<b>{id === "wants-drasi" ? "Change-driven context" : "Context"}</b><span>connected and updated as things change</span></div>
        <Arrow />
        <ul className="v-answers">{["Who's affected?", "What changed?", "What should we do?", "Did it work?"].map(t => <li key={t}><Icon name="check" size={36} />{t}</li>)}</ul>
      </div>
      <ul className="v-wants">
        <li>Situations, not signals</li><li>Outcomes, including what didn't happen</li><li>Action backed by evidence, with approval</li><li>In the tools they already use</li>
      </ul>
    </Slide>;
    case "scenario-map": return <Slide id={id} title="The scenario: an AI service">
      <div className="v-scenario">
        <div className="v-work"><b>Customer support</b><span>Restricted data · answer in 15 s · 60k</span></div>
        <div className="v-work"><b>Document summaries</b><span>Public data · 80k</span></div>
        <div className="v-work empty" />
        <span className="v-down" /><span className="v-down" /><span />
        <div className="v-region eu"><small>Europe</small>
          <DeployBox name="EU-A" region="Europe" detail="serves support" /><DeployBox name="EU-B" region="Europe" detail="serves summaries" /></div>
        <div className="v-region us"><small>United States</small><DeployBox name="US-C" region="US" detail="extra capacity, just provisioned" pending /></div>
      </div>
      <div className="v-rules">
        <span className="rule"><Icon name="model" size={32} />Restricted data stays in Europe</span>
        <span className="rule"><Icon name="model" size={32} />Summaries: Europe, for now</span>
        <span className="rule"><Icon name="user" size={32} />Workload deployment changes need human approval</span>
      </div>
      <div className="v-stack">
        <p>In this demo</p>
        <ul className="v-tools">{[["otel", "OpenTelemetry"], ["prometheus", "Prometheus"], ["prometheus", "Grafana"], ["query", "Langfuse"], ["mattermost", "Mattermost"], ["pg", "PostgreSQL"]].map(([i, t]) => <li key={t}><Icon name={i} size={40} />{t}</li>)}</ul>
        <p className="other">Also common, not used here</p>
        <ul className="v-others">{["Datadog", "Dynatrace", "Splunk", "New Relic", "Elastic", "Azure Monitor", "ServiceNow", "PagerDuty", "Slack", "Teams", "Jira"].map(t => <li key={t}>{t}</li>)}</ul>
      </div>
    </Slide>;
    case "closing": return <Slide id={id} title="What you saw" withLogo>
      <ol className="v-scenes">
        <li><small>1</small><b>Status from telemetry + configuration</b><span>OpenTelemetry and the app's own data, joined</span></li>
        <li><small>2</small><b>What didn't happen</b><span>Requests with no answer, at their deadline</span></li>
        <li><small>3</small><b>Compliance meets resilience</b><span>One policy change; readiness re-checked</span></li>
        <li><small>4</small><b>A fix people approve</b><span>Proposed, approved in chat, verified</span></li>
      </ol>
      <ol className="v-ribbon"><li className="role-source">Connect sources</li><li className="role-query">Configure continuous models</li><li className="role-reaction">Connect reactions</li></ol>
      <p className="v-foot">Richer context, in the tools you already use. Apart from one Prometheus reaction plugin, this demo is configuration.</p>
    </Slide>;
    case "links": return <Slide id={id} title="Learn more" withLogo>
      <div className="v-links">
        <div className="v-qr">{qr}<b>drasi.io</b></div>
        <ul className="v-tiles links">
          <li className="blue"><Icon name="query" size={48} /><b>Docs and getting started</b><span>drasi.io</span></li>
          <li className="purple"><Icon name="code" size={48} /><b>Code</b><span>github.com/drasi-project</span></li>
          <li className="green"><Icon name="app" size={48} /><b>This demo</b><span className="todo">TODO: link</span></li>
          <li className="lime"><Icon name="discord" size={48} /><b>Community</b><span>Discord: aka.ms/drasidiscord · YouTube: @DrasiProject</span></li>
        </ul>
      </div>
    </Slide>;
  }
}
