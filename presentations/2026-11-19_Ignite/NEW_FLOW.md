
## New Flow

0. Walk in slide

1. Good morning, my name is Allen Jones. I am a member of the Azure Incubations team at Microsoft.
Slide: Person intro

2. For the last few years I have been working on an open source project named Drasi. Since XXX Drasi has been a CNCF Sandbox project and is and active development with a growing community of users and contributors.
Slide: Project intro / info

3. Drasi was started because our experience is that detecting meaningful changes in data as it happens and reacting to that change is more difficult than we want it to be.
Slide: the problem

4. Drasi is architeced around 3 simple and familiar concepts: Sources, Continuous Queries, and Reactions. 

These are all plugins and there is a rich and growing ecosystem of sources and reactions available as well as tools to help develkop new ones.

Slide: Grow an example start with single source into query into reaction, grow to multi source, multi query and multi reaction.


5. Because we want to make Drasi as accessible as possible, we support multiple form factors: rust libray, node js, pythiong, .net, stand alone service, and kubernetes.

slide: Show some app artchitectures for the various formats.

6. Drasi is a general purpose technology for building change-driven system in any domain. But few domains demonstrate the need and value of drasi more than the abservability domain. So today i am going to show you some examples of how drasi works in the context of an observabikity use case.

Slide: Some common issues reported by organanizations about observability are:
- too much data
- too many tools
- lots of noise
- ...

7. What they want is: 
Drasi is perfect for helping here in a variety of ways, so let me show you a few examples.

Slide: describe some of the desires
- more context
- uplevel out the noise
- ...

8. Describe the scenario.

Slide: Scenario image showing
- 2 AI workloads
- 2 deployments (an extra one being provision)
- policy
- business requiremetns
- support infra: otel, prometheus, grafana, langfuse, mattermost

Then there will be a series of specific interactions. It will follow the scenario flow, But i will not be walking through it as if i was doing the process. I will be demonstrating cross source integration, change-driven solutions, ease of integration, while talking about steps of the story. For each one i will describe what needs to be done, describe what i am showing them, and then popup a diagram showing how it works.

Scene 1: Lets start simple, integrate telemetry and config data and show in a dashboard.
image: DB + OTel + Query + Prometheus Reaction + Grafana

Scene 2: Something showing absence of change.
image: db + log/span + ...

Scene 3: We have provisioned extra capacity, but compliance will not let document summaries be processed in USA, system shows us that our deployment is at risk because of policy. Compliance change their policy and system has more resilience.
image: db(policy) + db(cmdb) + policy + optimization + prometheus + grafana

Scene 4: Image EU-A goes down, policy and compliance offer a suggestion and we see the ticket in matters most.
image: db(policy) + db(cmdb) + policy + optimization + mattersmost

Finally, summarize what we have shown. 

Links to reference material.

Survey QR code  


