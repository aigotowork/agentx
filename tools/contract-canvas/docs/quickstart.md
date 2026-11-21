# Create and publish a project

[Documentation home](../README.md)

## 1. Start with a working model

Copy the entire Contract Canvas directory. From that copy, check the included library example and open [its entry](../example/index.html). Node.js 18 or later is needed only for the commands.

```sh
node build.mjs example ../viewer
```

For a new project, create a `my-project/` directory beside `viewer/`. Use the following two files for a minimal usable model, or copy the library [model](../example/model.json) and [view](../example/view.json) if you want to start with multiple layers and a scenario. Do not copy or edit the example's generated files as inputs.

`my-project/model.json`:

```json
{
  "schemaVersion": 1,
  "id": "bookkeeping",
  "title": "Bookkeeping contracts",
  "authority": "Proposed bookkeeping model",
  "layers": [
    {
      "id": "ledger",
      "name": "Ledger",
      "summary": "Ledger declarations.",
      "status": "proposed",
      "sourceRefs": []
    }
  ],
  "nodes": [
    {
      "id": "Account",
      "name": "Account",
      "kind": "interface",
      "domain": "bookkeeping",
      "layerId": "ledger",
      "provenance": "proposed",
      "summary": "An account in the ledger.",
      "fields": [],
      "ports": [],
      "sourceRefs": []
    }
  ],
  "edges": [],
  "sources": [],
  "scenarios": []
}
```

`my-project/view.json`:

```json
{ "schemaVersion": 1 }
```

## 2. Build and open it

```sh
node build.mjs my-project ../viewer --write
node build.mjs my-project ../viewer
```

`--write` validates the two input files and generates `my-project/index.html` and `my-project/data.js`. The second command checks both the input and the freshness of those two files without writing. Open `my-project/index.html` directly in the browser; the first layer and `Account` should be visible. The method tab is disabled until the model declares a method.

`../viewer` is the **URL relative to the generated HTML**, regardless of where you invoke Node. For example, if the generated page and the viewer assets are siblings under a common root, use `../viewer`; if the assets are in a child folder called `viewer/` next to `index.html`, use `viewer`. Regenerate if the relative location changes.

## 3. Add one real reading path

Pick one question a reader should answer, such as “what does submitting an entry return?” Add the owning object, port and return type, then draw their relationships as exact edges. [Model authoring](model.md) walks through the corresponding library port and which edges the validator expects. Add source evidence where you have it and mark design proposals explicitly. Then choose an optional [view configuration](view.md) for the reader's starting point or a step-by-step walkthrough.

After changing either JSON file, rerun the build with `--write`, refresh the page and follow the intended path through at least one real reference. Run the command without `--write` in CI if you check in the generated files.

## 4. Share the output

For a local copy or a static host, ship the generated HTML and data script **plus the files under the configured viewer URL**. Preserve their relative paths. The JSON inputs and Node builder are useful for authors, but opening a generated page does not need them. For a shared site, deploy the generated page, its data and the compatible renderer together.

For a local HTTP preview, from the common parent directory of `viewer/` and `my-project/`:

```sh
python3 -m http.server 18989 --bind 127.0.0.1
```

Open <http://127.0.0.1:18989/my-project/>. Direct file opening works without Python. The generated script contains the complete dataset, including implementation-visible declarations and source labels. Publish only the data you intend viewers to receive.

| Symptom | Fix |
| --- | --- |
| `stale or missing …; regenerate the static entry` | Run the same command with `--write`, then check again. |
| `unresolved …`, `missing … edge` or `duplicate …` | Check the declaration ID and the exact ownership/typed edge in [Model authoring](model.md). Update view and scenario references when renaming IDs. |
| Page shows no contracts or lacks styling | Verify `data.js` is present beside the generated HTML and that its configured viewer URL reaches the shared CSS and scripts. Refresh after copying all outputs. |
| An optional tab or shortcut is missing | Check whether the model has a method or a scenario, and whether the relevant optional [view binding](view.md) is configured. |
