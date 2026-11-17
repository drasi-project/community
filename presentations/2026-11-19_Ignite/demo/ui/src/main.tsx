import { lazy, Suspense, useState } from "react";
import { createRoot } from "react-dom/client";
import { Audience } from "../../../presentation/Audience";
import { useStage } from "./stage";
import { useClock, useLive } from "./live";
import { recapFromHistory, type View } from "../../../presentation/scenes";
import { z } from "zod";
import { request, rowSchema } from "./contracts";
import "./style.css";

const Presenter = lazy(() => import("../../../presentation/Presenter"));
const Inspector = lazy(() => import("./views").then(module => ({ default: module.Demo })));
const preview = new URLSearchParams(location.search).has("preview");
function Slides() {
  const control = useStage(preview);
  return <Audience control={control} preview={preview} />;
}
function Desk() {
  const live = useLive(), now = useClock(), control = useStage(false, true);
  return <Presenter live={live} now={now} control={control} />;
}
function Diagnostics() {
  const live = useLive(), now = useClock();
  const [view, setView] = useState<View>("change");
  return <div className="private-inspector"><header className="app-header">PRIVATE DIAGNOSTICS · NOT THE PRESENTATION <a href="/presenter">Presenter desk</a></header><Inspector live={live} now={now} view={view} setView={setView} selection={null} /></div>;
}
function App() {
  if (!preview && location.pathname === "/presenter") return <Desk />;
  if (!preview && location.pathname === "/inspect") return <Diagnostics />;
  return <Slides />;
}
// "?slide=recap" shows the "What Drasi did" visual for how far the live demo has got
// (for a projected window navigated by hand). If the demo can't be reached, it shows the title.
async function resolveDrasiSlide() {
  const params = new URLSearchParams(location.search);
  if (params.get("slide") !== "recap") return;
  try {
    const { audit } = z.object({ audit: z.array(rowSchema) })
      .parse(await request("/api/decisions", { signal: AbortSignal.timeout(2000) }));
    params.set("slide", recapFromHistory(audit).id);
    history.replaceState(null, "", `${location.pathname}?${params}`);
  } catch { /* fall back */ }
}
void resolveDrasiSlide().then(() =>
  createRoot(document.getElementById("root")!).render(<Suspense fallback={<p>Loading...</p>}><App /></Suspense>));
