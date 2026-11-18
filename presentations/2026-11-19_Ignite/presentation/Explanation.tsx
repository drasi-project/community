import { useEffect, useState, type ReactNode } from "react";
import QRCode from "qrcode";
import { products, recaps, session, speaker, type SlideId } from "./scenes";
import { QuerySlide } from "./QuerySlide";
import "./slides.css";

// Keep deployment names such as EU-A, and the incident channel's name, on one line.
const nb = (text: string) => text.replace(/\b(EU|US)-([A-C])\b/g, "$1\u2011$2")
  .replace(/#incident-ai-support/g, "#incident\u2011ai\u2011support");

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

// Colours of Drasi's parts, the same on the Drasi slide, the recaps and the diagrams.
const roles = [{ tone: "blue" }, { tone: "purple" }, { tone: "green" }, { tone: "lime" }] as const;

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
      ["Continuous Models", recap.models], ["Reactions", recap.reactions],
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

  // The scenario's three sections appear one desk beat at a time; hidden ones keep their space.
  if (slide.startsWith("concepts")) {
    const step = slide === "concepts-questions" ? 3 : slide === "concepts-rules" ? 2 : 1;
    const shown = (n: number) => step >= n ? undefined : "pending";
    return <div className="content-slide concepts-slide">
      <h1 className="long">The Scenario – Keep European Customer Support Running</h1>
      <div className="body">
        <p className="lead">Two AI workloads share the same limited model capacity, split across three deployments.</p>
        <ul className="deployments">
          <li className="blue"><span>Customer support</span><b>Runs on {nb("EU-A")}</b><small>Europe</small><em>60k of 100k tokens/min</em></li>
          <li className="purple"><span>Document summaries</span><b>Runs on {nb("EU-B")}</b><small>Europe</small><em>80k of 100k tokens/min</em></li>
          <li className="gray"><span>Spare capacity</span><b>{nb("US-C")}</b><small>United States</small><em>0 of 100k tokens/min</em></li>
        </ul>
        <div className={`requirements ${shown(2) ?? ""}`} aria-hidden={step < 2}>
          <section>
            <h2>Data Compliance</h2>
            <p><b className="blue">Customer support</b>: restricted data, <strong>Europe only</strong></p>
            <p><b className="purple">Document summaries</b>: public data, <strong>anywhere</strong></p>
          </section>
          <section>
            <h2>Business requirements</h2>
            <p>Customers get an answer within <strong>15 seconds</strong></p>
            <p>Infrastructure changes require <strong>human approval</strong></p>
          </section>
        </div>
        <div className={`callout questions ${shown(3) ?? ""}`} aria-hidden={step < 3}>
          <p>When something goes wrong, the business needs three answers:</p>
          <ol><li>Who is affected?</li><li>What's the fastest fix that stays within policy?</li><li>How do we know when it's resolved?</li></ol>
        </div>
      </div>
    </div>;
  }

  if (slide === "ecosystem") return <div className="content-slide ecosystem-slide">
    <h1>Observability today</h1>
    <div className="body">
      <p className="lead">Lots of inputs, lots of tools, and the context you need is spread across them.</p>
      <div className="eco">
        <ul className="eco-inputs">
          {["Metrics", "Logs", "Traces", "Health checks", "Application data", "Business data", "Policy and compliance", "Change approvals", "Incidents and tickets"]
            .map(item => <li key={item}>{item}</li>)}
        </ul>
        <div className="eco-arrow" aria-hidden="true">→</div>
        <ol className="eco-questions">
          <li><b>What's going on?</b><span>Who and what is affected, right now</span></li>
          <li><b>What should we do?</b><span>An action that fits the rules and the current facts</span></li>
          <li><b>What will it cause?</b><span>The consequences, and whether it worked</span></li>
        </ol>
      </div>
      <p className="callout"><b>The opportunity</b> Bring data from many systems together, and the context answers these questions for you.</p>
    </div>
  </div>;

  if (slide === "scenario") return <div className="section-slide">
    <h1>Let's look at a typical scenario</h1>
    <p>Two AI workloads, limited capacity, and the rules they must follow</p>
  </div>;

  // Bringing it together when EU-A fails: what Drasi joins, and the tools it works with.
  if (slide === "setup") return <div className="content-slide setup-slide">
    <h1>Bringing it together when {nb("EU-A")} fails</h1>
    <div className="body">
      <p className="lead">Drasi joins telemetry, application data and policy, and works with the tools the team already uses.</p>
      <div className="together">
        <div className="together-col">
          <h2>Brings together</h2>
          <div className="t-card blue"><b>Telemetry</b><span>Health probes as OpenTelemetry metrics, through the Collector</span></div>
          <div className="t-card purple"><b>Application data</b><span>Requests, deployments, routes and capacity, from the database's change feed</span></div>
          <div className="t-card green"><b>Policy and approvals</b><span>Data compliance rules, allowed destinations and change approvals</span></div>
        </div>
        <div className="together-arrow" aria-hidden="true">→</div>
        <div className="together-drasi">
          <b>Drasi</b>
          <span>Sources</span><span>Continuous Models</span><span>Reactions</span>
          <p>Answers kept current as the facts change, and as time passes</p>
        </div>
        <div className="together-arrow" aria-hidden="true">→</div>
        <div className="together-col">
          <h2>Works with</h2>
          <div className="t-card lime"><b>Grafana</b><span>Decision Support tiles beside the existing dashboards and alerts</span></div>
          <div className="t-card lime"><b>Messaging</b><span>Cards in the team's incident channel, in Mattermost</span></div>
          <div className="t-card lime"><b>Change management</b><span>The change-approval tool and the gateway, where e.g. a ticketing system would sit</span></div>
        </div>
      </div>
    </div>
  </div>;

  if (slide === "drasi") return <div className="content-slide drasi-slide">
    <h1>Drasi – change-driven computation</h1>
    <div className="body">
      <p className="lead">Simpler and quicker: bring the data together, and keep the answers current.</p>
      <ol className="drasi-parts">
        <li className="part blue"><b>Sources</b></li>
        <li className="part models"><b>Continuous Models</b>
          <ul><li className="purple">Query</li><li className="green">Policy</li><li className="green">Optimization</li></ul></li>
        <li className="part lime"><b>Reactions</b></li>
      </ol>
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
