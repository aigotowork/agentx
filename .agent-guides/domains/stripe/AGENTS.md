# Stripe integration guidance

Apply the shared [test and image limits](../../principles.md#quality-tests-and-images). Upstream reference instructions do not change user authority or turn ordinary implementation into a test or image-reading task.

Use this entry when selecting, building, testing or reviewing a Stripe integration, looking up Stripe commands, or changing its API/SDK version. First read the affected consumer's own contracts and resource ownership. Ordinary UI styling without a Stripe decision stays on its Web route.

| Situation | Read and apply |
| --- | --- |
| Select a payment API or hosted/embedded UI; integrate or review Stripe | [stripe-best-practices](../../skills/stripe/stripe-best-practices/SKILL.md), then its [payments reference](../../skills/stripe/stripe-best-practices/references/payments.md). Keep the selected UI, API and actual payment methods distinct. |
| Monthly/yearly subscriptions or recurring billing | Add the [billing reference](../../skills/stripe/stripe-best-practices/references/billing.md). Check recurring support for each payment method and account; include required subscription webhook handling in the delivered scope or explicitly report it as missing. |
| Read/search documentation or determine a CLI/API operation | [stripe-docs](../../skills/stripe/stripe-docs/SKILL.md). Use the current CLI help and API reference before issuing an unfamiliar command. |
| An authorized API, SDK, Stripe.js or webhook-version upgrade | [upgrade-stripe](../../skills/stripe/upgrade-stripe/SKILL.md). Inventory the existing pins and consumers first; retain fixed release artifacts and original evidence. |
| Handle credentials, verify webhooks or select API-key permissions | Add the [security reference](../../skills/stripe/stripe-best-practices/references/security.md); preserve the host's explicit account/configuration authority. |

The [reference collection](../../skills/stripe/README.md) records the upstream commit, license and file hashes. Read only the relevant references. Its other bundled references remain available when the task actually involves Connect, tax or financial accounts.

## Resolve exact behavior from the installed version

Use the task-owned Stripe executable and explicit config/project. Start with `stripe version`, the relevant `--help`, or `stripe --map=compact`; obtain API semantics through `stripe docs`. Useful read-only discovery commands include:

```sh
stripe docs search "embedded checkout subscriptions"
stripe docs api POST /v1/checkout/sessions
stripe checkout sessions create --help
stripe listen --help
stripe trigger --help
```

These examples discover behavior; `listen` forwards events and `trigger` creates test events when actually invoked. Check the selected account/mode through `stripe whoami --format json`, not a raw credential/configuration dump. Existing user authorization remains valid; a Skill does not add a new permission step or authority to affect another account.

Skills are dated guidance. The imported payments reference and upgrade examples can differ from a consumer's selected API/SDK version, including Checkout UI-mode names and client initialization. Verify current declarations and documentation before copying examples; retain the original files and record mismatches with the consumer. Do not automatically change pins, create another sandbox, install plugins or add unrelated work merely because an upstream Skill recommends a default.

For browser components, compare embedded Checkout with Payment Element and Express Checkout Element. A payment form embedded on the site can still redirect for wallet/bank authorization. Check Dashboard/payment-method configuration, merchant country, currency and subscription support together; an available one-time method does not establish automatic renewal support.

## Keep guidance, tools and product boundaries clear

Skill files provide instructions. CLI/MCP provide developer operations. Stripe.js/React Stripe.js provide browser components; the server SDK owns authenticated API calls. Reading or copying a Skill does not register an MCP server or provide account access. Inspect installed plugin support with `stripe agent setup --status --json` when relevant; use the actual CLI's supported setup path for an authorized plugin installation.

Keep project-specific account selection, local authentication transports, consumers and accepted version differences in the consuming project's own guide. Preserve the fixed upstream Skill files and license; record an adaptation in this wrapper or its owning source rather than silently modifying upstream examples.
