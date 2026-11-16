**There are 2 chats named 'Review articles, blog posts, product' that contain responses to the same question from different LLM models. The question relates to the current top issues and future trends in the observability domain in large enterprises. There are many common topics across the 2 responses but also some differences. I want you to consolidate the 2 responses into a single response and also reduce the size so the key issues and directions are easier to review. In the consolidate response, i dont need the survey evidence quoted, just give me a simple list of issues and future directions in concise, plain english.**

## Current issues

1. **Fragmented visibility.** Too many disconnected tools make it hard to follow a customer request across applications, infrastructure, and external services.
2. **Unpredictable costs.** Collecting and storing monitoring data is expensive; cutting it indiscriminately creates blind spots.
3. **Too much operational work.** Maintaining monitoring platforms and handling recurring incidents consumes engineering time that should go toward improvements.
4. **Noisy alerts and late detection.** Teams receive too many low-value alerts yet still learn about important failures from customers.
5. **Slow root-cause diagnosis.** Seeing symptoms is easier than understanding which dependency, deployment, or configuration change caused them.
6. **Missing or misleading data.** Gaps in coverage, inconsistent naming, and lost context make investigations unreliable—even when plenty of data exists.
7. **Fragmented ownership and expertise.** Cross-team incidents stall because responsibilities, standards, and operational knowledge are inconsistent or concentrated in a few people.
8. **Weak connection to business impact.** Healthy components do not necessarily mean successful customer experiences; teams struggle to quantify impact and prioritize investment.
9. **Privacy and governance risks.** Monitoring data can expose sensitive information and must meet access, retention, residency, and audit requirements.
10. **New risks from AI.** AI applications can produce incorrect results or unsafe actions while appearing healthy, and AI-generated code increases the pace of changes needing scrutiny.

## Future directions

1. **Open standards and easier instrumentation.** Use OpenTelemetry, consistent data definitions, automatic collection, and code-level profiling to improve coverage and reduce vendor dependence.
2. **Connected operational context.** Link monitoring data to dependencies, recent changes, code, owners, previous incidents, and customer impact—not just more dashboards.
3. **Smarter data economics.** Control collection, sampling, retention, and storage costs while preserving useful evidence; query across data stores without necessarily centralizing everything.
4. **AI-assisted operations with safeguards.** Start with evidence-backed investigation and recommendations; expand to limited actions with permissions, approvals, audit trails, and recovery checks.
5. **Monitoring AI behavior, not just uptime.** Track output quality, agent decisions, tool use, safety, cost, and whether the intended task actually succeeds.
6. **Continuous feedback into development.** Use production evidence to improve code, assess releases, catch regressions early, and prevent repeat incidents—including those involving AI-generated code.
7. **Customer-focused reliability.** Measure completed customer journeys and business outcomes rather than relying only on individual service health.
8. **Shared platforms and clearer responsibilities.** Central platform teams provide common capabilities while service teams retain ownership; operations, security, and business teams share evidence with appropriate access controls.
9. **Monitoring that survives outages.** Ensure visibility remains available when the cloud region, provider, or system being monitored fails.
10. **Resource and energy efficiency.** An earlier-stage direction: connect application behavior to infrastructure utilization, GPU costs, energy use, and carbon impact.

**Suggest 5 ways in which Drasi could play a valuable role in solving these issues and future directions**

**Drasi’s strongest role would be a continuously updated context and change-detection layer alongside existing observability tools—not a replacement for their telemetry collection, storage, or analysis.**

1. **Keep a live picture of operational context.**  
   Combine service health, dependencies, deployments, configuration, ownership, and customer information into continuously maintained views. For example: “Which customer-facing services depend on this degraded database, who owns them, and what recently changed?” This would reduce cross-tool investigation and give people and AI assistants current, relevant context. It would support diagnosis, not automatically prove root cause.

2. **Turn noisy signals into meaningful operational changes.**  
   Define conditions that matter across multiple systems rather than reacting independently to every event. For example: “A critical service has degraded, its backup is unavailable, and customers are affected.” Drasi’s result changes can drive notifications when that situation appears, changes, or clears. This could reduce alert noise and replace custom polling, correlation, and state-tracking code.

3. **Monitor business outcomes—including things that fail to happen.**  
   Observe application and business records directly to detect incomplete workflows, not just unhealthy infrastructure. For example: “Payment succeeded, but fulfillment has not started within the expected time.” This would expose failures that component-level monitoring misses and connect reliability to customer impact. The same approach could detect stalled AI tasks even when the model endpoint remains healthy.

4. **Continuously assess releases and configuration changes.**  
   Connect deployment records and feature flags with health signals and business outcomes. For example: “Customers receiving the new version are experiencing increased checkout failures.” Drasi could maintain the affected services and customer groups, trigger a release-review or rollout-pause workflow, and track recovery. This would feed production evidence back into development, especially as AI increases the rate of code changes.

5. **Give AI-assisted operations explicit rules and feedback.**  
   Use continuous queries to identify situations eligible for automated handling and send agents focused context when those situations change. After an approved action, keep observing whether the expected recovery occurs. For example: “Request investigation when these conditions hold; escalate if the service does not recover.” Drasi would supply observable conditions and feedback—not the agent’s reasoning, authorization, or safety guarantees.

These would require suitable source integrations and explicit data relationships; approvals, permissions, and safe execution would still need to be enforced by the surrounding system.

**how are these handled today, and why would drasi be beneficial?**

**All five are already handled today. Drasi’s potential benefit is not a new observability capability—it is a simpler, reusable way to maintain cross-system context and react when meaningful conditions change.** Its value is strongest where organizations currently stitch tools together with custom code.

### 1. Keeping operational context current

**Today:** Observability platforms build service maps and connect telemetry with deployments and ownership. Service catalogs and configuration databases add more context. Teams use integrations, API lookups, and manual investigation to bridge gaps.

**Drasi’s benefit:** Continuously combine selected information from those systems with business data. A change to a dependency, deployment, customer relationship, or owner could update the same operational view—not just changes in telemetry.

**Best fit:** Questions that cross tool boundaries, such as “Which premium customers now depend on a degraded service?” Less valuable if an existing platform already maintains that view.

### 2. Turning noisy signals into actionable situations

**Today:** Alerting tools already support grouping, deduplication, suppression, and combined conditions—for example, [Datadog composite monitors](https://docs.datadoghq.com/monitors/types/composite.md). More complex cases use stream processors or custom correlation services.

**Drasi’s benefit:** Express conditions involving changing relationships and business state, not just combinations of alerts. Maintain which situations currently match and report when they appear, change, or disappear.

**Best fit:** “Alert when a failed dependency affects an active customer workflow with no available alternative.” Simple threshold alerts do not justify adding Drasi.

### 3. Detecting incomplete business outcomes

**Today:** Distributed tracing and business-observability products connect workflow events. Workflow engines track progress and deadlines. Elsewhere, scheduled jobs query databases for overdue work.

**Drasi’s benefit:** Observe records across existing systems and detect missing progress without taking over execution of the workflow. For example, connect payment and fulfillment records and identify orders that remain unfulfilled beyond a deadline.

**Best fit:** Processes spread across legacy applications, databases, and organizational boundaries. If one workflow engine already owns the complete process and its timers, Drasi may add little.

### 4. Assessing releases and configuration changes

**Today:** Deployment tools compare error rates, latency, and other indicators against release criteria. [Argo Rollouts](https://argo-rollouts.readthedocs.io/en/stable/features/analysis/), for example, can continue, pause, or abort a rollout based on analysis.

**Drasi’s benefit:** Supply richer release conditions involving deployment state, feature flags, customer groups, and business outcomes. For example, detect that a new version is associated with stalled fulfillment for a particular customer group, despite normal HTTP success rates.

**Best fit:** Business-aware release assessment that requires joining multiple systems. The deployment controller should still handle rollout and rollback; Drasi would provide additional evidence.

### 5. Supporting AI-assisted operations

**Today:** Operational assistants query monitoring tools, retrieve runbooks and incident history, and use integrations to investigate. Automation platforms provide execution workflows and approval controls. Some products already maintain shared context and operational memory.

**Drasi’s benefit:** Maintain predefined operational conditions continuously and notify an agent when they change, rather than making it repeatedly reconstruct everything through tool calls. The same mechanism could track whether an approved action produced the expected recovery.

**Best fit:** Repeatable, cross-system questions and explicit recovery conditions. Drasi would not replace open-ended diagnosis, authorization, or safeguards—and a disappearing result must not be mistaken for recovery if its source stopped reporting.

### Where the advantage is credible

The strongest proposition is **replacing scattered polling jobs, joins, caches, timers, and change-comparison logic with shared continuous queries and reactions**.

That is not exclusive to Drasi: stream processors and streaming databases can implement much of it too. Drasi would need to demonstrate less implementation and maintenance effort for the chosen workload. If existing tools already solve the problem cleanly, adding Drasi means another component to operate—not an automatic improvement.

**what products and standards dominate in the observability stack implemented in large organizations. which of them are open source**

**Large enterprises generally run a mixed stack:** commercial observability platforms, open-source collection and monitoring components, and their cloud provider’s native tools. The strongest common foundations are **OpenTelemetry, Prometheus, and Grafana**; the major commercial names include **Datadog, Dynatrace, Splunk, New Relic, and Elastic**.

Below is a practical landscape—not a precise market-share ranking. Prevalence varies between cloud-native teams and established enterprise IT.

## 1. Main enterprise platforms

| Product/platform | Typical role | Open source? |
|---|---|---|
| **Datadog** | Integrated infrastructure, application, logs, traces, and user-experience monitoring | **No** for the platform; some agents and libraries are open source |
| **Dynatrace** | Application monitoring, dependency mapping, infrastructure, and automated analysis | **No** for the platform |
| **Splunk Enterprise / Splunk Cloud** | Centralized log search and analytics, often shared between operations and security | **No** |
| **Splunk Observability Cloud / AppDynamics** | Application performance, infrastructure monitoring, and tracing | **No** for the platforms |
| **New Relic** | Application performance and broader full-stack observability | **No** for the platform; many instrumentation components are open source |
| **Elastic Observability** | Log analytics, application monitoring, metrics, and tracing | **Mixed**—important licensing distinction below |
| **Grafana ecosystem / Grafana Cloud** | Dashboards and integrated metrics, logs, traces, and profiling | **Open-source core projects**, with commercial cloud services and enterprise features |
| **Azure Monitor / Application Insights; AWS CloudWatch / X-Ray; Google Cloud Monitoring / Logging / Trace** | Native monitoring within each cloud | **Commercial managed services**, even where they integrate with open-source components |

**IBM Instana** is another established commercial application-observability product. Around these platforms, **Cribl** handles telemetry routing and processing, while **PagerDuty** and **ServiceNow** handle incident and operational workflows; these commercial products are not open-source platforms.

## 2. Important open-source building blocks

These are components organizations combine, not necessarily complete replacements for an integrated platform.

| Component | What it does | License |
|---|---|---|
| **OpenTelemetry SDKs and Collector** | Instrument applications; receive, process, and export telemetry | Apache-2.0 |
| **Prometheus and Alertmanager** | Collect and query metrics; evaluate alerts and route notifications | Apache-2.0 |
| **Grafana OSS** | Dashboards, visualization, and alerting across data sources | AGPLv3 |
| **Grafana Loki** | Log storage and querying | AGPLv3 |
| **Grafana Tempo** | Distributed trace storage and querying | AGPLv3 |
| **Grafana Mimir** | Scalable, long-term Prometheus-compatible metrics storage | AGPLv3 |
| **Jaeger** | Distributed tracing and investigation | Apache-2.0 |
| **OpenSearch and OpenSearch Dashboards** | Search, log analytics, and visualization | Apache-2.0 |
| **Fluent Bit / Fluentd** | Collect, process, and forward logs and other supported telemetry | Apache-2.0 |
| **Grafana Alloy** | An OpenTelemetry Collector distribution with additional integrations | Apache-2.0 |

Among these, **OpenTelemetry, Prometheus, and Grafana are the most important common integration targets**. Loki, Tempo, Mimir, Jaeger, and OpenSearch are significant backend choices, but should not all be assumed to have equal enterprise adoption.

Sources: [OpenTelemetry](https://opentelemetry.io/docs/what-is-opentelemetry/), [Prometheus](https://prometheus.io/docs/introduction/overview/), [Grafana licensing](https://grafana.com/licensing/), [OpenSearch](https://opensearch.org/about/).

### The Elastic licensing caveat

“ELK is open source” is now too imprecise:

- **Elasticsearch and Kibana:** free portions of their source code offer an **AGPLv3** option. Their default distributions use **Elastic License 2.0**, which is source-available rather than OSI-approved open source.
- **Logstash and Beats:** retain Apache-2.0-licensed components; check the particular distribution and included features.
- **OpenSearch:** provides a separate Apache-2.0-licensed search and analytics stack.

[Elastic’s licensing FAQ](https://www.elastic.co/pricing/faq/licensing) explains the distinction. **AGPL is open source; source-available does not necessarily mean open source.**

## 3. Standards and interfaces that matter

Standards are specifications, so “open standard” is more appropriate than “open-source product.”

| Standard or interface | Why it matters |
|---|---|
| **OpenTelemetry APIs and semantic conventions** | Common instrumentation interfaces and names for services, requests, errors, and other telemetry attributes. Coverage and maturity vary by domain. |
| **OTLP—OpenTelemetry Protocol** | A common way to transport metrics, logs, and traces between applications, collectors, and backends. |
| **W3C Trace Context** | Standard headers that preserve trace identity across service boundaries, allowing a request to be followed end to end. |
| **Prometheus exposition formats / OpenMetrics** | Widely supported interfaces for exposing metrics, particularly in Kubernetes and cloud-native infrastructure. |
| **PromQL** | A widely implemented metrics query language—a de facto ecosystem interface rather than a universal observability standard. |
| **Syslog and SNMP** | Still important for network equipment, appliances, and traditional enterprise infrastructure. |

Sources: [OpenTelemetry specifications](https://opentelemetry.io/docs/what-is-opentelemetry/), [W3C Trace Context](https://www.w3.org/TR/trace-context/), [OpenMetrics](https://prometheus.io/docs/specs/om/open_metrics_spec/).

**The key distinction:** an enterprise can standardize on open instrumentation and protocols while keeping a proprietary backend. That is a common architecture—not a contradiction.

**I dont really understand the purpose / relationship between prometheus, grafana, and Otel. please clarify**

They do different jobs, although their capabilities overlap:

| Technology | Main job | Plain-English description |
|---|---|---|
| **OpenTelemetry—OTel** | Generate, collect, and export monitoring data | “Make the application report what it is doing, using a common format.” |
| **Prometheus** | Collect, store, query, and alert on **metrics** | “Keep a history of numerical measurements and detect concerning conditions.” |
| **Grafana** | Query data sources and present dashboards and alerts | “Let people see and explore what is happening.” |

## A concrete example

Suppose you run an online store and want to understand slow checkouts.

**1. OpenTelemetry instruments the application.**  
It helps the application produce:
- **Metrics:** request counts, error counts, and response times.
- **Traces:** the path and time taken through checkout, payment, and inventory services.
- **Logs:** individual records describing what happened.

An optional **OTel Collector** receives that data, processes it, and sends it to the appropriate storage systems. **OTel itself is not the historical database or dashboard.**

**2. Prometheus handles the metrics.**  
It stores numerical measurements over time. You can ask, “What percentage of checkout requests failed in the last five minutes?” It can also evaluate an alert rule when that percentage gets too high.

**Prometheus does not store your application logs or distributed traces.** Those need other backends, such as Loki for logs and Tempo for traces.

**3. Grafana presents the information.**  
Grafana queries Prometheus to draw checkout performance charts. It can also query the log and trace backends, letting you investigate different kinds of evidence from the same interface.

## Why the boundaries seem confusing

**Prometheus can collect metrics without OTel.** It commonly reads—or “scrapes”—metrics exposed directly by applications. So **Prometheus + Grafana works perfectly well without OpenTelemetry**.

**OTel can send data somewhere other than Prometheus**, including a commercial platform such as Datadog or Dynatrace.

**“Grafana” can mean two things:** the open-source dashboard application, or the broader Grafana product family and cloud service, which also supplies storage and other capabilities.

The simplest mental model is: **OTel provides instrumentation and transport; Prometheus is a metrics engine; Grafana is the viewing and exploration interface.**

**Does an otel collector push metrics, traces, and logs to other systems, or does it make the data available for the others systems to pull? WRT the storage of metrics, traces, and logs what product options exist for each type of data.**

## 1. Does the OTel Collector push or expose data for pulling?

**Both are possible. Most exporters push; exposing metrics for Prometheus to pull is an important exception.** You choose the behavior through the Collector’s configuration.

| Method | What happens | Data types |
|---|---|---|
| **OTLP exporter—push** | The Collector sends batches to a backend’s ingestion endpoint over HTTP or gRPC | Metrics, traces, and logs, depending on backend support |
| **Prometheus exporter—pull** | The Collector exposes a `/metrics` endpoint. Prometheus periodically reads—or “scrapes”—it | Metrics only |
| **Prometheus Remote Write exporter—push** | The Collector sends metrics to a compatible backend such as Mimir | Metrics only |

The same Collector can therefore **push logs and traces while exposing metrics for scraping**. It can also send copies of data to multiple destinations.

The input side is independent: a Collector can receive data pushed by applications, scrape metrics itself, or read log files, then forward that data through its configured exporters.

**The Collector is not a historical database.** Its scrape endpoint exposes current metric values, not an archive you can search. Queues and buffering help delivery; historical storage belongs in the backend.

Sources: [Collector architecture](https://opentelemetry.io/docs/collector/architecture/), [Prometheus exporter](https://github.com/open-telemetry/opentelemetry-collector-contrib/tree/main/exporter/prometheusexporter), [Remote Write exporter](https://github.com/open-telemetry/opentelemetry-collector-contrib/tree/main/exporter/prometheusremotewriteexporter).

## 2. What can store each type of data?

You can use **specialized backends for each signal**, or **one commercial platform that handles all three**.

### Open-source and self-hosted options

| Data | Main options | How they differ |
|---|---|---|
| **Metrics** | **Prometheus**, **Grafana Mimir**, **VictoriaMetrics**, **Prometheus with Thanos** | Prometheus is a straightforward metrics database and monitoring engine. Mimir and VictoriaMetrics support larger centralized deployments. Thanos adds capabilities such as long-term storage and querying across Prometheus instances. |
| **Traces** | **Grafana Tempo**, **Jaeger** | Tempo provides trace storage and querying, commonly using object storage underneath. Jaeger provides tracing capabilities backed by a database such as OpenSearch, Elasticsearch, or Cassandra. |
| **Logs** | **Grafana Loki**, **OpenSearch**, **Elasticsearch** | Loki primarily indexes labels and stores compressed log content. OpenSearch and Elasticsearch provide richer indexed search over log content and fields. |

**Licensing:** Prometheus, Thanos, VictoriaMetrics’ open-source edition, Jaeger, and OpenSearch use Apache-2.0. Mimir, Tempo, and Loki use AGPLv3. Elasticsearch has the mixed licensing discussed earlier: an AGPL option for eligible source code, but ELv2 for default distributions.

An important distinction: **Jaeger is not itself the underlying database**. It uses a [storage backend](https://www.jaegertracing.io/docs/latest/storage/). Similarly, systems such as Loki and Tempo can store their data in object storage, but provide the ingestion and query capabilities that a storage bucket alone does not.

### Commercial and managed options

**Datadog, Dynatrace, New Relic, Elastic Observability, and Grafana Cloud** offer services spanning metrics, traces, and logs. You do not have to assemble separate backends yourself.

Cloud providers also offer signal-specific services:

| Data | AWS examples | Azure examples | Google Cloud examples |
|---|---|---|---|
| **Metrics** | CloudWatch Metrics; Amazon Managed Service for Prometheus | Azure Monitor Metrics; managed Prometheus | Cloud Monitoring; Managed Service for Prometheus |
| **Traces** | X-Ray | Application Insights | Cloud Trace |
| **Logs** | CloudWatch Logs | Azure Monitor Logs / Log Analytics | Cloud Logging |

## Putting it together

A straightforward open-source combination is **OTel Collector → Prometheus for metrics, Tempo for traces, and Loki for logs**, with **Grafana querying those three backends**.

Grafana reads the stored data to build dashboards and support investigation; it generally does **not** query the Collector for historical telemetry.

**Do any of these commercial products provide test/dev/trial versions that i would be able to use to develop / demonstrate commercial product integration with drasi?**

## Summary

**Important current change:** Splunk Observability Cloud now publicly offers a **Free Edition for up to 15 hosts, with no time-limited trial and no credit card required**—not merely the older 14-day trial. Datadog offers a 14-day trial and an ongoing limited infrastructure Free tier; Dynatrace documents a 15-day trial. Splunk Cloud Platform and self-managed Splunk Enterprise have separate offers.

Checked official public pages on **November 7, 2026**. Only pages were fetched; no registrations, submissions, contacts, or changes occurred.

## Offers

| Product | Public offer / credit card | Ongoing free/dev option and expiry | Integration entitlement caveat |
|---|---|---|---|
| **Datadog** | **14 days; no card.** Trial page explicitly describes metrics, logs, traces and alerting. [Trial](https://www.datadoghq.com/free-datadog-trial/) | Pricing FAQ says that after trial you **choose paid or Free**; Free provides **up to five Basic hosts, one-day metric retention**. This is not a full-stack permanent free allowance. Automatic downgrade is **not** stated. [Pricing—Free-tier FAQ](https://www.datadoghq.com/pricing/#infrastructure-monitoring-what-infrastructure-monitoring-is-included-in-the-free-tier) | API/webhook documentation exists, but I did not verify an explicit trial entitlement matrix. Also, do **not** repeat the old blanket claim “Datadog Free has no alerts”: the current Basic comparison lists alerts, but webhook eligibility specifically on the Free tier remains unverified. [Pricing](https://www.datadoghq.com/pricing/) |
| **Dynatrace** | **15 days**, documented under “Try with your data”; **no card**, stated on the public offer page. [Getting started](https://docs.dynatrace.com/docs/discover-dynatrace/get-started), [Offer](https://www.dynatrace.com/try-free/) | No ongoing own-data free tier verified. **Playground** is a preconfigured sample-data sandbox, explicitly involving no installation/data ingestion—not a substitute for a writable integration environment. Trial-expiry/access/deletion behavior not verified. [Getting started](https://docs.dynatrace.com/docs/discover-dynatrace/get-started) | Product APIs and custom webhook notifications are documented; their availability/scopes in a newly provisioned trial were not tested or guaranteed by the offer pages. |
| **Splunk Observability Cloud** | **Free Edition, up to 15 hosts; no card; explicitly “no time-limited trial.”** [Current offer](https://www.splunk.com/en_us/download/observability-cloud-free-edition.html) | Ongoing Free Edition, rather than a fixed trial countdown. The offer currently emphasizes Agent Observability alongside infrastructure/services. No inactivity-deletion or complete quota/retention policy verified. [Offer](https://www.splunk.com/en_us/download/observability-cloud-free-edition.html), [Product page](https://www.splunk.com/en_us/products/observability.html) | Strongest advertised ongoing SaaS offer in this scope, **but no verified promise that every API, detector, outgoing webhook, APM capability, or log facility is enabled in Free Edition**. Form asks for business email/company/job title/phone; acceptance of an individual’s personal email was not tested. |
| **Splunk Cloud Platform** | **14 days; no card.** This is the data/search platform, **not Observability Cloud**. [Trial](https://www.splunk.com/en_us/download/splunk-cloud.html) | No permanent public free Cloud Platform tier verified. Separate Cloud Developer Edition is limited/private preview—see below. Expiry/data deletion behavior unverified. | **Do not assume external REST/search API access in the free cloud trial.** I could not verify trial-specific access; relevant Splunk REST/cloud documentation returned 403. Product API existence is not sufficient. |
| **Splunk Enterprise — self-managed** | **60 days; 500 MB/day; no card.** Download page says afterward you can convert to a **perpetual free license** or purchase Enterprise. [Trial/download](https://www.splunk.com/en_us/download/splunk-enterprise.html) | Separate **free Developer license: 10 GB/day, full Enterprise features, six months, renewable, development only**. Requires Developer Program membership/application; review can take up to three days. **Not automatic eligibility.** [License](https://dev.splunk.com/enterprise/dev_license/), [Program](https://dev.splunk.com/enterprise/docs/devtools/developerprogram/) | Developer license is a stronger documented development entitlement than relying on reduced Free-license functionality. Free-license alerting restrictions could not be reverified because the admin documentation returned 403. |

## Documented integration surfaces — not trial/Free entitlement guarantees

- **Datadog → Drasi:** Custom outgoing webhook integration sends JSON HTTP POST on monitor alerts, with customizable headers/payload. `$ALERT_ID`, `$ALERT_CYCLE_KEY`, and `$ALERT_TRANSITION` support identifying and updating alert state, including recovered notifications. [Webhooks](https://docs.datadoghq.com/integrations/webhooks.md)  
  **Queries:** [metric time series](https://docs.datadoghq.com/api/latest/metrics/query-timeseries-points.md), [logs search](https://docs.datadoghq.com/api/latest/logs.md), [span search/aggregation](https://docs.datadoghq.com/api/latest/spans.md). **Write-back:** [Events API](https://docs.datadoghq.com/api/latest/events.md). Authentication uses API keys and, for some endpoints, application keys. [API authentication](https://docs.datadoghq.com/api/latest.md)

- **Dynatrace → Drasi:** Custom problem-notification webhooks POST customizable payloads on problem detection/resolution; placeholders include problem identifiers/state, and custom authorization headers are supported. This documentation is explicitly **Dynatrace Classic / Settings Classic**. [Webhook integration](https://docs.dynatrace.com/docs/analyze-explore-automate/alerting-and-notifications-classic/problem-notifications/webhook-integration)  
  **Queries:** `GET /api/v2/metrics/query`, requiring `metrics.read`; Grail/DQL covers logs, traces and other telemetry. [Metrics API](https://docs.dynatrace.com/docs/dynatrace-api/environment-api/metric-v2/get-data-points), [Grail](https://docs.dynatrace.com/docs/platform/grail/dynatrace-grail), [Query SDK](https://developer.dynatrace.com/develop/sdks/client-query/)  
  **Write-back:** `POST /api/v2/events/ingest`, with documented ingest permissions. [Events ingest](https://docs.dynatrace.com/docs/dynatrace-api/environment-api/events-v2/post-event)

- **Splunk Observability Cloud → Drasi:** Detector notifications support arbitrary webhook services and trigger/clear state. [Notifications](https://dev.splunk.com/observability/docs/detectors/notifications/)  
  **Queries:** metrics via SignalFlow streams or limited `GET /v1/timeserieswindow`; custom events via `/v2/event/find`; APM trace download by trace ID/segment. [Data retrieval](https://dev.splunk.com/observability/docs/datamodel/data_retrieval_overview/), [Trace download](https://dev.splunk.com/observability/docs/apm/download_traces/)  
  **Write-back:** `POST https://ingest.<REALM>.observability.splunkcloud.com/v2/event`, requiring an INGEST-scoped organization token. [Custom events](https://dev.splunk.com/observability/docs/datamodel/custom_events/) Authentication uses `X-SF-TOKEN`. [Authentication](https://dev.splunk.com/observability/docs/apibasics/authentication_basics/) **A standalone log-query entitlement was not verified.**

- **Splunk Platform/Enterprise → Drasi:** Scheduled searches can invoke built-in webhook alert actions; SDK job/search APIs support querying and exporting results. **Write-back:** token-authenticated HTTP Event Collector accepts application logs/metrics over HTTP(S). [Alert actions](https://dev.splunk.com/enterprise/docs/devtools/customalertactions/), [Search/job APIs](https://dev.splunk.com/enterprise/docs/devtools/java/sdk-java/howtousesdkjava/howtoworkjobjava/), [HEC](https://dev.splunk.com/enterprise/docs/devtools/httpeventcollector/)

## Developer sandboxes

- **Datadog:** Approved Technology Partners can request a dedicated development sandbox through the Partner Portal. Application approval and a sandbox request are required; this is **not** an automatic personal developer tier. [Official prerequisites](https://docs.datadoghq.com/extend/integrations.md#prerequisites)
- **Splunk:** Cloud Developer Edition provides Cloud Platform deployments for testing apps, but remains **limited private preview**. [SCDE](https://dev.splunk.com/enterprise/docs/testappsscde/)
- **Dynatrace:** Verified public Playground only; no dedicated, automatically eligible integration-development sandbox verified. [Playground description](https://docs.dynatrace.com/docs/discover-dynatrace/get-started)

## Practical recommendation

**Investigate Splunk Observability Cloud Free Edition first for a durable hosted demo**, but make API-token creation and a detector’s outbound webhook the first entitlement checks. For a Platform/SPL integration, prefer **local Splunk Enterprise**, initially its trial and potentially the approved renewable Developer license. Datadog is an attractive short, focused webhook demo; do not assume its ongoing Free tier preserves every integration feature.

Suggested architecture: vendor alert webhook → small authenticated adapter → normalized alert-state updates into Drasi; use product query APIs for enrichment and optional event APIs/HEC for derived-event write-back. Treat this as a proposed integration design, not an already verified Drasi connector.

**Retrieval caveat:** Several marketing pages’ markdown extraction omitted the actual offer. Verified raw HTML locations were approximately Datadog trial **35,000–41,000**, Datadog pricing Free FAQ **430,000–443,000**, and Splunk Observability Free Edition **103,000–110,000**. The latter page also contains hidden “Thank you for registering” text; that was page content, **not a registration performed during research**.

**Yes. You can develop a small Drasi integration without buying an enterprise subscription.** For an environment that stays available, I would start with **New Relic or Grafana Cloud** rather than a time-limited trial.

The following offers are currently advertised on the vendors’ sites.

## Available environments

| Product | Free, trial, or developer access | Suitability for Drasi development |
|---|---|---|
| **[Grafana Cloud](https://grafana.com/pricing/)** | Ongoing free tier: **10,000 active metrics series**, **50 GB/month each of logs and traces**, and 14-day retention. No credit card. | Strong starting point: OTLP ingestion, telemetry query APIs, and alert webhooks. Also lets you develop against related open-source components locally. |
| **[New Relic](https://newrelic.com/pricing)** | Ongoing free tier: **100 GB/month total ingestion**, **one full-platform user**, and no credit card. | Probably the easiest sustained demo against a primarily proprietary platform. Its **NerdGraph query API explicitly supports getting started with a free account**. |
| **[Splunk Observability Cloud](https://www.splunk.com/en_us/download/observability-cloud-free-edition.html)** | **Free Edition for up to 15 hosts**, with no time-limited trial or credit card. | Promising ongoing option. Confirm that the Free Edition enables the specific API tokens, detectors, and outbound webhooks your demo needs. |
| **[Splunk Enterprise—self-managed](https://www.splunk.com/en_us/download/splunk-enterprise.html)** | **60-day trial at 500 MB/day**. Separately, an application-based **[Developer license](https://dev.splunk.com/enterprise/dev_license/)** offers **10 GB/day for six months**, renewable, for development only. | Particularly useful for a local, repeatable integration using searches, alert webhooks, and HTTP Event Collector. Developer-license approval is required. |
| **[Elastic Cloud](https://www.elastic.co/cloud/cloud-trial-overview)** | **14-day trial**, no credit card, with resource limits. | Good for Elasticsearch query/ingestion APIs and Elastic Observability demonstrations. Register directly with Elastic; its page says Azure and Google Cloud marketplace signups do not include this trial. |
| **[Datadog](https://www.datadoghq.com/free-datadog-trial/)** | **14-day trial**, no credit card. An ongoing Free tier exists, but is limited infrastructure monitoring—not a permanent full-stack trial. | Good for a focused monitor-webhook and event-write-back demonstration. Approved technology partners can request a development sandbox. |
| **[Dynatrace](https://docs.dynatrace.com/docs/discover-dynatrace/get-started)** | **15-day own-data trial**, no credit card. | Suitable for problem notifications, telemetry queries, and event ingestion. Its sample-data Playground is useful for exploration, but not a replacement for your own integration environment. |

## What I would build first

**New Relic** would be my first choice for demonstrating integration with a proprietary observability platform. **Grafana Cloud** would be my first choice for combining a commercial service with an open-source development environment.

A useful demonstration would:

1. Generate synthetic application telemetry and send it to the vendor.
2. Feed vendor alerts into Drasi through an authenticated adapter, using query APIs for additional context.
3. Join those alerts with customer, deployment, or ownership data in Drasi.
4. Send the resulting business-impact event back to the vendor through its ingestion API.

That demonstrates **Drasi adding cross-system context**, rather than merely forwarding telemetry.

Two qualifications matter: **these are available integration interfaces, not necessarily existing Drasi connectors**, and **documented API availability does not guarantee every feature is enabled in a particular free/trial account**. Check the exact webhook and API permissions early. For a standing public/customer demo, confirm the vendor’s evaluation or developer-license terms rather than assuming development access permits every demonstration use.

**Consider the headline issues and future directions identified earlier. Consider the dominant open source products and the commercial products that have a free tier or a trial option, especially those that i can run locally. I am going to give this presentation at Microsoft Ignite: https://ignite.microsoft.com/en-US/sessions/1787856602420001EGgA-1791588144696001Cv8p?source=sessions. I can change the specifics of the topic, but it is generally about using drasi in an observability environment. Suggest a demo i could do that would enable people to see the value of drasi.**

## Recommended demo: “Checkout is healthy. Customers aren’t getting their orders.”

**Show Drasi turning technical signals and business records into one continuously updated customer-impact incident—and keeping it open until the customer problem is actually resolved.**

This fits your [25-minute Ignite session](https://ignite.microsoft.com/en-US/sessions/1787856602420001EGgA-1791588144696001Cv8p?source=sessions) better than several disconnected examples of failures, heartbeats, and SLOs. One story can demonstrate all three, plus a controlled recovery.

### The scenario

A small order-processing application has a checkout API and a fulfillment worker.

Checkout continues accepting orders successfully, but the worker stops processing them. HTTP success rates remain healthy. Existing monitoring can report the worker problem or growing backlog, but the operational questions are:

> **Which customers are affected? Which commitments are overdue? Who should act? Has the customer problem actually recovered?**

Drasi combines order progress, worker state, customer priority, and service ownership to maintain those answers.

Use explicitly shortened demo deadlines—perhaps 30 seconds—rather than making the audience wait.

## The onstage story

| Moment | What the audience sees | What it demonstrates |
|---|---|---|
| **1. Everything works** | Orders arrive and complete. Grafana shows normal traffic. The live customer-impact panel is empty. | Establishes an ordinary observability environment, not a replacement platform. |
| **2. Pause the fulfillment worker** | Checkout still returns success. Worker progress heartbeats stop and orders accumulate. Existing monitoring shows technical symptoms. | Component health and successful customer outcomes are different questions. |
| **3. Do nothing** | An order crosses its completion deadline. Drasi adds it to the overdue results **without another update to that order**. An incident appears with affected customers, overdue orders, and owner. | The strongest “without polling” moment: the passage of time changes the answer. |
| **4. Change business context** | Mark an affected customer as business-critical in the customer database. The existing incident’s priority changes, without another infrastructure failure or threshold crossing. | Live cross-system context—not just a one-time enrichment of an alert. |
| **5. Approve a bounded recovery** | An operator approves “resume this demo worker.” A reaction invokes that specific action. Heartbeats recover, but the customer-impact incident stays open while overdue orders remain. | Safe action and verification of the business outcome, not merely successful execution of a command. |
| **6. Watch the backlog clear** | As orders complete, the affected count falls. The current incident resolves when the outstanding problem is gone. Its history remains visible. | Added/updated/removed results drive an incident lifecycle. Recovery does not erase missed deadlines. |

**The two memorable moments are:** “Nothing changed, but Drasi detected an overdue commitment” and “The server recovered, but Drasi knows the customers have not recovered yet.”

## What to run locally

I would use Docker Compose and **Drasi Server**, avoiding Kubernetes for this presentation.

| Component | Purpose |
|---|---|
| **Small order API and fulfillment worker** | Generate real requests, records, telemetry, and a controllable failure |
| **PostgreSQL** | Orders and worker progress; a separate logical database can hold customer priorities and service ownership |
| **OpenTelemetry Collector** | Collect and route the application’s telemetry |
| **Prometheus + Grafana** | Familiar metrics, technical alerts, and dashboards |
| **Loki** | Application logs and the incident events produced by Drasi |
| **Drasi Server** | Observe database changes, maintain the cross-system conditions, and trigger reactions |
| **Small live impact panel** | Display result changes immediately through Drasi’s SSE reaction |

Drasi’s documented [PostgreSQL source](https://drasi.io/drasi-server/how-to-guides/configuration/configure-sources/configure-postgresql-source/) uses logical replication. Its [future functions](https://drasi.io/reference/query-language/drasi-custom-functions/), such as `drasi.trueLater`, support deadline-driven evaluation. These are a direct fit rather than features that need to be invented for the demo.

**Keep raw telemetry in the observability stack.** Drasi should receive the selected operational state needed for its queries—not become another repository for every log and trace.

You can also feed Grafana alert transitions through Drasi’s HTTP webhook source as supporting context. Crucially, the deadline detection should not depend on waiting for a Grafana alert.

## Where a commercial product fits

**Use local Grafana as the primary environment. Add local Splunk Enterprise only if showing a commercial integration is important to the story.**

Splunk’s local trial would let you send the same enriched incident events through its HTTP Event Collector and display them in a Splunk dashboard. That makes a useful closing point:

> “This is not a special Grafana feature. Drasi maintains the operational condition, and existing tools consume the results.”

Keep Splunk as a second output, not another detection engine or a second version of the demo. Confirm the trial’s demo-use terms and compatibility with your presentation machine beforehand.

I would not make New Relic, Datadog, or another internet-hosted service essential to the live presentation. Their free tiers and trials are useful for subsequent integration examples, but introduce avoidable conference-network and account dependencies.

## Why this shows Drasi’s value

The demo makes several earlier themes concrete:

- **Business impact:** affected customers and delayed orders, not just error rates.
- **Alert noise:** design one evolving incident rather than notifying separately for every symptom.
- **Missing outcomes:** detect work that should have completed but did not.
- **Shared context:** changes in customer priority or ownership immediately affect the operational picture.
- **


## Recommended demo: “Checkout is healthy. Customers are still waiting.”

**Show Drasi turning technical signals and business data into a live, actionable picture of customer impact—and detecting a problem even when no new event arrives.**

For your [25-minute Ignite session](https://ignite.microsoft.com/en-US/sessions/1787856602420001EGgA-1791588144696001Cv8p?source=sessions), I would use **one end-to-end incident**, not separate demonstrations of heartbeats, alerts, SLOs, and automation.

The audience’s takeaway should be:

> “Keep your observability tools. Use Drasi to continuously determine what their signals mean for your business—and whether corrective action actually worked.”

## The scenario

A small order-processing application has a checkout API and a fulfillment worker.

Checkout continues accepting orders successfully, but the worker stops processing them. HTTP success rates remain good. Existing monitoring can show the stalled worker or growing backlog, but the operational questions are:

**Which customers are affected? Which orders have missed their deadline? Who should act? Has the customer problem actually been resolved?**

Drasi combines:

- Order status and timestamps from the application database.
- Customer priority and service commitments from a separate business-data source.
- Worker heartbeats and relevant alerts.
- Ownership and permitted recovery actions.

Use explicitly shortened demo deadlines—perhaps 30 seconds instead of a real-world commitment measured in minutes or hours.

## The onstage story

| Moment | What the audience sees | What it demonstrates |
|---|---|---|
| **1. Everything works** | Orders arrive and complete. Grafana shows normal application health. Drasi’s “customers needing attention” view is empty. | Drasi complements the existing stack rather than replacing it. |
| **2. Pause the fulfillment worker** | Checkout still returns success. The worker stops making progress; technical monitoring provides evidence of the problem. | Component health and successful customer outcomes are different questions. |
| **3. Do nothing—and a deadline passes** | Without another update to an affected order, Drasi identifies it as overdue. An impact card shows the customer, delayed order value, responsible team, and supporting evidence. | **Absence-of-change detection**, combined with business context. |
| **4. Change business context, not telemetry** | Mark an affected account as business-critical. The existing incident’s priority changes without another metric crossing a threshold. | A continuously maintained operational view—not a one-time enrichment attached to an alert. |
| **5. Approve a narrowly defined recovery** | An operator approves “resume this demo worker.” A Drasi reaction invokes that specific action. | Bounded automation driven by explicit conditions, not unrestricted “AI self-healing.” |
| **6. Watch the real recovery** | The worker heartbeat returns immediately, but the customer-impact incident remains until outstanding orders complete. Then it resolves. | **A healthy process is not the same as a recovered business outcome.** |

**Moments 3 and 6 are the memorable ones.** Something becomes wrong without a new event; later, the infrastructure recovers before the customer problem does.

Do not have the application generate an “order overdue” event. Drasi should derive that condition from order state and time. Also preserve missed-deadline history after resolution—the original breach did not disappear.

## The stack I would use

Run the main demonstration locally with Docker Compose.

| Component | Purpose |
|---|---|
| **Small checkout API and fulfillment worker** | Generate real requests, operational records, telemetry, and controlled failures |
| **OpenTelemetry Collector** | Route application telemetry |
| **Prometheus + Grafana** | Familiar metrics, technical alerts, and dashboards |
| **Loki** | Application logs and the enriched incident events produced by Drasi |
| **PostgreSQL** | Orders, customer commitments, and ownership; separate logical databases can represent separate systems |
| **Drasi Server** | Continuous queries over database changes and pushed operational signals |
| **Small live impact panel** | Display Drasi result changes immediately, using its SSE reaction |

Drasi Server already documents [PostgreSQL change capture](https://drasi.io/drasi-server/how-to-guides/configuration/configure-sources/configure-postgresql-source/), [HTTP/webhook inputs](https://drasi.io/drasi-server/how-to-guides/configuration/configure-sources/configure-http-source/), [time-driven query functions](https://drasi.io/reference/query-language/drasi-custom-functions/), and HTTP/SSE reactions. That makes this a plausible integration project rather than a proposal requiring a new execution engine.

For Grafana alerts, use a small mapping layer where necessary to translate firing/resolved notifications into stable, keyed state in Drasi. **Do not send every raw log and metric through Drasi**; keep those in their existing backends.

I would omit Kubernetes and Tempo from the first version. Neither is necessary to communicate the central value.

## Where a commercial product fits

**Splunk Enterprise is the most useful local commercial addition.** Its trial gives you enough time to build and rehearse, and its developer-license route could support longer-term development.

Keep it as a **second output**, not another system the demo depends on:

- Send application logs to Splunk.
- Send Drasi’s enriched incident changes through Splunk’s HTTP Event Collector.
- Briefly show the customer-impact incident alongside its underlying technical evidence.

That demonstrates commercial integration without rebuilding the story or relying on conference internet. Grafana Cloud or New Relic could receive the same kind of output later, but I would not put a cloud account or tunnel on the critical path of the live presentation.

## Why this makes Drasi’s value visible

It addresses the earlier issues through observable behavior:

- **Fragmented context:** one view combines operational and business systems.
- **Alert noise:** responders see a maintained customer-impact situation rather than another unrelated notification.
- **Business relevance:** prioritization reflects affected customers and commitments.
- **Operational toil:** queries and reactions replace bespoke polling, deadline tracking, and change-comparison logic.
- **Safe automation:** recovery is constrained and its outcome is observed.

Be explicit that existing products and stream processors can implement similar behavior. **The claim is that Drasi makes this cross-system, change-driven logic easier to express and reuse—not that nobody else can detect a timeout.**

Likewise, “without polling” should describe **Drasi’s business-condition detection**. Prometheus scraping and Grafana’s existing alert evaluation may still be periodic.

For the 25 minutes, I would allocate roughly **4 minutes to the problem, 12 to the incident story, 6 to the model and queries, and 3 to limitations and the takeaway**. Keep AI off the critical path: the resulting structured incident is useful input to an agent, but the demo is stronger when the audience can clearly see what Drasi itself contributes.

**what about something that talks to the AI Observability issue**

**Yes—and I think it could make a stronger Ignite demo.** Focus on **observability of AI agents**, rather than using AI to analyze monitoring data.

## Demo: “The agent says it’s done. Did it actually succeed?”

The central distinction is:

> **An agent finishing its execution is not evidence that it achieved the intended outcome.**

AI observability tools capture prompts, responses, tool calls, latency, and cost. Drasi could complement them by continuously comparing **what the agent claims, what its tools did, and what actually changed in the business systems**.

## The scenario

A customer-support agent receives:

> “Cancel my order and issue the appropriate refund.”

Everything is simulated locally—no real payments or customer data.

The agent checks the order, calls cancellation and refund tools, and responds to the customer. The refund tool is asynchronous: it can accept a request before the refund is completed.

That gives you an understandable failure: **successful model calls, successful HTTP requests, and a confident final answer—but an incomplete customer outcome.**

## The main demonstration

| Moment | What happens | What Drasi adds |
|---|---|---|
| **1. Show a successful run** | The agent cancels the order, the refund completes, and it reports success. The AI observability tool shows the trace and cost. | Drasi independently confirms that the requested outcome exists in the business records. |
| **2. Pause the refund processor** | In a deliberately faulty run, the agent treats “request accepted” as “refund completed.” Its execution finishes without a technical error. | Drasi distinguishes the agent’s completion claim from verified completion. |
| **3. Let the completion deadline pass** | The trace has finished, but the refund remains pending. No new refund event arrives. | Drasi detects the missing outcome and identifies the affected task, customer, order, and tool operation. |
| **4. Put the finding back into the AI tool** | The original trace receives an outcome assessment and an explanation linked to the business evidence. | The AI observability platform now shows a quality problem that depends on external, changing state—not just the conversation. |
| **5. Restore processing** | The refund completes later. | Drasi reports recovery, while preserving the fact that the original completion claim was premature. |

**The strongest moment is step 3:** the agent has stopped running, but evaluation of its success must continue.

Use structured agent output such as `claimedOutcome: refund_completed`. Drasi would verify that claim against authoritative records; it would **not** infer the meaning of arbitrary prose itself.

## Add one short variation: “The agent is busy, but not making progress”

Run a second case where the agent repeatedly retries the stalled operation.

The AI tracing tool shows the sequence of model and tool calls. Drasi maintains a condition such as:

> “This task has made repeated attempts, consumed more than its assigned budget, and produced no corresponding business-state progress.”

That combines three important AI-observability concerns:

- **Agent loops:** repeated activity without useful progress.
- **Cost versus outcome:** spending per completed task, not just total token consumption.
- **Safe intervention:** request human review or signal the agent controller to pause further attempts.

The controller must enforce the pause and tool permissions. An asynchronous Drasi reaction alone is not a guarantee that another tool call cannot occur.

## Products I would use

| Component | Role |
|---|---|
| **Self-hosted Langfuse** | Show agent traces, prompts, tool calls, token usage, costs, and evaluation results |
| **Small support agent** | Perform the cancellation/refund task using a few clearly defined tools |
| **Local mock order and refund services backed by PostgreSQL** | Supply authoritative business state and controllable failure conditions |
| **Drasi Server** | Combine agent activity, business records, deadlines, and task budgets |
| **A thin integration adapter** | Push selected structured agent events into Drasi and publish Drasi assessments back to Langfuse |

[Langfuse supports local Docker Compose deployment](https://langfuse.com/self-hosting) and [programmatically supplied scores attached to traces](https://langfuse.com/docs/evaluation/scores/overview). That makes it a particularly good fit: **Drasi’s contribution can appear inside an existing AI-observability product**, rather than only in a custom dashboard.

Send full traces to Langfuse, but only selected task/tool lifecycle events to Drasi, with shared task and trace identifiers. PostgreSQL changes provide the independent outcome evidence. This avoids periodically polling Langfuse for completed traces.

For this version, I would make **Langfuse—not a full Grafana stack—the main observability interface**. Keep the number of systems visible onstage small.

## Why this demonstrates Drasi rather than just another evaluator

Langfuse and other AI-observability products already support evaluations and custom business checks. The differentiator is **continuously maintaining an assessment as external facts change**, including after the agent’s trace has ended.

Drasi is not judging whether an answer sounds convincing. It is evaluating explicit conditions such as:

- Was the requested operation actually completed?
- Did it complete within the agreed deadline?
- Did repeated attempts create duplicate business effects?
- Is the agent spending resources without making progress?

These checks also apply to conventional workflows. Their importance increases with agents because **the system’s own declaration of success is especially unsafe to treat as ground truth**.

For stage reliability, use controlled tool failures and retain a clearly labelled replay of a real faulty agent run. Do not depend on a live model making the desired mistake.

**Suggested session framing:**  
**“Beyond traces and tokens: verifying AI-agent outcomes with Drasi.”**


**any way to integrate traditional observability infrastructure with this?**

**Yes—and combining them makes the demo stronger.** Drasi can connect three views that are often investigated separately:

| View | Existing tooling | Question it answers |
|---|---|---|
| **Infrastructure and services** | Prometheus, Grafana, optionally Loki | Is the refund service healthy? Is its queue growing? |
| **AI-agent execution** | Langfuse | What did the agent say, which tools did it call, and what did it cost? |
| **Business outcome** | Order and refund databases | Was the correct order cancelled and the correct refund completed? |
| **Cross-system situation** | **Drasi** | Which agent tasks and customers are affected, what evidence supports that, and has the outcome recovered? |

## The integrated demo: “The service failed. The agent said success.”

Keep the cancellation/refund scenario, but make a conventional service failure the starting point.

### 1. Establish normal operation

An agent processes several synthetic customer requests. Show:

- **Grafana:** service health, request rates, queue depth, and processing throughput.
- **Langfuse:** agent conversations, tool calls, execution time, and cost.
- **A Drasi impact panel:** requested outcomes, agent-reported outcomes, and independently verified outcomes.

This establishes that all three views describe the same tasks.

### 2. Pause the refund worker

The refund API continues accepting requests, but its worker stops completing them.

Traditional monitoring correctly detects a growing backlog or missing worker heartbeat. **Do not portray the existing tools as failing to notice the infrastructure problem.**

In a controlled faulty agent run, “request accepted” becomes “your refund is complete.” Another agent might keep retrying and spending tokens.

Now the audience sees two related problems:

**The infrastructure is failing, and the agents are responding badly to that failure.**

### 3. Drasi turns those observations into an impact incident

Drasi combines the infrastructure alert, agent task records, customer context, and refund status.

An illustrative incident could read:

> **Refund processing stalled**  
> Four customer tasks affected.  
> Two agents reported completion without a completed refund.  
> One task is repeatedly retrying without progress.  
> Owner: Payments team.  
> Evidence: worker alert, affected agent traces, and pending refund records.

The useful output is not another copy of “queue depth is high.” It is **which AI-driven customer interactions are now incorrect or at risk**.

### 4. Publish the result into both existing tools

Send Drasi’s findings back to:

- **Grafana:** an enriched incident event or annotation, with links to affected tasks.
- **Langfuse:** an outcome assessment attached to each relevant agent trace.

An operator can start from either the infrastructure dashboard or an AI trace and discover the same customer-impact situation.

### 5. Restore the worker—and distinguish two recoveries

The worker becomes healthy before all pending refunds complete.

Grafana shows infrastructure recovery. Drasi continues tracking outstanding customer outcomes and resolves the impact incident only when those outcomes are confirmed.

**The premature agent claims remain recorded as failures; subsequent recovery does not make them retrospectively correct.**

## How to wire it

| Connection | Integration approach |
|---|---|
| Applications → observability tools | Use an **OTel Collector** to route metrics to Prometheus and AI traces to Langfuse. Langfuse supports [OTLP trace ingestion](https://langfuse.com/integrations/native/opentelemetry). |
| Traditional monitoring → Drasi | Send Grafana’s [alert webhooks](https://grafana.com/docs/grafana/latest/alerting/configure-notifications/manage-contact-points/integrations/webhook-notifier/) through a mapping layer into Drasi’s HTTP Source. Preserve alert identity and firing/resolved state. |
| Agent and business state → Drasi | Use database change capture for durable task, order, and refund records; push any additional structured agent lifecycle events. |
| Drasi → existing tools | Use reactions and a small adapter to publish Grafana annotations/events and Langfuse scores. |

Use shared **task IDs, trace IDs, service IDs, and business-operation IDs** throughout. Those explicit relationships make the correlation meaningful.

Do not use sampled traces as the only evidence that a refund happened—or did not happen. Outcome verification should rely on authoritative business records. Likewise, a missing or stale monitoring feed means **unknown**, not healthy.

## Keep the stage version focused

I would run **Prometheus + Grafana, Langfuse, PostgreSQL, and Drasi Server locally**, with one OTel Collector. Add Loki only if examining logs materially improves the story; you do not need another trace backend initially.

This demonstrates a clear role for Drasi:

> **Traditional observability tells us a service has a problem. AI observability tells us what the agent did. Drasi continuously connects those facts to whether the customer’s task actually succeeded.**

It supports operational diagnosis without claiming that correlation alone proves root cause, or that Drasi replaces either existing observability stack.

**the latest version of drasi still in development in the agentofreality-parallel-computation-graph branch of drasi-core repo significantly extends the functionality of drasi beyond just continuous queries. in addition you can have transformers that observe changes and update some stateful model that they maintain and output the changes to their state. this is used in the 'gpu cluster lab' example (https://github.com/drasi-project/drasi-server/tree/agentofreality-parallel-computation-graph/examples/gpu-cluster-lab) to deal with things like policy (rego) and optimization/sufficiency, as well as the move-a-wall example that does geometric collision detection (https://github.com/drasi-project/drasi-server/tree/agentofreality-parallel-computation-graph/examples/move-a-wall). With these capabilities to do more than just continuous queries, what else could be done as part of the presentation.**

**Yes—this changes the strongest demo from “Drasi detects meaningful conditions” to “Drasi continuously maintains models that assess what is happening and what can be done about it.”** My earlier recommendations were too query-centric.

The examples illustrate the distinction well:

- **GPU Cluster Lab:** policy evaluation, placement optimization, and resilience assessment produce derived state that queries and other components consume.
- **Move-a-wall:** geometry produces an `Obstruction`; a subsequent query connects that mathematical result to an affected task and destination.

That same pattern could connect **infrastructure telemetry → domain-specific assessment → AI-agent decisions → verified outcomes**.

## What the transformers make possible

These are proposed applications of the capability—not claims that all these components already exist.

| Addition | Model maintained by the transformer | Compelling demonstration |
|---|---|---|
| **Live policy assessment** | Which proposed actions are permitted under current policies, data classifications, destinations, and approvals | An agent’s proposed action becomes invalid when a policy parameter changes—even though the agent and infrastructure have produced no new error. |
| **Stateful agent-progress assessment** | Expected task stages, completed effects, outstanding obligations, retries, and accumulated cost | The agent is making successful tool calls but repeating the same work. The model identifies “activity without progress,” rather than merely counting errors. |
| **Feasible recovery planning** | Workload requirements, available resources, constraints, and alternative assignments | There is plenty of aggregate spare capacity, but no valid recovery plan—or recovery requires rearranging several workloads rather than restarting one service. |
| **Continuous what-if analysis** | Whether the current system can meet its requirements under specified hypothetical failures | Every dashboard is green, but losing one more VM would make recovery impossible. Adding capacity or changing policy updates that assessment before an outage happens. |

The most distinctive additions are **feasibility and what-if analysis**. They let you demonstrate something more substantial than another alerting rule.

## My revised recommendation: “Can this AI service recover—and is the recovery allowed?”

I would build on **GPU Cluster Lab**, integrating traditional observability and a small AI operations agent.

The agent’s job is to propose or explain recovery actions. **It does not replace the policy engine or optimizer.** Drasi maintains independent assessments of whether the proposal is permitted, feasible, and subsequently achieved.

### The scenario

You operate two kinds of AI workloads:

- A customer-support workload using restricted customer documents, permitted only in specified European regions.
- An unrestricted workload using synthetic data, which can run elsewhere.

Several modeled GPU hosts have different available capacities. The conventional monitoring dashboard shows their health and utilization.

The regional restriction is a fictional customer policy—not a claim about what GDPR universally requires.

### The onstage story

**1. Start with everything green—but insufficient resilience.**

The current allocation works. Prometheus/Grafana show healthy hosts and running services.

Drasi’s resilience transformer reports:

> “Current requirements are satisfied, but this fleet cannot recover from losing this VM under the current capacity and policy constraints.”

This is the first important contrast: **healthy now does not mean capable of recovery**.

**2. Fail the VM.**

Traditional monitoring correctly raises an infrastructure alert. Drasi updates the affected workloads and recovery assessment.

You are not replacing that alert. You are answering the next operational question.

**3. Let the agent propose an apparently reasonable shortcut.**

The agent proposes moving the restricted workload to an available US host.

Langfuse shows the agent’s reasoning and proposed tool call. Drasi’s policy assessment says:

> “Destination healthy and available. Processing this customer’s data there is not permitted.”

A separate execution boundary refuses the action. The trace receives the policy result and explanation.

**4. Show a recovery that requires a model, not a threshold.**

Use a prepared scenario where a valid solution requires moving an unrestricted workload out of Europe, freeing European capacity for the restricted workload.

The optimizer proposes a **complete valid allocation**, considering all workloads together and minimizing disruption.

This is considerably more interesting than “pick another healthy server.” An individually attractive destination is not necessarily part of a feasible system-wide plan.

**5. Change the circumstances while the proposal is pending.**

Change an allowed-region parameter or remove capacity.

The proposed plan becomes stale. Drasi publishes a new assessment rather than leaving the old recommendation looking valid indefinitely.

This demonstrates the value of a **maintained decision model**, rather than a one-off agent answer.

**6. Approve a current plan and observe actual recovery.**

Show the stages separately:

**Proposed → saved → applied → confirmed by fresh observations.**

A successful command response is not proof that the workloads are running correctly. The existing GPU example already makes this distinction central.

## Where each product belongs

| Component | Role |
|---|---|
| **Prometheus/Grafana** | Familiar infrastructure health, utilization, capacity, and alerts |
| **Langfuse** | Agent proposal, tool-call history, cost, and assessments attached to the agent trace |
| **Drasi policy transformer** | Current permission decisions with reasons |
| **Drasi placement transformer** | Feasible assignments and recovery proposals |
| **Drasi resilience transformer** | Read-only assessments of additional failures |
| **Continuous queries** | Select and combine context; connect assessments to workloads, customers, and operators |
| **Execution service and reactions** | Apply approved actions with current-version checks, then observe their effects |

The policy and optimization models produce **facts that can themselves be queried**. That is the broader architectural point—not simply that Drasi can call a solver.

## Why this is stronger than the original proposal

The refund demo primarily demonstrated **whether an agent’s outcome was true**.

This version additionally demonstrates:

- Whether an agent’s proposed action is **permitted**.
- Whether it is **possible given the whole system**, rather than one resource.
- Whether a working configuration is **resilient to specified failures**.
- Whether a recommendation remains valid as the world changes.
- Whether an accepted action actually produces the intended result.

The resulting headline is:

> **“Observability tells us what changed. Drasi keeps the models of permission, feasibility, and recovery current as it changes.”**

### Important implementation boundaries

The [GPU example](https://github.com/drasi-project/drasi-server/tree/agentofreality-parallel-computation-graph/examples/gpu-cluster-lab) already supplies much of the domain computation, but its working version uses the **embedded host**; its stock-Server version is not yet implemented. Its GPU execution and telemetry are simulated. Adding an agent, Grafana integration, and Langfuse integration would be new work.

Also, the existing example supports live changes to **policy parameters**, not arbitrary hot-reloading of its compiled Rego rule text. Its solver can return **unknown** as well as feasible or infeasible, and execution-time enforcement remains necessary.

Finally, “change-driven” does not mean every algorithm is internally incremental: a transformer may recompute its model and emit only the changed results. The value is hosting that domain computation in the same connected change-processing system—not claiming a universal incremental optimizer.

**OK, this is starting to look pretty interesting. However, does this aproximate a realistic real-world scenario that people in the audience will realate to?**

**The underlying problem is realistic. The literal GPU-placement scenario is more specialized than much of your Ignite audience will relate to.** I would keep the policy, optimization, and resilience models, but present them through **an enterprise AI application using shared model endpoints**, rather than through GPU allocation.

That is a framing change—not a rejection of the computation you already have.

## What is realistic—and what feels constructed?

| Element | Assessment |
|---|---|
| Several AI applications sharing limited capacity | Very credible. Interactive assistants, document processing, and background jobs compete for quotas and throughput. |
| A healthy fallback that cannot process particular data | Credible, given explicit customer agreements, approved-provider lists, or organizational processing policies. |
| Recovery requiring changes to other workloads | Credible: defer batch work or reduce its allocation to protect customer-facing requests. |
| Everything working now but insufficient capacity after a failure | A familiar resilience problem, applied to AI services. |
| Application developers rearranging GPU replicas | Relevant to teams hosting models themselves, but less relatable to developers consuming managed model APIs. |
| An AI operations agent proposing a deliberately bad recovery | Possible, but risks feeling staged—and introduces a second AI story that the presentation does not need. |

The problem is established enough that [Azure API Management’s AI gateway documentation](https://learn.microsoft.com/en-us/azure/api-management/genai-gateway-capabilities) explicitly addresses shared token quotas, multiple applications, backend routing, and preventing one application from consuming everyone else’s capacity.

## A more relatable version

### “Keep the customer assistant working when AI capacity becomes constrained”

A company runs:

- **A customer-support assistant** handling confidential customer records.
- **Background summarization** of public documents.
- Two approved European model deployments and another deployment elsewhere.

Use the same model—or explicitly approved compatible models—so this is not pretending that arbitrary models are interchangeable.

Then:

1. **The primary European endpoint degrades.** Traditional observability detects errors, throttling, or increased latency.
2. **The second European endpoint is healthy but already heavily allocated to background processing.**
3. **Another endpoint has spare capacity, but the customer-support data is not permitted there.**
4. **Drasi determines a valid response:** defer or redirect eligible background requests, reserve the approved capacity for customer support, and identify any demand that still cannot be served.
5. **The gateway applies the approved allocation.** Monitoring shows whether customer requests actually recover—not merely whether a configuration update succeeded.

The audience can understand this immediately:

> “Our customer assistant is struggling. We have spare capacity, but we cannot use all of it for every customer. What can we change without breaking another commitment?”

Importantly, redirect **new requests** or defer queued work. Do not imply that in-flight model executions can be transparently moved between providers.

## Where the transformers matter

The existing GPU example gives you a useful architectural starting point, but the endpoint model would require adaptation.

- **Policy transformer:** maintains which workload/customer classes may use which endpoints.
- **Allocation transformer:** maintains a feasible distribution of limited capacity across competing workloads, respecting priorities and constraints.
- **Resilience transformer:** assesses whether those commitments remain feasible if an endpoint becomes unavailable.
- **Continuous queries:** connect those assessments to affected applications, customers, alerts, and operators.

The most compelling early moment remains:

> **“All endpoints are healthy, but we cannot maintain our critical service if this one fails.”**

That moves the story from reactive monitoring to an actionable assessment of operational risk.

## The important credibility test

**Basic failover is already handled by gateways and load balancers.** API Management, for example, already provides [backend pools and circuit breakers](https://learn.microsoft.com/en-us/azure/api-management/backends). Drasi should not be presented as necessary to switch from endpoint A to endpoint B.

Its contribution becomes credible when the decision requires **changing business policies, competing workloads, shared capacity, and current operational evidence together**.

Also, provider quotas and measured throughput are not guarantees of future capacity or latency. Present the optimizer’s result as **feasible under the stated model and current evidence**, then verify the actual outcome. The gateway must still enforce authorization and rate limits at execution time.

**My recommendation:** make the customer-facing AI assistant the application being observed; do not make an AI operations agent central to the demo. Show the real operational trade-off, let Drasi’s models produce the assessment, and let an operator approve the response. That is more recognizable, less contrived, and keeps the audience focused on Drasi’s contribution.

**Create me a diagram of the demo solution we would build**

[Ignite Demo Solution](drasi-ai-observability-demo.png)

**I would lik to do this presentation with almost no slides. maybe a single intro slide telling people what drasi is and highlighting the role of SOurces, queries, transformers, and reaxctions. Then just spend the rest of the time doing a demo. give me an outline of the presentation flow with that in mind**

**Use one slide to give people a vocabulary, then teach everything else through one evolving incident.** The presentation should feel like operating a system—not touring a collection of tools.

## 1. The single introductory slide — 90 seconds

**Title: Drasi: continuously maintain models and react to change**

A short definition:

> Drasi connects changes in existing systems to continuously updated models, decisions, and actions.

Four components:

| Component | Role |
|---|---|
| **Sources** | Bring changes into Drasi from existing systems. |
| **Queries** | Select, connect, and derive the information that matters. |
| **Transformers** | Maintain stateful models using domain logic—such as policy evaluation or optimization. |
| **Reactions** | Publish results or initiate controlled actions when those results change. |

Your spoken transition:

> “Rather than explain those in isolation, let’s use them to keep a customer-facing AI application working when capacity becomes constrained.”

Briefly identify the computation-graph functionality as **in development**, then leave the slide.

## 2. Presentation flow

| Time | What you do live | What the audience should understand |
|---|---|---|
| **1:30–4:00 — Establish the situation** | Run customer-support requests and background summarization. Show three model endpoints, their current allocations, and the customer’s processing restrictions. Point briefly to Grafana’s health view and a Langfuse trace. | “Two applications share limited AI capacity. They have different priorities and permissions.” |
| **4:00–7:00 — Break the primary endpoint** | Inject throttling or failure. Show the conventional infrastructure alert and the effect on customer requests. Point out that another region has spare capacity—but cannot process this customer’s data. | “Detecting the failure is straightforward. Choosing a valid response is harder.” |
| **7:00–11:00 — Find and apply a valid response** | Show Drasi’s policy decisions and proposed allocation: redirect eligible background work, freeing approved capacity for customer support. Review the reasons, approve the current plan, and watch requests recover. Distinguish proposed, applied, and observed recovery. | “Drasi combines policy and system-wide constraints to maintain an actionable recovery assessment.” |
| **11:00–14:00 — Follow one change through Drasi** | Open the demo’s inspector. Follow the actual failed-endpoint observation through its source, context query, policy and allocation transformers, result query, and reaction. Show a short query and a compact policy fragment—not entire configuration files. | “Those four components from the slide are doing concrete work. The models are not browser-side magic.” |
| **14:00–17:00 — Change policy, not infrastructure** | Restore a stable situation, then change a permitted-region parameter. Keep the health charts visible: no server failed. Show eligibility and allocation assessments updating, and any obsolete proposal losing validity. | “Operational decisions depend on changing business context, not just changing telemetry.” |
| **17:00–20:00 — Show risk while everything is green** | Reduce spare, approved capacity without disrupting currently running work. Show the resilience model reporting that a specified additional endpoint failure would leave insufficient recovery capacity. Restore headroom and watch that assessment change. | “A system can be healthy now but unable to recover. Drasi can maintain that assessment before an incident.” |
| **20:00–22:00 — Return to the application and evidence** | Run another customer request. Show its successful outcome, the infrastructure evidence, and Drasi’s assessment published back into the existing observability experience. Finish with the system still running. | “This complements the tools we already use and closes the loop from observation to verified outcome.” |
| **22:00–25:00 — Reserve** | Questions, unexpected latency, or a deeper look at something the audience responds to. | Avoid rushing the most important moments. |

## 3. Make the demo itself explain the architecture

I would build the main demo interface around **three persistent areas**:

| Area | What stays visible |
|---|---|
| **What is happening** | Request success, endpoint health, current workload allocation, and pending work |
| **What Drasi currently knows** | Policy eligibility, capacity feasibility, resilience, reasons, and freshness |
| **What we can do** | Candidate plan, approval, application status, and recovery confirmation |

Add an expandable **“Follow this change”** inspector containing real input records and intermediate outputs.

That lets you explain architecture without returning to slides. Grafana and Langfuse become brief evidence drill-downs, not separate product demonstrations.

## 4. The three moments to emphasize

### “Healthy does not mean permitted.”

Point to the available endpoint that cannot receive the restricted workload.

This makes the policy transformer’s purpose immediately understandable.

### “Individually reasonable choices may not form a feasible plan.”

Show why moving customer traffic alone does not work, but changing the allocation of background work makes recovery possible.

This makes the optimizer more than a sophisticated-looking routing rule.

### “Working now does not mean recoverable.”

Show the resilience assessment changing while current requests remain successful.

This is the strongest demonstration of functionality beyond conventional threshold monitoring.

## 5. Keep the presentation disciplined

- **Use scenario buttons to change inputs, not fabricate results.** The policy engine, solver, and queries should produce the displayed decisions.
- **State the simulation boundary once, early.** Endpoint behavior and capacities are controlled for the demo; the Drasi computation and integrations are real.
- **Do not introduce a separate AI operations agent.** The customer assistant already supplies the AI story.
- **Show one query, one policy fragment, and one model explanation.** Avoid live editing or scrolling through implementation code.
- **If time slips, shorten the implementation walkthrough or skip the separate policy-change scene.** Preserve the failure, valid recovery, and green-but-fragile moments.

Close while looking at the running system:

> **“Our observability tools tell us what changed. Drasi keeps the models of what that means—and what we can safely do next—up to date.”**

**what about being able to flow between the demo and additional support slides/commentary by using a HTML based slide show built using Reveal.js?**

**Yes—Reveal.js is a good fit for this.** I would build a **presentation shell around the live demo**, rather than a conventional slide deck containing several copies of the demo.

That gives you a primarily hands-on presentation, with explanations available exactly when you need them.

## The audience experience

You start with the one-slide introduction, then enter the live application. During the demonstration, you can briefly open an explanation, inspect a query, or show the architecture—and return to **the same running state**.

| Main presentation position | Live view | Optional supporting content |
|---|---|---|
| **Introduction** | — | What Drasi is; Sources, Queries, Transformers, Reactions |
| **Normal operation** | Applications, endpoints, current allocation | Scenario assumptions and component responsibilities |
| **Endpoint failure** | Infrastructure alert and affected requests | Why a healthy fallback might not be usable |
| **Recovery assessment** | Policy results, feasible plan, approval | Rego policy, optimization constraints, decision explanation |
| **Follow the change** | Actual inputs and intermediate results | Architecture diagram and selected query/component code |
| **Policy change** | Updated eligibility and plan revisions | Why decisions must remain current |
| **Resilience** | Current health versus recovery capability | What the what-if model assumes—and does not guarantee |
| **Closing** | Recovered application and outcome evidence | Three takeaways, if needed |

Reveal’s [vertical slides](https://revealjs.com/vertical-slides/) are designed for optional material: the horizontal route is your main story; vertical branches contain deeper explanations.

**You could finish the entire presentation without opening most of those branches.**

## The important implementation choice

**Mount the demo once and keep it running.**

The page would contain two cooperating parts:

- **Persistent demo application:** owns the UI, live subscriptions, selected entities, and current inspection state.
- **Reveal.js presentation layer:** owns the narrative position, supporting content, transitions, and speaker notes.

Reveal’s [slide-change events](https://revealjs.com/events/) can tell the application to focus on a particular view—for example, policy decisions or resilience—without recreating the application.

I would avoid placing a separately loaded demo iframe on every slide. That risks reconnecting subscriptions, losing selections, or creating multiple application instances.

Most importantly:

> **Changing presentation position must not change the system being demonstrated.**

Moving to “Endpoint failure” should reveal that part of the presentation, not automatically fail an endpoint. Fault injection, approval, and reset remain explicit demo actions.

## Three useful explanation modes

**Full demo:** The application occupies almost the entire screen.

**Demo plus commentary:** A narrow panel explains what to notice while the live system remains visible—for example, “Available capacity is not necessarily permitted capacity.”

**Full explanation:** Temporarily show the architecture or a readable code fragment. A prominent **Return to live demo** control restores the previous view and selection.

The backend should continue running while explanations are open. If you freeze a display to discuss a particular decision, label it as a snapshot rather than implying it is still live.

## Presenter support

Reveal’s [speaker view](https://revealjs.com/speaker-view/) provides private notes and timing. For each scene, I would include:

- The action to perform.
- The result to point out.
- The one sentence the audience should remember.
- A shortcut or fallback if you are behind schedule.

One subtlety: **Space normally walks through both horizontal and vertical slides.** Configure the clicker and keyboard so normal advancement follows your main route and does not accidentally visit all the supporting material.

Also ensure speaker-view previews cannot start another simulator or submit demo commands.

## Make it reliable for Ignite

Serve everything from a local web server, with Reveal.js, fonts, images, and supporting content bundled locally. Keep a clearly labelled replay available as a fallback.

Embed your own demo UI directly. For Grafana and Langfuse, use supported local embedding where practical, otherwise open a prepared view in another tab; do not make the presentation depend on relaxing SaaS framing restrictions.

**This preserves your original goal: one introduction, then a live demonstration—with an explanation layer you can enter and leave without breaking the flow.**

**Is existing observability infrastructure like OTel, grafana, and prometheus still playing a realistic, recognizable, and sufficiently prominant role?**

**They still have realistic roles, but they are not yet sufficiently prominent in the latest presentation flow.** We have drifted toward a Drasi decision-and-recovery demo with observability in the background.

I would correct that by making **Grafana the primary operational view**, with Drasi adding assessments to an observability workflow the audience already recognizes.

## Give each component a substantive job

| Component | Real responsibility | What the audience sees |
|---|---|---|
| **OTel** | Instrument the application and gateway; collect and route metrics and traces with consistent identifiers | A customer request connected to its model calls, errors, latency, and token usage—not a separate synthetic monitoring feed |
| **Prometheus** | Store measurements and support queries for request rate, errors, latency, throttling, queue depth, and utilization | Real measurements before, during, and after the failure |
| **Grafana** | Provide the operational dashboard and technical alerting, backed by Prometheus | The familiar place where operators notice the problem, investigate it, and confirm recovery |
| **Drasi** | Combine operational signals with policies, priorities, demand, and configuration; maintain permission, allocation, and resilience assessments | Explain which responses are permitted and feasible, propose changes, and track their outcome |

**Without Drasi, the existing stack should still detect and help diagnose the problem.** Drasi’s contribution is answering questions that require additional context and domain models.

## Adjust the demo around Grafana

### 1. Start in the existing operational dashboard

Show a recognizable Grafana dashboard with:

- Customer-request success rate and latency.
- Model-endpoint errors and throttling.
- Interactive versus background traffic.
- Queue depth and token consumption.

Run a customer request and briefly open its trace. This establishes that the telemetry is connected to actual application behavior.

### 2. Let the existing monitoring detect the failure

Fail or throttle the primary endpoint.

**Grafana’s alert fires from actual Prometheus measurements.** It sends an alert webhook to Drasi. Do not have the fault-injection button directly create a Drasi incident and merely animate the Grafana charts afterward.

The narrative is:

> “Our existing monitoring has correctly identified the problem. Now we need to decide what to do.”

### 3. Add Drasi’s assessment alongside that evidence

Keep Grafana visible while opening a Drasi assessment panel:

> “The customer-support workload is affected. These destinations are permitted. This healthy destination is not. Current allocations leave insufficient approved capacity, but redirecting background work would make recovery feasible.”

The panel should link back to the relevant dashboard and trace. It augments the investigation rather than moving the audience into an unrelated application.

### 4. Apply the plan—and return to measurements

After approval, the gateway changes how it admits and routes new requests.

Watch the effects in **Grafana**:

- Throttling decreases.
- Interactive latency improves.
- Background traffic shifts or queues.
- Customer requests complete successfully.

A Drasi status saying “plan applied” is not the final proof. **The existing monitoring supplies independent evidence of what happened.**

### 5. Show the green-but-fragile assessment in Grafana too

Add a small section containing Drasi-derived indicators:

- Current allocation feasible.
- Recovery feasible after the specified endpoint loss.
- Assessment current, stale, or unknown.
- Candidate plan awaiting approval.

Show an infrastructure dashboard that is green while the recovery assessment is not. That makes the new capability visible **inside the familiar operational experience**.

Detailed explanations and affected-task lists can open in the Drasi inspector; they do not all need to become Prometheus metrics.

## Keep the integration technically honest

For the initial build, I would choose one straightforward path:

**OTel Collector exposes metrics → Prometheus scrapes them → Grafana queries and alerts → alert webhooks update Drasi.**

Gateway and database changes supply the separate demand, configuration, policy, and task-state inputs. Alert webhooks represent alert transitions; they are **not a continuous capacity-measurement feed**.

Publish Drasi assessments back as Grafana annotations and a few low-cardinality metrics. Keep individual customer/task details out of metric labels.

Also retain normal scraping and alert-evaluation behavior. This should not be a contrived speed contest against Prometheus; the benefit is continuously maintained **cross-system decisions**, not pretending the existing stack never polls.

**My revised presentation rule: start and finish each operational scene in Grafana; open Drasi to explain the assessment; use Langfuse for one focused AI-behavior drill-down.** That gives the established infrastructure a recognizable, necessary, and prominent role.


