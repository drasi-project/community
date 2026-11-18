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
            <h2>Policy</h2>
            <p><b className="blue">Customer support</b>: restricted data, <strong>Europe only</strong></p>
            <p><b className="purple">Document summaries</b>: public data, <strong>anywhere</strong></p>
          </section>
          <section>
            <h2>Business requirements</h2>
            <p>Customers get an answer within <strong>15 seconds</strong></p>
            <p>Moving work needs <strong>a person's approval</strong></p>
          </section>
        </div>
        <div className={`callout questions ${shown(3) ?? ""}`} aria-hidden={step < 3}>
          <p>When something goes wrong, the business needs three answers:</p>
          <ol><li>Who is affected right now?</li><li>What's the fastest fix that stays within policy?</li><li>How do we know it's really fixed?</li></ol>
        </div>
      </div>
    </div>;
  }

  // What happens when EU-A fails: the demo's setup, simplified, and where Drasi sits beside existing observability.
  if (slide === "setup") return <div className="content-slide setup-slide">
    <h1>What happens when {nb("EU-A")} Fails</h1>
    <div className="body">
      <p className="lead">Imagine your observability setup looks something like this. We'll see how Drasi works with it to respond to events intelligently, and sooner.</p>
      <svg className="setup-diagram" viewBox="0 0 1736 620" role="img" aria-label="The AI service's deployments in Europe and the United States, the existing monitoring and incident channel, and Drasi beside them">
        <defs><marker id="setup-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" className="arrowhead" /></marker></defs>
        <rect className="group" x="0" y="0" width="740" height="620" />
        <text className="group-label" x="28" y="48">AI SERVICE</text>
        <rect className="box" x="28" y="76" width="684" height="84" />
        <text className="box-name" x="56" y="129">Gateway <tspan className="box-note">routes each customer request</tspan></text>
        <rect className="region" x="28" y="200" width="446" height="290" />
        <text className="region-label" x="52" y="240">Europe</text>
        <rect className="deployment blue" x="52" y="262" width="193" height="200" />
        <text className="dep-name" x="74" y="318">{nb("EU-A")}</text>
        <text className="dep-work blue" x="74" y="362">Customer</text><text className="dep-work blue" x="74" y="396">support</text>
        <rect className="deployment purple" x="257" y="262" width="193" height="200" />
        <text className="dep-name" x="279" y="318">{nb("EU-B")}</text>
        <text className="dep-work purple" x="279" y="362">Document</text><text className="dep-work purple" x="279" y="396">summaries</text>
        <rect className="region" x="498" y="200" width="214" height="290" />
        <text className="region-label" x="520" y="240">United States</text>
        <rect className="deployment gray" x="520" y="262" width="170" height="200" />
        <text className="dep-name" x="542" y="318">{nb("US-C")}</text>
        <text className="dep-work gray" x="542" y="362">Spare</text>
        <rect className="box" x="28" y="514" width="684" height="84" />
        <text className="box-name" x="56" y="567">Database <tspan className="box-note">every request, health report, policy change</tspan></text>

        <rect className="group" x="960" y="0" width="776" height="300" />
        <text className="group-label" x="988" y="48">YOUR OBSERVABILITY</text>
        <rect className="box" x="988" y="76" width="344" height="196" />
        <text className="box-name" x="1016" y="130">Prometheus</text><text className="box-name" x="1016" y="170">and Grafana</text>
        <text className="box-note" x="1016" y="220">Metrics, dashboards, alerts</text>
        <rect className="box" x="1356" y="76" width="352" height="196" />
        <text className="box-name" x="1384" y="130">Incident</text><text className="box-name" x="1384" y="170">channel</text>
        <text className="box-note" x="1384" y="220">Where the team responds</text>

        <rect className="drasi" x="960" y="380" width="776" height="218" />
        <text className="drasi-name" x="992" y="446">Drasi</text>
        <text className="drasi-note" x="992" y="500">Reacts to every change and keeps the</text>
        <text className="drasi-note" x="992" y="540">answers current, where the team looks</text>

        <path className="flow" d="M712,118 H982" /><text className="edge" x="760" y="100">Metrics, traces</text>
        <path className="flow" d="M712,556 H954" /><text className="edge" x="760" y="538">Every change</text>
        <path className="flow answer" d="M1160,380 V278" /><text className="edge answer" x="1174" y="342">Answers</text>
        <path className="flow answer" d="M1420,380 V278" /><text className="edge answer" x="1434" y="342">Who's affected, what to do</text>
        <path className="flow" d="M370,160 V198" />
      </svg>
    </div>
  </div>;

  if (slide === "drasi") return <div className="content-slide drasi-slide">
    <h1>Drasi – change-driven computation</h1>
    <div className="body">
      <ol className="drasi-parts">
        <li className="part blue"><b>Sources</b></li>
        <li className="part models"><b>Continuous models</b>
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
