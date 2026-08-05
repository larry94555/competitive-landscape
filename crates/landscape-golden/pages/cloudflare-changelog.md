Changelogs | Cloudflare Docs Skip to content
Docs Directory API SDKs Changelog
Search
Log in
# Changelog
New updates and improvements at Cloudflare.
All products
Product groups
AI
Analytics
Application performance
Application security
Cloudflare One
Consumer services
Core platform
Developer platform
Docs collections
Media
Network security
Privacy
Storage
Products
1.1.1.1 (DNS Resolver)
Access
Agents
AI Crawl Control
AI Gateway
AI Search
Analytics
API Shield
Artifacts
Audit Logs
Billing
Bots
Browser Isolation
Browser Run
Cache / CDN
CASB
Challenges
Cloudflare for SaaS
Cloudflare Fundamentals
Cloudflare Images
Cloudflare Mesh
Cloudflare Network Firewall
Cloudflare One
Cloudflare One Appliance
Cloudflare One Client
Cloudflare Tunnel
Cloudflare Tunnel for SASE
Cloudflare WAN
Cloudflare Web Analytics
Containers
D1
Data Localization Suite
Data Loss Prevention
Digital Experience Monitoring
DNS
Durable Objects
Email security
Email Service
Flagship
Gateway
Go SDK
Hyperdrive
KV
Load Balancing
Log Explorer
Logpush
Logs
Magic Transit
Multi-Cloud Networking
Network Flow
Network Interconnect
Organizations
Pages
Pipelines
Privacy Proxy
Queues
R2
R2 Data Catalog
R2 SQL
Radar
Realtime
Registrar
Resource Tagging
Risk Score
Rules
Sandbox SDK
SDK
Secrets Store
Security Center
Security Overview
Speed
SSL/TLS
Stream
Support
Terraform
Vectorize
WAF
Workers
Workers AI
Workers Analytics Engine
Workers for Platforms
Workers VPC
Workflows
Zaraz
No products found.
View RSS feeds Subscribe to RSS
2026-08-04 Aug 4, 2026
## Agent traces for Think, Flue, and AI SDK instrumented by Agents SDK
Agents Workers
Agent tracing is now available for applications built with the Agents SDK. Traces show each agent turn alongside model calls, tool runs, approvals, token usage, and Workers runtime operations.
Turn on Workers tracing in your Wrangler configuration:
{ "$schema" : "./node_modules/wrangler/config-schema.json" , "observability" : { "traces" : { "enabled" : true } } }
[ observability . traces ] enabled = true
Think and Flue applications emit agent traces automatically. For direct AI SDK calls, wrap the AI SDK namespace once. wrapAISDK() supports AI SDK v6 and v7. This AI SDK v7 example also supplies the agent identity:
import * as ai from "ai" ; import { wrapAISDK } from "agents/observability/ai" ; const tracedAI = wrapAISDK (ai); await tracedAI. generateText ({ model, prompt: "Find an available appointment" , runtimeContext: { agentId: "booking-agent-production" , conversationId: "conversation-123" , }, telemetry: { functionId: "booking-agent" , includeRuntimeContext: { agentId: true , conversationId: true , }, }, });
import * as ai from "ai" ; import { wrapAISDK } from "agents/observability/ai" ; const tracedAI = wrapAISDK (ai); await tracedAI. generateText ({ model, prompt: "Find an available appointment" , runtimeContext: { agentId: "booking-agent-production" , conversationId: "conversation-123" , }, telemetry: { functionId: "booking-agent" , includeRuntimeContext: { agentId: true , conversationId: true , }, }, });
Message and tool payload recording is off by default. Turn it on only when the payloads are safe to store:
const tracedAI = wrapAISDK (ai, { storeMessages: true , storeTools: true , });
const tracedAI = wrapAISDK (ai, { storeMessages: true , storeTools: true , });
Open the Agents tab ↗ in the Cloudflare dashboard to inspect sessions, replay conversations, and view trace waterfalls. For advanced setup, privacy controls, and trace structure, refer to Agent tracing .
2026-08-04 Aug 4, 2026
## Build and deploy Artifacts repos on every push
Artifacts Workflows
You can now run your CI/CD pipeline on your Artifacts repo by defining a CI Workflow with the CI SDK ↗ , automatically triggered on Artifacts push events.
This allows you to:
- Automatically build and deploy application code stored in Artifacts.
- Run linting, type checking, tests, and other checks on every push.
- Reuse dependencies when the lockfile (i.e. pnpm-lock.yaml ) has not changed.
- Stop deployment when a check or build fails.
- Restrict API token access to the deployment step.
- Deploy the output to a Worker or a Workers for Platforms User Worker.
Define your CI steps with @cloudflare/ci . Each ci.runner() spins up an isolated sandbox, and the cache option reuses installed dependencies across each sandboxed step in your CI job.
Point cache.inputs at your lockfile (i.e. pnpm-lock.yaml , bun.lock ), and the install step only runs again when that lockfile changes:
src/index.js js const deps = await ci. runner ({ name: "install" , command: "bun install --frozen-lockfile" , cache: { inputs: [ "package.json" , "bun.lock" ] }, }); await Promise . all ([ deps. runner ({ name: "lint" , command: "bun run lint" }), deps. runner ({ name: "test" , command: "bun run test" }), deps. runner ({ name: "typecheck" , command: "bun run typecheck" }), deps. runner ({ name: "build" , command: "bun run build" }), ]); await deps. runner ({ name: "deploy" , command: "bun wrangler deploy" });
src/index.ts ts const deps = await ci. runner ({ name: "install" , command: "bun install --frozen-lockfile" , cache: { inputs: [ "package.json" , "bun.lock" ] }, }); await Promise . all ([ deps. runner ({ name: "lint" , command: "bun run lint" }), deps. runner ({ name: "test" , command: "bun run test" }), deps. runner ({ name: "typecheck" , command: "bun run typecheck" }), deps. runner ({ name: "build" , command: "bun run build" }), ]); await deps. runner ({ name: "deploy" , command: "bun wrangler deploy" });
To start the Workflow automatically after each push, add a cf.artifacts.repo.pushed trigger to your Wrangler configuration:
{ "triggers" : { "events" : [ { "type" : "cf.artifacts.repo.pushed" , "filter" : { "namespace" : "CI" , "repoName" : "my-repo" , }, "target" : { "scriptName" : "my-ci-worker" , "workflowName" : "ci-workflow" , }, }, ], }, }
[[ triggers . events ]] type = "cf.artifacts.repo.pushed" [ triggers . events . filter ] namespace = "CI" repoName = "my-repo" [ triggers . events . target ] scriptName = "my-ci-worker" workflowName = "ci-workflow"
To learn more, refer to Build and deploy Artifacts repos .
2026-08-04 Aug 4, 2026
## Create Free accounts from the dashboard
Cloudflare Fundamentals
You can now create standalone Free accounts directly from the Cloudflare dashboard using the new Create Account button. This feature is currently available to all users.
When creating a Free account:
- You can create up to 5 Free accounts .
- Your user account must have at least 7 days of tenure to be eligible.
- The account is created immediately and ready to use.
To create a Free account, go to the Cloudflare dashboard ↗ and select Create Account from either the account switcher in the top left (where your account name appears) or from the Accounts page.
#### Limitations
- This feature can only be used to create a Cloudflare Free account. To create an Enterprise Account under your existing contract, please contact Cloudflare Support.
- All users can create a Cloudflare Free account, however, Enterprises wish to restrict this action to only Super Administrators. We will deliver this improvement in a future release.
#### Next steps
After creating your Free account, you can:
- Add a payment method to enable additional Cloudflare products and services.
- Update billing information to manage payment methods, billing address, or tax IDs.
- Review how Cloudflare billing works to understand the billing lifecycle and charge types.
- Assign accounts to an Enterprise Organization to centrally manage multiple accounts from a single dashboard.
2026-08-04 Aug 4, 2026
## WAF Release - 2026-08-04
WAF
This release introduces new rules and updates Microsoft SharePoint RCE alongside enhanced SSRF cloud protection rule actions.
Key Findings
- CVE-2026-50522: An insecure deserialization vulnerability in Microsoft SharePoint Server. This may allow an unauthenticated attacker to execute arbitrary code using crafted requests.
- CVE-2026-66066: An improper input processing vulnerability in Ruby on Rails Active Storage image variant transformations. This may allow an unauthenticated attacker to perform arbitrary file reads and achieve Remote Code Execution (RCE) using maliciously crafted payload requests.
- Generic Cloud Protections: Added improved detection logic targeting Server-Side Request Forgery (SSRF) in cloud-hosted applications.

| Ruleset | Rule ID | Legacy Rule ID | Description | Previous Action | New Action | Comments |
|---|---|---|---|---|---|---|
| Cloudflare Managed Ruleset | ...052b07cf | N/A | Microsoft SharePoint - Remote Code Execution - CVE:CVE-2026-50522 | Log | Block | This is a new detection. |
| Cloudflare Managed Ruleset | ...3a5b40d6 | N/A | Rails - Arbitrary File Read & RCE - CVE:CVE-2026-66066 | Block | Block | This was labeled as File Upload - RCE. |
| Cloudflare Managed Ruleset | ...8242627b | N/A | SSRF - Local | Disabled | - | This detection has been removed. |
| Cloudflare Managed Ruleset | ...743a63ec | N/A | SSRF - Local - 2 - Beta | Disabled | - | This detection has been removed. |
| Cloudflare Managed Ruleset | ...c2e84e2d | N/A | SSRF - Cloud - Beta | Disabled | - | This detection has been removed. |
| Cloudflare Managed Ruleset | ...ab8af26f | N/A | SSRF - Cloud - 2 - Beta | Disabled | - | This detection has been removed. |
| Cloudflare Managed Ruleset | ...25ba9d7c | N/A | SSRF - Cloud | Disabled | Block | We are changing the action for this rule from Disabled to BLOCK |
| Cloudflare Managed Ruleset | ...01a076eb | N/A | SSRF - Local - Beta | Disabled | - | This detection has been removed. |

2026-08-04 Aug 4, 2026
## WAF Release - Scheduled changes for 2026-08-10
WAF

| Announcement Date | Release Date | Release Behavior | Legacy Rule ID | Rule ID | Description | Comments |
|---|---|---|---|---|---|---|
| 2026-08-04 | 2026-08-10 | Log | N/A | ...94f3006b | vBulletin - Remote Code Execution - CVE:CVE-2026-61511 | This is a new detection. |
| 2026-08-04 | 2026-08-10 | Log | N/A | ...098b749e | Version Control - Information Disclosure - Beta | This is a beta detection and will replace the action on original detection "Version Control - Information Disclosure" (ID: ...0550c529 ) |
| 2026-08-04 | 2026-08-10 | Log | N/A | ...d56225d8 | vBulletin - Code Injection - Invalid image format - CVE:CVE-2019-17132 - Beta | This is a beta detection and will replace the action on original detection "vBulletin - Code Injection - Invalid image format - CVE:CVE-2019-17132" (ID: ...8fe9f1c7 ) |

2026-08-04 Aug 4, 2026
## AI agents can debug Workers with local tracing
Workers
wrangler dev and vite dev automatically capture structured OpenTelemetry traces and correlated console logs during local Worker invocations.
#### Debug with AI agents
When the tooling detects an AI agent session, it prints a terminal hint pointing to the Local Explorer API at /cdn-cgi/explorer/api . The API serves an OpenAPI schema and exposes a read-only observability query endpoint for discovering telemetry, querying traces and logs, and inspecting binding state.
The agent can identify the exact failing operation, fix the code, rerun the request, and verify the result. This debug loop requires no deployment or temporary logs.
#### Inspect traces in Local Explorer
Humans can inspect the same traces and correlated console logs in the Local Explorer browser UI. Each trace shows spans, timing, attributes, and errors.
Automatic spans cover handler calls, outbound fetch() calls, and binding calls. Custom spans appear alongside these automatic spans.
For more details, refer to the Local Explorer documentation .
2026-08-03 Aug 3, 2026
## Control authorization cookies for multi-domain Access applications
Access
Cloudflare Access administrators can now control whether a self-hosted application preemptively sets authorization cookies across its public hostnames.
Previously, Access automatically used eager redirects for applications with five or fewer hostnames. Applications with more than five hostnames received cookies as users visited each hostname. Administrators can now choose either behavior, regardless of the number of hostnames.
The new Eager redirect cookie setting is turned on by default for new applications. After a user signs in, Access redirects the browser through each hostname and sets a CF_Authorization cookie. This supports applications that need to make requests across hostnames before the user visits each one.
For applications with many hostnames, the redirect chain can cause sign-in loops in some browsers. Turn off the setting to issue the cookie only when a user visits each hostname.
To configure the setting, refer to Authorization cookie .
2026-08-03 Aug 3, 2026
## Preview: @cloudflare/computer agent runtime
Agents Workers
We're releasing an early preview of @cloudflare/computer ↗ , an open-source agent runtime that gives every agent its own computer. The runtime dynamically orchestrates between fast, efficient isolates and full Linux containers, so the agent always runs on the right compute primitive for the task at hand.
@cloudflare/computer provides a virtual filesystem backed by SQLite, which you can populate from cloud storage, source control, or any files you choose. Agents can read, write, and edit files, run shell commands, and interact with Git repositories. All operations are gated, audited, and observed.
Install the package via npm:
npm install @cloudflare/computer
Instantiate a Workspace inside any Durable Object to give your agent a filesystem and execution runtime:
import { Workspace } from "@cloudflare/computer" ; export class Agent { workspace = new Workspace ({ storage: this .ctx.storage, }); }
Several execution backends are included or you can write your own:
- Isolate runtime — fast, horizontally scalable execution via just-bash and Dynamic Workers, ideal for file manipulation and data processing.
- Container runtime — full Linux environment via Cloudflare Containers, mounted through FUSE, for tasks that need native binaries, package managers, or a complete userland.
The AI SDK-compatible toolkit provides common agent tools ( read , write , edit , ls , exec ) and guides the model to choose the appropriate backend for each task.
For more examples, including a step-by-step tutorial, visit the @cloudflare/computer repository ↗ .
Read the announcement blog post for more details: Your agent needs a computer, not a container ↗ .
2026-08-03 Aug 3, 2026
## Billing is now enabled for Pipelines
Pipelines
Billing is now enabled for Cloudflare Pipelines on non-enterprise accounts. Pipelines usage beyond the included free tier will appear on your next invoice.
Pipelines charges based on two usage dimensions. Ingress into a Pipeline stream remains free regardless of volume:
- SQL transforms : $0.04 / GB for stateless transforms (filter, reshape, unnest, cast, compute).
- Sinks (egress) : $0.03 / GB for JSON output, $0.06 / GB for Parquet or Iceberg output.
Workers Paid plans include 50 GB / month for both SQL transforms and sinks. Standard R2 storage and operations charges apply for data written to R2 buckets, and R2 Data Catalog charges apply when writing to Iceberg tables.
For example, a pipeline that ingests 500 GB of event data per month, uses a SQL transform to filter and reshape it, and writes 300 GB to an R2 Data Catalog Iceberg table would be billed as follows:

| Dimension | Usage | Included | Billable | Cost |
|---|---|---|---|---|
| Streams | 500 GB | Unlimited | 0 GB | $0.00 |
| SQL transforms | 500 GB | 50 GB | 450 GB | $18.00 |
| Sinks (Iceberg) | 300 GB | 50 GB | 250 GB | $15.00 |
| Total |  |  |  | $33.00 |

For full pricing details and billing examples, refer to Pipelines pricing .
2026-08-03 Aug 3, 2026
## Billing is now enabled for R2 SQL
R2 SQL
Billing is now enabled for R2 SQL on non-enterprise accounts. R2 SQL usage beyond the included free tier will appear on your next invoice.
R2 SQL charges based on a single dimension:
- Data scanned : $0.0025 / GB ($2.50 / TB) of compressed data read from R2 to execute your query.
All plans include 10 GB of data scanned per month. Each query is billed for a minimum of 10 MB of data scanned. R2 SQL pricing is additive to standard R2 storage and operations and R2 Data Catalog charges. R2 does not charge for egress, so there is no additional data transfer cost.
For example, a user who stores 500 GB of Parquet data in R2 Data Catalog and runs queries that scan a total of 50 GB of compressed data during the month would be billed as follows:

| Dimension | Usage | Included | Billable | Cost |
|---|---|---|---|---|
| R2 storage | 500 GB-month | 10 GB-month | 490 GB-month | $7.35 |
| R2 SQL (data scanned) | 50 GB | 10 GB | 40 GB | $0.10 |
| Total |  |  |  | $7.45 |

For full pricing details and billing examples, refer to R2 SQL pricing .
2026-08-03 Aug 3, 2026
## Billing is now enabled for R2 Data Catalog
R2
Billing is now enabled for R2 Data Catalog on non-enterprise accounts. R2 Data Catalog usage beyond the included free tier will appear on your next invoice.
R2 Data Catalog charges based on two dimensions, in addition to standard R2 storage and operations :
- Catalog operations : $9.00 / million operations for metadata requests such as creating tables, reading table metadata, and updating table properties.
- Compaction : $0.005 / GB processed and $2.00 / million objects processed. These charges only apply when automatic compaction is turned on for a table.
Each dimension includes a monthly free tier: 1 million catalog operations, 10 GB of compaction data processed, and 1 million compaction objects processed.
For example, a single Iceberg table with 50 GB of data, 500,000 catalog operations per month, and compaction turned on that processes 20 GB across 200,000 files would be billed as follows:

| Dimension | Usage | Included | Billable | Cost |
|---|---|---|---|---|
| Catalog operations | 500,000 | 1,000,000 | 0 | $0.00 |
| Compaction (data processed) | 20 GB | 10 GB | 10 GB | $0.05 |
| Compaction (objects) | 200,000 | 1,000,000 | 0 | $0.00 |
| Total (Data Catalog) |  |  |  | $0.05 |

Standard R2 storage charges ($0.015 / GB-month) apply separately for the 50 GB of data stored.
For full pricing details and billing examples, refer to R2 Data Catalog pricing .
2026-08-03 Aug 3, 2026
## Python and JavaScript Workers can now call each other via RPC
Workers
You can now call methods between Python and JavaScript Workers using Workers RPC . This works through Service bindings without extra dependencies, schema definitions, or serialization code.
Cross-language RPC calls behave like ordinary function calls. Exceptions propagate to the call site. You can pass structured cloneable types ↗ as parameters or return values, and Pyodide Foreign Function Interface (FFI) automatically converts types between languages.
#### Call a TypeScript Worker from Python
Define a method in a TypeScript Worker:
index.js js import { WorkerEntrypoint } from "cloudflare:workers" ; export class RpcService extends WorkerEntrypoint { async add ( a , b ) { return a + b; } }
index.ts ts import { WorkerEntrypoint } from "cloudflare:workers" ; export class RpcService extends WorkerEntrypoint { async add ( a : number , b : number ) : Promise &#x3C; number > { return a + b; } }
Call it from a Python Worker through a Service binding:
from workers import Response, WorkerEntrypoint class Default ( WorkerEntrypoint ): async def fetch (self, request): rpc = self .env. RPC result = await rpc.add( 42 , 144 ) return Response.json({ "result" : result})
Configure the Service binding in the Python Worker's Wrangler configuration:
{ "services" : [ { "binding" : "RPC" , "service" : "ts-rpc-server" , "entrypoint" : "RpcService" } ] }
[[ services ]] binding = "RPC" service = "ts-rpc-server" entrypoint = "RpcService"
#### Call a Python Worker from JavaScript
Define a method in a Python Worker:
from workers import WorkerEntrypoint class Default ( WorkerEntrypoint ): async def highlight_code (self, code: str , language: str ) -> dict : from pygments.formatters import HtmlFormatter from pygments import highlight from pygments.lexers import get_lexer_by_name lexer = get_lexer_by_name(language, stripall = True ) formatter = HtmlFormatter( linenos = True , cssclass = "highlight" , style = "monokai" ) highlighted_html = highlight(code, lexer, formatter) css = formatter.get_style_defs( ".highlight" ) return { "html" : highlighted_html, "css" : css }
Call it from a JavaScript Worker through a Service binding:
index.js js export default { async fetch ( request , env ) { const rpc = env. PYTHON_RPC ; const result = await rpc. highlight_code ( "print(42)" , "python" ); return Response. json (result); }, };
index.ts ts export default { async fetch ( request , env ) { const rpc = env. PYTHON_RPC ; const result = await rpc. highlight_code ( "print(42)" , "python" ); return Response. json (result); }, };
Configure the Service binding in the JavaScript Worker's Wrangler configuration:
{ "services" : [ { "binding" : "PYTHON_RPC" , "service" : "py-rpc-server" } ] }
[[ services ]] binding = "PYTHON_RPC" service = "py-rpc-server"
For more details on the announcement, read the blog post ↗ .
For more information, refer to the Workers RPC documentation and the Python Workers overview .
2026-07-31 Jul 31, 2026
## Cloudflare One Client for Windows (version 2026.7.1210.1)
Cloudflare One Client
A new Beta release for the Windows Cloudflare One Client is now available on the beta releases downloads page .
This beta release includes the following changes and improvements:
- Improved connection reliability: the client now swaps protocol order after repeated connectivity-check failures, which helps when HTTP/3 is blocked after the QUIC handshake.
- Fixed issue where a certificate error could be incorrectly displayed right after the connection is established.
- A DNS search domain parsing failure no longer prevents connection.
- Fixed a MASQUE issue where the tunnel could stall while uploading at a high rate.
- Fixed being unable to switch organizations when the client was stuck in the "Device not in organization" state.
- Fixed the Home Screen dropdown popup not anchoring correctly.
- Fixed a crash during dialog dismissal.
- Increased tolerance for configurations with a large number of local domain fallback resolver IPs, so DNS resolution behaves correctly even when more fallback resolvers are configured than recommended.
- Fixed a networking issue where IPv6 multicast routes were being assigned to the WARP tunnel interface.
- Fixed fatal errors on UI load on Windows 10.
- Fixed a crash during Windows notification initialization.
- Made the Windows domain-joined posture check more reliable.
- Fixed orphaned credentials left behind on multi-user uninstall.
- A successful re-authentication will cause the device profile to be re-evaluated.
- Improved dashboard-managed client updates by running the updater only when needed.
2026-07-31 Jul 31, 2026
## Cloudflare One Client for macOS (version 2026.7.1210.1)
Cloudflare One Client
A new Beta release for the macOS Cloudflare One Client is now available on the beta releases downloads page .
This beta release includes the following changes and improvements:
- Improved connection reliability: the client now swaps protocol order after repeated connectivity-check failures, which helps when HTTP/3 is blocked after the QUIC handshake.
- Fixed issue where a certificate error could be incorrectly displayed right after the connection is established.
- A DNS search domain parsing failure no longer prevents connection.
- Fixed a MASQUE issue where the tunnel could stall while uploading at a high rate.
- Fixed being unable to switch organizations when the client was stuck in the "Device not in organization" state.
- Fixed the Home Screen dropdown popup not anchoring correctly.
- Fixed a crash during dialog dismissal.
- Increased tolerance for configurations with a large number of local domain fallback resolver IPs, so DNS resolution behaves correctly even when more fallback resolvers are configured than recommended.
- Fixed the WARP client stealing window focus (for example, during reauth).
- Fixed a client crash when connecting to a captive portal over Wi-Fi.
- Fixed the system tray icon showing "disconnected" while the UI showed "connected".
- A successful re-authentication will cause the device profile to be re-evaluated.
- Improved dashboard-managed client updates by running the updater only when needed.
2026-07-31 Jul 31, 2026
## Static OAuth client credentials for MCP server portals
Access
MCP server portals can now connect to upstream MCP servers that require a pre-registered OAuth client. This supports OAuth providers that do not offer Dynamic Client Registration or have disabled it. This unlocks portal connections to major SaaS providers such as Slack and GitHub, whose MCP servers do not yet support DCR.
When adding an MCP server, administrators can enter the client ID and client secret from an OAuth application registered with the upstream provider. The configuration also supports custom OAuth endpoints, scopes, and the client_secret_post and client_secret_basic token endpoint authentication methods.
Cloudflare stores the client secret encrypted. Users still authenticate to the upstream server with their own accounts when they connect through a portal.
For setup instructions, refer to Configure manual OAuth credentials .
2026-07-31 Jul 31, 2026
## Browser Run adds a Playground to the Cloudflare dashboard
Browser Run
Browser Run now includes a Playground in the Cloudflare dashboard. Use it to try Quick Actions against a live browser without creating a Worker, installing an SDK, or deploying code first.
The Playground helps you test a target URL or raw HTML input, tune viewport and page-load settings, preview the output, and copy working code for the same request.
With the Playground, you can:
- Capture visuals as screenshots or PDFs .
- Generate multiple output formats in one request with the snapshot endpoint .
- Extract HTML , Markdown , links , or scraped data .
- Extract structured data with AI using a prompt and optional JSON Schema.
You can also configure desktop, laptop, tablet, mobile, or custom viewport sizes, set browser scale, choose page-load conditions, set timeouts, and wait for selectors before running a request.
Select Show Code to generate the same request as cURL, TypeScript SDK, Python, or Workers Binding code. For example, a screenshot request can be copied as a Workers Binding call:
interface Env { BROWSER : BrowserRun ; } export default { async fetch ( request , env ) : Promise < Response > { return await env. BROWSER . quickAction ( "screenshot" , { url: "https://developers.cloudflare.com" , viewport: { width: 1920 , height: 1080 , }, }); }, } satisfies ExportedHandler < Env >;
Requests made in the Playground incur Browser Run charges . AI extraction also incurs Workers AI charges.
To try the Playground, go to Browser Run in the Cloudflare dashboard and select Playground .
Go to Browser Run &#8599;
For more information, refer to the Quick Actions documentation .
2026-07-31 Jul 31, 2026
## Rotate Stream broadcast keys for live inputs
Stream
You can now rotate the broadcast credentials for a Stream live input without changing the live input identifier.
Use key rotation when live input credentials may have been shared with the wrong audience, exposed in client code or a screenshare, or need to be refreshed as part of your security process. Rotating keys revokes the old credentials, disconnects broadcasts using stale credentials, and returns refreshed credentials in the API response.
To rotate keys for a live input, make a POST request to the rotate_keys endpoint:
curl --request POST \ https://api.cloudflare.com/client/v4/accounts/{ account_id}/stream/live_inputs/ {live_input_identifier} /rotate_keys \ --header "Authorization: Bearer <API_TOKEN>"
Live input responses now also include keysRotatedAt , which indicates when the live input keys were last rotated. This field is omitted for live inputs whose keys have never been rotated.
For endpoint details, refer to Rotate keys for a live input . For usage guidance, refer to Manage live inputs .
2026-07-31 Jul 31, 2026
## Inspect Worker startup performance with Wrangler
Workers Durable Objects
wrangler check startup now reports your Worker's raw and compressed bundle sizes. It also summarizes local CPU activity during startup directly in your terminal.
Large bundles and costly startup work can introduce cold-start latency, so use this command to find code and large dependencies that slow your Worker before it handles requests.
The summary includes sampled, active, garbage collection, and idle time. Wrangler continues to save a .cpuprofile file for detailed flamegraph analysis in Chrome DevTools or VS Code.
⛅️ wrangler 4.116.0 ─────────────────────────────────────────────── ├ Building your Worker │ Worker Built! 🎉 │ ├ Analysing │ Startup phase analysed │ │ Bundle: 7171.25 KiB / gzip: 2197.00 KiB │ │ Local startup profile: │ Profile window: 70.3 ms │ Sampled time: 70.3 ms │ Active: 38.5 ms (including 3.7 ms garbage collection ) │ Idle: 31.8 ms │ Samples: 36 │ │ CPU Profile has been written to worker-startup.cpuprofile. Load it into the Chrome DevTools profiler (or directly in VSCode ) to view a flamegraph. │ │ Note that the CPU Profile was measured on your Worker running locally on your machine, which has a different CPU than when your Worker runs on Cloudflare. │ │ As such, CPU Profile can be used to understand where time is spent at startup, but the overall startup time in the profile should not be expected to exactly match what your Worker's startup time will be when deploying to Cloudflare.
The profile runs locally, so its duration will differ from startup time on Cloudflare. For authoritative startup time, deploy your Worker or upload a version.
Available in Wrangler version 4.116.0 or later. For more information, refer to wrangler check startup .
2026-07-30 Jul 30, 2026
## Use AI Search with the Agents SDK, AI SDK, and LangChain
AI Search
You can now use AI Search directly from popular agent frameworks, adding grounded retrieval to an existing app instead of calling the REST API by hand. The new Agents section has guides for the Vercel AI SDK , LangChain , and the Cloudflare Agents SDK . The AI SDK integration is a new package, and the LangChain integration is a new retriever in the existing langchain-cloudflare package.
#### Vercel AI SDK
The ai-search-provider ↗ package connects AI Search to the AI SDK, and targets AI SDK v6 ( ai@^6 ). Pass instance.chat() to generateText or streamText to generate a response grounded in your indexed content, with the retrieved chunks returned as sources . You can also expose instance.search() as a tool for agent loops.
import { createAISearchNamespace } from "ai-search-provider" ; import { generateText } from "ai" ; const aiSearch = createAISearchNamespace ({ binding: env. AI_SEARCH }); const { text , sources } = await generateText ({ model: aiSearch. get ( "knowledge-base" ). chat (), messages: [{ role: "user" , content: "How does caching work?" }], });
import { createAISearchNamespace } from "ai-search-provider" ; import { generateText } from "ai" ; const aiSearch = createAISearchNamespace ({ binding: env. AI_SEARCH }); const { text , sources } = await generateText ({ model: aiSearch. get ( "knowledge-base" ). chat (), messages: [{ role: "user" , content: "How does caching work?" }], });
#### LangChain
The langchain-cloudflare package ( PyPI ↗ , GitHub ↗ ) provides CloudflareAISearchRetriever , a standard LangChain retriever backed by AI Search. Use it on its own, wrap it with create_retriever_tool to give an agent a search tool, or drop it into a RAG chain. It works with REST credentials or a Worker binding inside a Python Worker.
from langchain_cloudflare import CloudflareAISearchRetriever retriever = CloudflareAISearchRetriever( account_id = ACCOUNT_ID , api_token = API_TOKEN , instance_name = "knowledge-base" , retrieval_type = "hybrid" , ) docs = retriever.invoke( "How do I configure Workers AI?" )
#### Cloudflare Agents SDK
The Cloudflare Agents SDK could already reach AI Search through the Workers binding. The new guide walks through building a stateful chat agent that provisions its own instance, indexes content, and searches it from a tool.
import { tool } from "ai" ; import { z } from "zod" ; const instance = env. AI_SEARCH . get ( "knowledge-base" ); // Expose AI Search to the agent's model as a tool it can call. const searchKnowledgeBase = tool ({ description: "Search the knowledge base for relevant content." , inputSchema: z. object ({ query: z. string () }), execute : ({ query }) => instance. search ({ query }), });
import { tool } from "ai" ; import { z } from "zod" ; const instance = env. AI_SEARCH . get ( "knowledge-base" ); // Expose AI Search to the agent's model as a tool it can call. const searchKnowledgeBase = tool ({ description: "Search the knowledge base for relevant content." , inputSchema: z. object ({ query: z. string () }), execute : ({ query }) => instance. search ({ query }), });
For the full walkthroughs, including creating an instance and indexing content, refer to the Agents guides.
2026-07-30 Jul 30, 2026
## Node.js 24 is now the default for Workers Builds
Workers
Workers Builds now uses Node.js 24.18.0 by default. The build image preinstalls Node.js 22.23.2 and 24.18.0.
You can continue to override the default with the NODE_VERSION environment variable, an .nvmrc file, or a .node-version file. For more information, refer to Override default versions .
2026-07-29 Jul 29, 2026
## WAF Release - 2026-07-29
WAF
This release introduces new rules and updates existing threat signatures to provide targeted protections for vulnerabilities in Nuxt Server Island components and Alibaba Fastjson deserialization routines, alongside enhanced protections for cloud metadata Server-Side Request Forgery (SSRF) and obfuscated command injection attempts.
Key Findings
-
Nuxt Server Island - RCE(GHSA-9473-5f9j-94wq): An unauthenticated vulnerability in Nuxt Server Islands where remote attackers can supply arbitrary component names or props to endpoints. Manipulating these parameters allows unauthenticated component Remote Code Execution (RCE) on the server.
-
Alibaba Fastjson JSONType Remote Code Execution: A unauthenticated remote code execution vulnerability in Alibaba Fastjson (≤ 1.2.83) during JSON deserialization. Under default configurations, attackers can execute arbitrary system commands, bypassing traditional classpath and gadget-based defenses.
-
Generic Protections (SSRF & Command Injection): Added improved detection logic targeting Server-Side Request Forgery (SSRF) in cloud-hosted applications, alongside new rules targeting obfuscated command injection patterns across request parameters.

| Ruleset | Rule ID | Legacy Rule ID | Description | Previous Action | New Action | Comments |
|---|---|---|---|---|---|---|
| Cloudflare Managed Ruleset | ...c2e84e2d | N/A | SSRF - Cloud - Beta | Log | Block | This is an improved detection. |
| Cloudflare Managed Ruleset | ...761e7a4c | N/A | Command Injection - Obfuscation | Log | Block | This is a new detection. |
| Cloudflare Managed Ruleset | ...7347c892 | N/A | Alibaba Fastjson JSONType Remote Code Execution - Body | Log | Block | This is a new detection. |
| Cloudflare Managed Ruleset | ...8ec012ea | N/A | Nuxt Server Island - RCE | N/A | Block | This is a new detection.This was labeled as Generic Rules - RCE. |
| Cloudflare Managed Ruleset | ...3590a4ad | N/A | Generic Rules - RCE | N/A | Block | This is a new detection. |
| Cloudflare Managed Ruleset | ...9c6dff1c | N/A | Generic Rules - XSS | N/A | Block | This is a new detection. |
| Cloudflare Managed Ruleset | ...3a5b40d6 | N/A | File Upload - RCE | N/A | Block | This is a new detection. |
| Cloudflare Free Ruleset | ...cfe1a93c | N/A | Generic Rules - RCE | N/A | Block | This is a new detection. |
| Cloudflare Free Ruleset | ...9ab5ed95 | N/A | Generic Rules - XSS | N/A | Block | This is a new detection. |
| Cloudflare Free Ruleset | ...1b7f9c67 | N/A | File Upload - RCE | N/A | Block | This is a new detection. |

2026-07-28 Jul 28, 2026
## Improved DoH JSON formatting for additional record types
1.1.1.1 (DNS Resolver)
Cloudflare is rolling out updated formatting for the data field in the 1.1.1.1 DoH JSON API ( application/dns-json ). During the roll out responses may use either the old or new format.
Note
These are breaking changes. The DoH JSON format has no formal RFC and its schema is not guaranteed to be stable. If you need a stable format, use the DoH wireformat instead.
#### Human-readable display for additional record types
Several record types previously returned their data field in RFC 3597 ↗ generic hex encoding ( \# <length> <hex> ). These now use standard presentation format:
CAA: 0 issue "letsencrypt.org" NAPTR: 100 10 "s" "SIP+D2U" "" _sip._udp.example.com. RP: admin.example.com. txt.example.com. IPSECKEY: 10 1 2 192.0.2.1 AwEA... SVCB: 1 target.example.com. alpn=h2 HTTPS: 1 . alpn=h3,h2 ipv4hint=192.0.2.1 TLSA: 3 1 1 aabbccdd... SSHFP: 1 2 aabbccdd... OPENPGPKEY: AwEA...
#### Numeric DNSSEC algorithm identifiers
DNSSEC-related records now use numeric algorithm identifiers as defined in RFC 4034 ↗ instead of mnemonic names. This affects RRSIG , DS , CDS , DNSKEY , and CDNSKEY records. For example, RSASHA256 becomes 8 , ECDSAP256SHA256 becomes 13 , and ED25519 becomes 15 . DS digest types also change from mnemonic to numeric: SHA-256 becomes 2 .
Before txt RRSIG: A RSASHA256 2 300 ... DS: 12345 RSASHA256 SHA-256 aabb... DNSKEY: 257 3 RSASHA256 AwEA... After txt RRSIG: A 8 2 300 ... DS: 12345 8 2 aabb... DNSKEY: 257 3 8 AwEA...
#### Other formatting changes
HINFO character-strings are now individually quoted to remove ambiguity when values contain spaces:
Before txt "data": "Intel Xeon Linux" After txt "data": "\"Intel Xeon\" \"Linux\""
2026-07-28 Jul 28, 2026
## Cloudflare MCP servers support the new MCP 2026-07-28 Specification
Agents Workers
Cloudflare's product-specific MCP servers now support the new MCP 2026-07-28 Specification. Each request runs on a fresh stateless server without an MCP protocol session or protocol-specific Durable Object.
The /mcp endpoint also accepts stateless requests from 2025 Streamable HTTP clients. Most clients can reconnect without configuration changes.
Use /mcp for new connections. Historical /sse URLs continue to work as aliases for the same Streamable HTTP handler, but they no longer serve the deprecated HTTP+SSE transport. If a client forces SSE transport, change it to Streamable HTTP or automatic transport detection.
2026-07-28 Jul 28, 2026
## Browser Run adds structured handoff for Human in the Loop
Browser Run
Browser Run now supports structured handoff for Human in the Loop workflows. Using Cloudflare-specific CDP commands , your agent can signal that it needs help, a human steps in through Live View to handle the task, and the agent resumes once the work is done.
For agents running multi-step browser workflows, a single login wall or unexpected prompt can fail the entire run. Previously, scripts had to manage human intervention manually by sharing a Live View URL and polling for completion. Structured handoff replaces this with a formal pause-and-resume flow.
The following example requests human intervention for a login page and waits for the human to finish before continuing:
const cdp = await page. createCDPSession (); // Get Live View URL for the human operator const { devtoolsFrontendUrl } = await cdp. send ( "Cloudflare.getLiveView" , { mode: "tab" , }); console. log ( `Human input needed: ${ devtoolsFrontendUrl }` ); // Request human intervention and wait for completion const handoffComplete = new Promise (( resolve ) => { cdp. once ( "Cloudflare.handoffComplete" , resolve); }); await cdp. send ( "Cloudflare.handoff" , { instructions:
