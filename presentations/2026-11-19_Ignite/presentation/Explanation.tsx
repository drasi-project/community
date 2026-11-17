import { useEffect, useState } from "react";
import QRCode from "qrcode";
import { products, recaps, speaker, type SlideId } from "./scenes";
import { QuerySlide } from "./QuerySlide";
import "./slides.css";

// Keep deployment names such as EU-A on one line.
const nb = (text: string) => text.replace(/\b(EU|US)-([A-C])\b/g, "$1\u2011$2");

const roles = [
  { name: "Sources", what: "Bring in every change as it happens", here: "the AI service's database change feed" },
  { name: "Continuous queries", what: "Keep a result current as data changes and as time passes", here: "overdue requests, deployment status, verified recovery" },
  { name: "Continuous models", what: "Run logic on query results, again on every change", here: "policy (Rego), planner and what-if (an optimizer)" },
  { name: "Reactions", what: "Deliver each change to other systems", here: "Prometheus metrics, approved plans to the gateway, Grafana annotations, Langfuse scores" },
] as const;

function Flow({ compact = false }: { compact?: boolean }) {
  return <ol className={`flow ${compact ? "compact" : ""}`}>
    {roles.map(role => <li key={role.name}><b>{role.name}</b>{!compact && <><span>{role.what}</span><small>Here: {role.here}</small></>}</li>)}
  </ol>;
}

function RepositoryCode() {
  const [svg, setSvg] = useState("");
  useEffect(() => { void QRCode.toString(products.repository, { type: "svg", margin: 0, color: { dark: "#e8eef5", light: "#0000" } }).then(setSvg); }, []);
  return <div className="qr" role="img" aria-label="QR code for the repository" dangerouslySetInnerHTML={{ __html: svg }} />;
}

export function Explanation({ slide, active = false }: { slide: SlideId; active?: boolean }) {
  if (slide === "title") return <div className="title-slide">
    <div className="slide-brand">drasi <span>Microsoft Ignite 2026 · THR1029</span></div>
    <div className="title-body">
      <div className="title-copy">
        <h1>Monitoring<br />without polling</h1>
        <p>Answers that update the moment things change, beside the monitoring you already use</p>
        {speaker.name && <p className="speaker"><b>{speaker.name}</b>{speaker.role && <> · {speaker.role}</>}</p>}
      </div>
      <Flow compact />
    </div>
  </div>;

  if (slide === "query") return <QuerySlide live={active} />;

  const recap = recaps.find(r => r.id === slide);
  if (recap) {
    const parts = [
      ["Sources", recap.sources], ["Continuous queries", "queries" in recap ? recap.queries : undefined],
      ["Continuous models", recap.models], ["Reactions", recap.reactions],
    ] as const;
    return <div className="recap-slide">
      <span className="eyebrow">What Drasi just did</span>
      <h1>{nb(recap.title)}</h1>
      <ul className="recap-saw">{recap.saw.map(item => <li key={item}>{nb(item)}</li>)}</ul>
      <ol className="recap-roles">
        {parts.map(([name, what]) => <li key={name} className={what ? "active" : "idle"}>
          <b>{name}</b><span>{what ? nb(what) : "Not involved in this step"}</span>
        </li>)}
      </ol>
      <p className="why"><b>Why it matters</b> {nb(recap.why)}</p>
    </div>;
  }

  if (slide === "concepts") return <div className="concepts-slide">
    <span className="eyebrow">The scenario</span>
    <h1>Keep customer support running</h1>
    <p className="lead">Two AI workloads share the same limited model capacity, split across three deployments.</p>
    <ul className="deployments">
      <li><b>{nb("EU-A")}</b><small>Europe</small><span>Customer support</span><em>60k of 100k tokens/min</em></li>
      <li><b>{nb("EU-B")}</b><small>Europe</small><span>Document summaries</span><em>80k of 100k tokens/min</em></li>
      <li><b>{nb("US-C")}</b><small>United States</small><span>Spare</span><em>0 of 100k tokens/min</em></li>
    </ul>
    <ul className="rules">
      <li><b>Customer support</b> uses restricted data: Europe only</li>
      <li><b>Document summaries</b> use public data: anywhere</li>
      <li>Customers expect an answer within <b>15 seconds</b></li>
      <li>Moving work needs a <b>person's approval</b></li>
    </ul>
    <div className="questions">
      <p>When {nb("EU-A")} fails, the business needs three answers:</p>
      <ol><li>Who is affected right now?</li><li>What's the fastest fix that stays within policy?</li><li>How do we know it's really fixed?</li></ol>
    </div>
    <p className="slide-meta">Simulated AI models and capacity · real requests, monitoring and Drasi</p>
  </div>;

  if (slide === "drasi") return <div className="drasi-slide">
    <span className="eyebrow">Drasi in one picture</span>
    <h1>Change-driven computation</h1>
    <Flow />
    <div className="ownership">
      <div><b>Built into Drasi</b><span>Sources, continuous queries kept current as data and time change, re-running models when their inputs change, and HTTP and streaming reactions</span></div>
      <div><b>Written for this demo, as Drasi plugins</b><span>A Rego policy model, an optimizer (planner and what-if) and a Prometheus reaction. The rules and settings are configuration</span></div>
    </div>
    <p className="polling"><b>"Without polling"</b> No answer is recomputed on a schedule: answers change when the facts or the time do. Health checks still run, and Prometheus and Grafana still collect and draw on their own schedule.</p>
  </div>;

  return <div className="closing-slide">
    <div className="slide-brand">drasi</div>
    <div className="closing-body">
      <div className="closing-copy">
        <h1>Answers that change when the facts do</h1>
        <p className="lead">Keep your monitoring tools. Add change-driven computation.</p>
        <ul>
          <li>Who is affected, the moment they're affected</li>
          <li>Plans that fit your rules and the current facts</li>
          <li>Proof a fix worked, not just that it was approved</li>
          <li>Safe from the next failure? Answered all the time</li>
        </ul>
      </div>
      <div className="closing-links">
        <RepositoryCode />
        <p className="repo">{products.repositoryLabel}</p>
        <p>drasi.io</p>
      </div>
    </div>
  </div>;
}
