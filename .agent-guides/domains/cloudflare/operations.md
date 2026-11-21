---
schema: scenario
description: Use project CLI, API and documentation for Cloudflare development and operations
tags: [cloudflare, cli, operations]
---

# Operate Cloudflare from the command line

Use this guide when preparing Cloudflare tools or choosing how to run a task. Start with the project's package manager, locked dependencies, scripts and Cloudflare configuration. Reuse the project's tools and authentication; install a missing dependency only when the selected task needs it.

## Choose the execution path

| Task | Default path | What to establish |
| --- | --- | --- |
| Workers/Pages development, deployment and supported resources | Project-local Wrangler; [official Skill](skills/wrangler/SKILL.md) owns detailed procedure | Project version, build/config source and environment |
| DNS, WAF, Access, Gateway and general account/Zone management | Supported product CLI, otherwise REST API or official SDK | Exact API, account/Zone and resource permissions |
| Repeated declarative infrastructure changes | Existing Terraform/Pulumi project and state | Configuration ownership and reconciliation of out-of-band changes |
| Tunnel connector process and local routing diagnostics | `cloudflared`; inspect its help and [Tunnel docs](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/) | Connector installation, process lifetime and origin reachability |
| Current docs, limits and API syntax | Fetch the relevant official page, Markdown variant or product `llms.txt` index | Current product documentation and installed-version compatibility |
| Runtime logs and deployment diagnostics | Wrangler, product REST/GraphQL APIs or dashboard | Correct environment, time window and retention |
| Browser performance | Existing browser tooling or Lighthouse CLI | Browser setup, requested measurements and actual evidence coverage |

Wrangler is not a CLI for every Cloudflare API. Do not invent commands such as `wrangler dns` or route all account administration through Workers. Use curl or an official SDK for API operations that lack a suitable CLI command. For a multi-step operation, preserve discovery, targeting, execution and result verification even if the transport is one command.

## Project tools and authentication

1. Inspect package scripts, lockfiles, Wrangler version and config. Run the project's existing command; use its package-manager executor when no script exists. If Wrangler is missing and development setup is requested, add it locally using the project's package manager and [installation instructions](https://developers.cloudflare.com/workers/wrangler/install-and-update/).
2. Read `wrangler --help` and the selected command's `--help` through that same executable. Current global help does not prove an older project dependency supports a feature.
3. For remote work, establish account, Zone/Worker/resource and environment from the task and project configuration. Check local versus remote storage/bindings explicitly. Use `wrangler whoami` when account verification is needed; it is a network read, not an offline installation check.
4. Reuse the configured authentication. For interactive development, use `wrangler login` when login is needed and in scope. For CI/API automation, use the documented `CLOUDFLARE_API_TOKEN` and account selection with the minimum required resource permissions. OAuth login for Wrangler does not authenticate a separate curl command.
5. Obtain credentials through the environment or a secret manager, never chat or committed files. Apply the [Turnstile Skill](skills/turnstile-spin/SKILL.md)'s stricter executable and secret-handling procedure when its helpers are involved.

Examples below assume an npm project with Wrangler already installed. Substitute its existing script or package-manager executor; these are task examples, not a batch setup script:

```sh
npx wrangler --help
npx wrangler dev
npx wrangler types
npx wrangler deploy --dry-run
```

When deployment is requested and its target is established, a typical command is `npx wrangler deploy --env staging`. Use the actual project config and environment; framework-generated configuration may require the framework's build/deploy workflow instead. A dry run validates packaging, not permissions or live resource behavior. Reading this guide does not select an account or request a deployment.

## Account and Zone operations

Read the [API reference](https://developers.cloudflare.com/api/) for the exact operation, permissions, account/Zone scope, body and pagination. The bundled [API guide](skills/cloudflare/references/api/README.md) routes to SDK and curl patterns. For repeated application automation, use the project's official Cloudflare SDK; for infrastructure managed as code, follow its existing state and provider workflow.

A read-only Zone lookup can be one curl invocation. This example assumes a token already supplied through the environment and a user-selected domain; it passes the authorization header through stdin rather than printing it or placing its value in process arguments:

```sh
: "${CLOUDFLARE_API_TOKEN:?Supply a scoped token through your environment}"
curl --disable --fail-with-body --silent --show-error --config - <<EOF
url = "https://api.cloudflare.com/client/v4/zones?name=example.com"
header = "Authorization: Bearer ${CLOUDFLARE_API_TOKEN}"
EOF
```

Keep shell tracing disabled for credential-bearing operations. Check both HTTP status and the API's `success`, `errors` and `result_info`; an empty result does not establish a nonexistent resource across other accounts or pages. Select the returned exact resource before a mutation. Follow the operation's documented retry semantics; after a lost mutation response, inspect the resource before deciding whether another attempt is appropriate.

## Diagnostics and builds

Use the project's `wrangler tail` for live Worker logs and `wrangler deployments list` or `wrangler versions list` for deployment history; inspect help for the supported flags and target selection. A live tail cannot recover an earlier incident. Retrieve stored logs, traces and analytics through the relevant product API, [GraphQL Analytics API](https://developers.cloudflare.com/analytics/graphql-api/), or dashboard, subject to enabled retention and permissions.

[Workers Builds](https://developers.cloudflare.com/workers/ci-cd/builds/) is a remote CI workflow. Use its documented API/dashboard for triggers and remote build logs; local `wrangler deploy` does not reproduce a remote build's state or prove that build succeeded. Keep the project's existing CI/configuration owner.

## Browser performance

Use the [web-perf Skill](skills/web-perf/SKILL.md)'s measurement method with the browser/performance tools already available. Its example tool names do not require installing a new client integration. For a CLI audit, follow [Lighthouse CLI](https://github.com/GoogleChrome/lighthouse#using-the-node-cli); it requires a compatible Chrome/Chromium and access to the target URL. For example, in an environment with Lighthouse installed:

```sh
lighthouse http://localhost:3000 --output=json --output-path=./lighthouse-report.json --chrome-flags="--headless"
```

Retain the tested URL, browser, viewport, throttling and report with the consuming task. Lab loading metrics and TBT do not establish field INP or real-user performance. If the task needs interaction traces, authenticated navigation or network inspection, use a suitable existing browser/CDP tool. State any uncollected measurements rather than translating unavailable tool calls into invented results.
