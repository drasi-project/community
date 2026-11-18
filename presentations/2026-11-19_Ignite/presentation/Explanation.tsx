import { useEffect, useState, type ReactNode } from "react";
import QRCode from "qrcode";
import { products, recaps, session, speaker, type SlideId } from "./scenes";
import { QuerySlide } from "./QuerySlide";
import "./slides.css";

// Keep deployment names such as EU-A on one line.
const nb = (text: string) => text.replace(/\b(EU|US)-([A-C])\b/g, "$1\u2011$2");

// The Ignite template's artwork, extracted on the presenting machine by ops/template-assets.py
// and served at /template/. Without it, slides keep the template's solid Dark Blue.
const template = (file: string) => `/template/${file}`;
const background = (file: string) => ({ backgroundImage: `url(${template(file)})` });

// The first of these images that loads, or the fallback if none does.
function Artwork({ files, alt = "", className = "artwork", fallback = null }: { files: string[]; alt?: string; className?: string; fallback?: ReactNode | ((file: string | null) => ReactNode) }) {
  const [at, setAt] = useState(0);
  const file = files[at] ?? null;
  const extra = typeof fallback === "function" ? fallback(file) : file ? null : fallback;
  return <>{file && <img className={className} src={template(file)} alt={alt} onError={() => setAt(at + 1)} />}{extra}</>;
}

// The Microsoft logo as the template places it (the template only has it as EMF).
const Logo = () => <div className="ms-logo" role="img" aria-label="Microsoft"><span aria-hidden="true"><i /><i /><i /><i /></span><b aria-hidden="true">Microsoft</b></div>;

const roles = [
  { name: "Sources", tone: "blue", what: "Bring in every change as it happens", here: "the AI service's database change feed" },
  { name: "Continuous queries", tone: "purple", what: "Keep a result current as data changes and as time passes", here: "overdue requests, deployment status, verified recovery" },
  { name: "Continuous models", tone: "green", what: "Run logic on query results, again on every change", here: "policy (Rego), planner and what-if (an optimizer)" },
  { name: "Reactions", tone: "lime", what: "Deliver each change to other systems", here: "Prometheus metrics, approved plans to the gateway, Grafana annotations, Langfuse scores" },
] as const;

function DrasiCode() {
  const [svg, setSvg] = useState("");
  useEffect(() => { void QRCode.toString(products.drasi, { type: "svg", margin: 0, color: { dark: "#091F2C", light: "#0000" } }).then(setSvg); }, []);
  return <div className="qr" role="img" aria-label="QR code for drasi.io" dangerouslySetInnerHTML={{ __html: svg }} />;
}

export function Explanation({ slide, active = false }: { slide: SlideId; active?: boolean }) {
  if (slide === "walkin") return <div className="walkin-slide">
    <Artwork files={["walkin.svg", "walkin.png"]} fallback={file => file === "walkin.svg" ? null
      : <><Logo /><p className="walkin-name">Microsoft<br />Ignite</p></>} />
  </div>;

  if (slide === "title") return <div className="title-slide" style={background("title-bg.jpg")}>
    <Logo />
    <div className="title-copy">
      <h1>Monitoring<br />without polling</h1>
      <p>Answers that update the moment things change, beside the monitoring you already use</p>
    </div>
    <div className="title-speaker">
      {speaker.name && <p className="name">{speaker.name}</p>}
      {speaker.role && <p>{speaker.role}</p>}
      <p className="session">{session.event} · {session.code}</p>
    </div>
  </div>;

  if (slide === "demo") return <div className="demo-slide" style={background("demo-bg.jpg")}>
    <h1>Demo</h1>
    <p>An incident, end to end, with real traffic</p>
  </div>;

  if (slide === "survey") return <div className="survey-slide" style={background("survey-bg.jpg")}>
    <div className="survey-copy">
      <h1>How did we do?</h1>
      <p>Tell us your thoughts about our sessions and complete the event survey</p>
    </div>
    <Artwork files={["survey-frame.svg"]} className="survey-frame" />
    <Artwork files={["survey-qr.png"]} className="survey-qr" alt="QR code for the event survey" />
  </div>;

  if (slide === "query") return <QuerySlide live={active} />;

  const recap = recaps.find(r => r.id === slide);
  if (recap) {
    const parts = [
      ["Sources", recap.sources], ["Continuous queries", "queries" in recap ? recap.queries : undefined],
      ["Continuous models", recap.models], ["Reactions", recap.reactions],
    ] as const;
    return <div className="content-slide recap-slide">
      <h1>What Drasi just did</h1>
      <div className="body">
        <p className="lead">{nb(recap.title)}</p>
        <ul className="recap-saw">{recap.saw.map(item => <li key={item}>{nb(item)}</li>)}</ul>
        <ol className="roles">
          {parts.map(([name, what], i) => <li key={name} className={what ? `active ${roles[i].tone}` : "idle"}>
            <b>{name}</b><span>{what ? nb(what) : "Not involved in this step"}</span>
          </li>)}
        </ol>
        <p className="callout"><b>Why it matters</b> {nb(recap.why)}</p>
      </div>
    </div>;
  }

  if (slide === "concepts") return <div className="content-slide concepts-slide">
    <h1>The scenario</h1>
    <div className="body">
      <p className="lead"><b>Keep customer support running.</b> Two AI workloads share the same limited model capacity, split across three deployments.</p>
      <ul className="deployments">
        <li className="blue"><b>{nb("EU-A")}</b><small>Europe</small><span>Customer support</span><em>60k of 100k tokens/min</em></li>
        <li className="purple"><b>{nb("EU-B")}</b><small>Europe</small><span>Document summaries</span><em>80k of 100k tokens/min</em></li>
        <li className="gray"><b>{nb("US-C")}</b><small>United States</small><span>Spare</span><em>0 of 100k tokens/min</em></li>
      </ul>
      <ul className="rules">
        <li><b>Customer support</b> uses restricted data: Europe only</li>
        <li><b>Document summaries</b> use public data: anywhere</li>
        <li>Customers expect an answer within <b>15 seconds</b></li>
        <li>Moving work needs a <b>person's approval</b></li>
      </ul>
      <div className="callout questions">
        <p>When {nb("EU-A")} fails, the business needs three answers:</p>
        <ol><li>Who is affected right now?</li><li>What's the fastest fix that stays within policy?</li><li>How do we know it's really fixed?</li></ol>
      </div>
      <p className="slide-meta">Simulated AI models and capacity · real requests, monitoring and Drasi</p>
    </div>
  </div>;

  if (slide === "drasi") return <div className="content-slide drasi-slide">
    <h1>Drasi in one picture</h1>
    <div className="body">
      <p className="lead"><b>Change-driven computation</b></p>
      <ol className="roles flow">{roles.map(role => <li key={role.name} className={`active ${role.tone}`}>
        <b>{role.name}</b><span>{role.what}</span><small>Here: {role.here}</small></li>)}</ol>
      <div className="ownership">
        <div><b>Built into Drasi</b><span>Sources, continuous queries kept current as data and time change, re-running models when their inputs change, and HTTP and streaming reactions</span></div>
        <div><b>Written for this demo, as Drasi plugins</b><span>A Rego policy model, an optimizer (planner and what-if) and a Prometheus reaction. The rules and settings are configuration</span></div>
      </div>
      <p className="callout"><b>"Without polling"</b> No answer is recomputed on a schedule: answers change when the facts or the time do. Health checks still run, and Prometheus and Grafana still collect and draw on their own schedule.</p>
    </div>
  </div>;

  return <div className="content-slide closing-slide">
    <h1>Answers that change when the facts do</h1>
    <div className="body">
      <div className="closing-copy">
        <p className="lead">Keep your monitoring tools. Add change-driven computation.</p>
        <ul>
          <li>Who is affected, the moment they're affected</li>
          <li>Plans that fit your rules and the current facts</li>
          <li>Proof a fix worked, not just that it was approved</li>
          <li>Safe from the next failure? Answered all the time</li>
        </ul>
      </div>
      <div className="closing-links">
        <DrasiCode />
        <p className="repo">drasi.io</p>
      </div>
    </div>
  </div>;
}
