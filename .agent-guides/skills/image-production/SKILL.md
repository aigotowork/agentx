---
name: image-production
description: Prepare a repeatable image-production batch from a product brief, explicit visual content, selected styles and asset formats. Includes a local catalog and Python compiler; use before handing jobs to an available image generator.
---

# Prepare image production

Translate the requested visual outcome into an explicit brief before invoking a generator. This Skill prepares local jobs and frozen inputs. It does not call a provider, read credentials or generate images. Real generation uses the consumer's available image capability and its authorization.

1. Identify the audience, task, real visible content and required placements. Distinguish product/behavior facts from what the image should actually depict. Preserve unknowns instead of inventing product data.
2. Browse the three bundled catalogs or use literal keyword suggestions. Compare structural choices such as density, geometry and typography; the catalog is seed knowledge, not validated product art. Keyword matching does not understand negation or aesthetic fit.
3. Create a brief using the [input contract](references/contract.md) and [working example](assets/fitness-brief.json). Each selected format requires its own `render` content/constraints/unknowns; independent assets use explicit subjects and per-style visual constraints. Keep UI behavior and orchestration metadata outside drawing prompts.
4. Run the compiler into a new output directory and inspect `review.md`, prompts and `manifest.json`. Verify same-family consistency, exact content, dimensions and dependencies. A `prepared` job or unbound reference is not a completed image.
5. Hand selected jobs to the actual image executor. Generate required system boards before dependent jobs and bind real outputs as references. Preserve generation receipts, image review and implementation evidence separately from the fixed prepared batch.

Use Python 3.9+ with its standard library. From this Skill directory:

```sh
python3 scripts/design_batch.py catalog
python3 scripts/design_batch.py suggest '清爽 安静 阅读'
python3 scripts/design_batch.py prepare assets/fitness-brief.json --out /tmp/design-batch-new
```

The script can run by absolute path from any working directory. Input/output paths are relative to the caller; catalogs default relative to the script. `--catalog-dir DIRECTORY` goes before the subcommand. Output must not exist and its parent must exist. No environment configuration or network is used; exit 2 reports invalid input or filesystem failure. See [tool usage](references/tool.md) for exact commands, preserved snapshots and replay.

Finish preparation when the reviewable jobs accurately express the requested visual work. Actual image quality, provider dimensions, transparency and UI usability require the later real output and consumer check.
