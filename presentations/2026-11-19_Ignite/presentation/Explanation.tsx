import { products, type SlideId } from "./scenes";

export function Explanation({ slide }: { slide: SlideId }) {
  if (slide === "title") return <div className="title-slide">
    <div className="slide-brand">drasi <span>Microsoft Ignite 2026</span></div>
    <div className="title-copy">
      <h1>Monitoring<br />without polling</h1>
      <p>Real-time, change-driven monitoring</p>
    </div>
    <p className="slide-meta">THR1029 · November 19, 2026</p>
  </div>;

  if (slide === "concepts") return <div className="concepts-slide">
    <span className="eyebrow">The scenario</span>
    <h1>Keep customer support running</h1>
    <p className="scenario-lead">An AI assistant answers customer-support questions. A background job summarizes documents. Both draw on the same limited AI model capacity, deployed in three regions, and support data must stay in Europe.</p>
    <div className="scenario-body">
      <div className="scenario-capacity">
      <p className="scenario-caption">AI model capacity by region · <b>300k tokens/min total · 140k in use</b></p>
      <div className="scenario-endpoints">
        <div><b>Customer support</b><span>EU-A <small>Europe</small></span><em>60k of 100k tokens/min</em></div>
        <div><b>Document summaries</b><span>EU-B <small>Europe</small></span><em>80k of 100k tokens/min</em></div>
        <div><b>Spare capacity</b><span>US-C <small>United States</small></span><em>0 of 100k tokens/min</em></div>
      </div>
      </div>
      <ul className="scenario-rules">
        <li><b>Customer support</b> uses restricted data: Europe only</li>
        <li><b>Document summaries</b> use public data: run anywhere</li>
        <li>Customers expect an answer within 15 seconds</li>
        <li>Moving work needs a human's approval</li>
      </ul>
    </div>
    <div className="scenario-question">
      <p><b>When EU-A fails, customers stop getting answers.</b></p>
      <ol><li>Who is affected right now?</li><li>What's the fastest fix that stays within company policy?</li><li>How do we know it's really fixed?</li></ol>
    </div>
    <section className="drasi-strip" aria-label="Drasi">
      <p className="drasi-strip-name"><b>Drasi</b> · change-driven computation answers those questions as they change</p>
      <div className="drasi-roles">
        <div><b>Sources</b><span>bring in changes: requests, deployment reports, policy</span></div>
        <i aria-hidden="true">→</i>
        <div><b>Continuous models</b><span>keep answers current as data and time change</span><small>continuous queries · continuous policy · continuous monitors</small></div>
        <i aria-hidden="true">→</i>
        <div><b>Reactions</b><span>publish answers to Grafana and Langfuse, and act on approval</span></div>
      </div>
    </section>
    <p className="slide-meta">Simulated AI models and capacity · Real requests, telemetry and Drasi computation</p>
  </div>;

  return <div className="closing-slide">
    <div className="slide-brand">drasi</div>
    <div className="closing-copy">
      <h1>From change to action</h1>
      <p className="closing-lead">Keep your observability tools.<br />Add change-driven computation.</p>
      <ul><li>Connect changing context</li><li>Maintain valid decisions</li><li>Verify the outcome</li></ul>
    </div>
    <a className="slide-resource" href={products.repository} target="_blank" rel="noopener noreferrer">github.com/drasi-project/community</a>
  </div>;
}
