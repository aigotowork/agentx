/**
 * @aiPurpose Check the upstream consumer APIs affected by development dependency security overrides.
 * @aiUsage Run npm run test:dependencies after installing the locked plugin SDK workspace.
 */
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test } from "node:test";

function consumerRequire(specifier) {
  return createRequire(import.meta.resolve(specifier));
}

function fixture(t, files) {
  const directory = mkdtempSync(join(tmpdir(), "agentx-dependencies-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  for (const [name, content] of Object.entries(files)) {
    const path = join(directory, name);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, content);
  }
  return directory;
}

test("npm workspace discovery preserves include/exclude patterns with the patched matchers", async (t) => {
  const cwd = fixture(t, {
    "packages/one/package.json": JSON.stringify({ name: "@fixture/one", version: "1.0.0" }),
    "packages/two/package.json": JSON.stringify({ name: "@fixture/two", version: "1.0.0" }),
    "packages/excluded/package.json": JSON.stringify({ name: "@fixture/excluded", version: "1.0.0" }),
    "node_modules/hidden/package.json": JSON.stringify({ name: "@fixture/hidden", version: "1.0.0" }),
  });
  const mapWorkspaces = consumerRequire("@npmcli/arborist")("@npmcli/map-workspaces");
  const result = await mapWorkspaces({ cwd, pkg: { workspaces: ["packages/*", "!packages/excluded"] } });
  assert.deepEqual([...result.keys()], ["@fixture/one", "@fixture/two"]);
  assert.equal(result.get("@fixture/one"), join(cwd, "packages/one"));
});

test("OpenCode's glob utility preserves async/sync scans, dot files and directory options", async (t) => {
  const cwd = fixture(t, {
    "src/one.ts": "export {};",
    "src/two.js": "export {};",
    "src/.hidden.ts": "export {};",
    "src/nested/three.ts": "export {};",
  });
  const { Glob } = await import("@opencode/util/glob");
  const options = { cwd, dot: true };
  const expected = ["src/.hidden.ts", "src/nested/three.ts", "src/one.ts", "src/two.js"];
  assert.deepEqual((await Glob.scan("src/**/*.{ts,js}", options)).sort(), expected);
  assert.deepEqual(Glob.scanSync("src/**/*.{ts,js}", options).sort(), expected);
  assert.equal(Glob.match("src/**/*.ts", "src/.hidden.ts"), true);
  assert.equal(Glob.match("src/**/*.ts", "other/one.ts"), false);
  const directories = await Glob.scan("src/*", { cwd, include: "all", absolute: true });
  assert.ok(directories.includes(join(cwd, "src/nested")));
});

test("npm ignore-walk preserves ignore files through the patched Minimatch class", async (t) => {
  const path = fixture(t, {
    ".gitignore": "ignored.txt\nbuild/\n",
    "keep.txt": "kept",
    "ignored.txt": "ignored",
    "build/output.txt": "ignored",
  });
  const walk = consumerRequire("npm-packlist")("ignore-walk");
  const files = await walk({ path, ignoreFiles: [".gitignore"] });
  assert.ok(files.includes("keep.txt"));
  assert.ok(!files.includes("ignored.txt"));
  assert.ok(!files.includes("build/output.txt"));
});

for (const sdk of ["@opencode-ai/plugin", "@kilocode/plugin"]) {
  test(`${sdk}'s Effect consumer retains the TOML namespace parse contract`, () => {
    const effect = consumerRequire(sdk).resolve("effect");
    const toml = createRequire(effect)("toml");
    const result = toml.parse('title = "fixture"\n[service]\nport = 8123\nenabled = true\ntags = ["one", "two"]\n');
    assert.equal(result.title, "fixture");
    assert.equal(result.service.port, 8123);
    assert.equal(result.service.enabled, true);
    assert.deepEqual(result.service.tags, ["one", "two"]);
    assert.throws(() => toml.parse("key = ["));
  });
}

test("the v1 SDKs and Puppeteer's own BiDi dependency retain schema validation", () => {
  const puppeteer = consumerRequire("@oh-my-pi/pi-coding-agent").resolve("puppeteer-core");
  const bidi = createRequire(puppeteer).resolve("chromium-bidi");
  const consumers = [consumerRequire("@opencode-ai/plugin"), consumerRequire("@kilocode/plugin"), createRequire(bidi)];
  for (const require of consumers) {
    const { z } = require("zod");
    const schema = z.object({ session: z.string().min(1), count: z.number().int().min(0) });
    assert.deepEqual(schema.parse({ session: "fixture", count: 2 }), { session: "fixture", count: 2 });
    assert.equal(schema.safeParse({ session: "", count: -1 }).success, false);
  }
});

test("OMP's Puppeteer dependency retains its connection API without starting a browser", async () => {
  const puppeteer = consumerRequire("@oh-my-pi/pi-coding-agent")("puppeteer-core");
  assert.equal(typeof puppeteer.launch, "function");
  await assert.rejects(puppeteer.connect({}), /Exactly one of/);
});

test("Undici retains each upstream consumer's fetch and dispatcher contract", async (t) => {
  const server = createServer((request, response) => {
    response.setHeader("content-type", "application/json");
    response.end(JSON.stringify({ path: request.url, identity: request.headers["x-fixture"] }));
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  t.after(() => new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve())));
  const address = `http://127.0.0.1:${server.address().port}/sdk`;
  for (const consumer of ["@earendil-works/pi-coding-agent", "@opencode/util/glob", "node-gyp"]) {
    const { Agent, fetch } = consumerRequire(consumer)("undici");
    const dispatcher = new Agent({ connections: 1 });
    try {
      const response = await fetch(address, { dispatcher, headers: { "x-fixture": consumer } });
      assert.equal(response.status, 200);
      assert.deepEqual(await response.json(), { path: "/sdk", identity: consumer });
    } finally {
      await dispatcher.close();
    }
  }
});
