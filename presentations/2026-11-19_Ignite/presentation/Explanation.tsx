import { useEffect, useState, type ReactNode } from "react";
import QRCode from "qrcode";
import { products, session, speaker, type SlideId } from "./scenes";
import "./slides.css";
import { Visual, type VisualId } from "./Visuals";

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


  // The new flow's slides (Visuals.tsx); the drasi.io code sits on Learn more.
  return <Visual id={slide as VisualId} active={active} qr={<DrasiCode />} />;
}
