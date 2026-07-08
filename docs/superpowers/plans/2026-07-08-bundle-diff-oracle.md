# Bundle Diff Oracle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Byte-diff the output of big bundled CLI tools (tsc, prettier, sass, rollup) run before vs. after minifying their production bundles in place with oxc main.

**Architecture:** A `bundle-tools.json` config describes each tool (bundle globs + run command). `scripts/bundle-diff-test.mjs` runs each tool twice unminified (determinism gate), backs up the bundle files, minifies them via the existing `runtime-minify` subcommand, re-runs the tool, byte-diffs stdout/stderr/exit-code/output-files, restores the backups, and saves mismatch artifacts. One fast `Test Runtime Bundles` CI job runs it all. Differential by construction — expected output is generated in-run, so no pins, no baseline classifier.

**Tech Stack:** Node ESM script (builtins only), existing `monitor-oxc runtime-minify` binary (no Rust changes), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-07-07-bundle-diff-oracle-design.md`

## Global Constraints

- CI runs on `ubuntu-latest` only; all actions pinned to full commit SHAs byte-identical to existing usages in `.github/workflows/ci.yml`.
- Node scripts are ESM, node builtins only — no new npm dependencies.
- No Rust changes: `runtime-minify --config <path> --name <name> --dir <dir>` is reused as-is (it reads `name`/`sources`/`ignore` from the config and ignores unknown fields, so `bundle-tools.json` extra fields are fine).
- CI job display name must start with `Test ` (status-comment filter).
- Commit subjects: conventional-commit style, imperative, ≤ 72 chars.
- Tool packages (typescript, prettier, sass, rollup) are already installed via package.json's npm-high-impact devDependencies — verified present. Versions float; the oracle is version-independent by construction. Verified on this machine: typescript 6.0.3 (`node node_modules/typescript/lib/_tsc.js --version` works), sass bin is `sass.js` (wrapper loading `sass.dart.js`), rollup 4.60.4 (`node node_modules/rollup/dist/bin/rollup --version` works), prettier's bin/entry layout must be resolved during vetting (v3 removed `bin-prettier.js`).
- The runner must always restore the original bundle files (success or failure) so subsequent tools and later CI cases see pristine node_modules.

---

### Task 1: Runner script + typescript entry + self-test verification

**Files:**
- Create: `scripts/bundle-diff-test.mjs`
- Create: `bundle-tools.json` (with the typescript entry only)
- Create: `fixtures/bundle/ts-project/tsconfig.json`, `fixtures/bundle/ts-project/src/shapes.ts`, `fixtures/bundle/ts-project/src/registry.ts`
- Modify: `.gitignore` (add `bundle-diff-artifacts/`)

**Interfaces:**
- Consumes: `./target/release/monitor-oxc runtime-minify --config <path> --name <name> --dir <dir>` (exit 0 = files rewritten in place; nonzero = diagnostics printed).
- Produces (later tasks rely on these exactly):
  - CLI: `node scripts/bundle-diff-test.mjs [--config <path>] [name ...]` — no names = run every entry. Exit 0 = all tools identical; 1 = at least one diff or minify failure; 2 = usage/config error.
  - Env: `MONITOR_OXC_BIN` overrides the binary path (default `./monitor-oxc`; locally use `./target/release/monitor-oxc`).
  - Config entry schema: `name` (string), `dir` (package dir relative to cwd), `sources` (globs relative to `dir`, consumed by both the script's backup and runtime-minify), `run` (shell command, cwd = repo root; the literal token `OUTDIR` is replaced with a fresh temp dir whose contents are diffed), optional `notes`.
  - On mismatch: artifacts under `bundle-diff-artifacts/<name>/` (minified bundle copies + `expected-stdout.txt` / `actual-stdout.txt` etc.).
  - Nondeterministic tool (two unminified runs differ): `::warning::` + skip, does not fail the job.

- [ ] **Step 1: Write the fixtures**

`fixtures/bundle/ts-project/tsconfig.json`:

```json
{
  "compilerOptions": {
    "strict": true,
    "target": "es2020",
    "module": "esnext",
    "moduleResolution": "bundler",
    "declaration": true,
    "pretty": false
  },
  "include": ["src"]
}
```

`fixtures/bundle/ts-project/src/shapes.ts`:

```ts
export enum Kind {
  Circle = "circle",
  Rect = "rect",
}

export interface Shape {
  readonly kind: Kind;
  area(): number;
}

export class Circle implements Shape {
  readonly kind = Kind.Circle;
  constructor(private radius: number) {}
  area(): number {
    return Math.PI * this.radius ** 2;
  }
}

export class Rect implements Shape {
  readonly kind = Kind.Rect;
  constructor(
    private w: number,
    private h: number,
  ) {}
  area(): number {
    return this.w * this.h;
  }
}

export type ShapeOf<K extends Kind> = K extends Kind.Circle ? Circle : Rect;
```

`fixtures/bundle/ts-project/src/registry.ts`:

```ts
import { Kind, type Shape, Circle, Rect } from "./shapes";

export class Registry<T extends Shape> {
  private items = new Map<string, T>();

  register(id: string, item: T): this {
    if (this.items.has(id)) {
      throw new Error(`duplicate: ${id}`);
    }
    this.items.set(id, item);
    return this;
  }

  totalArea(): number {
    let sum = 0;
    for (const item of this.items.values()) {
      sum += item.area();
    }
    return sum;
  }
}

export function defaults(): Registry<Shape> {
  return new Registry<Shape>()
    .register("c", new Circle(2))
    .register("r", new Rect(3, 4));
}

export async function* areas(r: Registry<Shape>): AsyncGenerator<number> {
  yield r.totalArea();
}

export const KINDS: readonly Kind[] = [Kind.Circle, Kind.Rect];
```

- [ ] **Step 2: Write `bundle-tools.json`**

```json
[
  {
    "name": "typescript",
    "dir": "node_modules/typescript",
    "sources": ["lib/_tsc.js"],
    "run": "node node_modules/typescript/lib/_tsc.js -p fixtures/bundle/ts-project --outDir OUTDIR",
    "notes": "lib/_tsc.js is the ~9MB single-scope tsc bundle; lib/tsc.js is just a wrapper. Compares emitted .js/.d.ts plus diagnostics stdout."
  }
]
```

- [ ] **Step 3: Write `scripts/bundle-diff-test.mjs`**

```js
// Differential oracle over bundled CLI tools (see
// docs/superpowers/specs/2026-07-07-bundle-diff-oracle-design.md).
//
// Usage: node scripts/bundle-diff-test.mjs [--config <path>] [name ...]
//
// Per tool: run twice unminified (determinism gate), minify the bundle in
// place via `runtime-minify`, run again, byte-diff stdout/stderr/exit code
// and OUTDIR files, then restore the original bundle. Any diff = minifier
// bug (exit 1). Nondeterministic tool = ::warning:: + skip.
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const args = process.argv.slice(2);
let configPath = "bundle-tools.json";
const names = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--config") {
    configPath = args[++i];
    if (!configPath) usage();
  } else {
    names.push(args[i]);
  }
}

function usage() {
  console.error("usage: node scripts/bundle-diff-test.mjs [--config <path>] [name ...]");
  process.exit(2);
}

const bin = process.env.MONITOR_OXC_BIN ?? "./monitor-oxc";
const tools = JSON.parse(fs.readFileSync(configPath, "utf8"));
const selected = names.length === 0 ? tools : names.map((name) => {
  const tool = tools.find((t) => t.name === name);
  if (!tool) {
    console.error(`no tool named \`${name}\` in ${configPath}`);
    process.exit(2);
  }
  return tool;
});

// Minimal glob for the `sources` patterns we use (`a/b.js`, `dir/*.js`,
// `dir/**/*.js`) — avoids fs.globSync's experimental warning.
function globMatch(pattern, rel) {
  const re = new RegExp(
    "^" +
      pattern
        .split(/(\*\*\/|\*\*|\*)/)
        .map((part) => {
          if (part === "**/") return "(?:.*/)?";
          if (part === "**") return ".*";
          if (part === "*") return "[^/]*";
          return part.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
        })
        .join("") +
      "$",
  );
  return re.test(rel);
}

function listFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true, recursive: true })) {
    if (entry.isFile()) {
      out.push(path.relative(dir, path.join(entry.parentPath, entry.name)));
    }
  }
  return out;
}

function bundleFiles(tool) {
  const rels = listFiles(tool.dir).filter((rel) =>
    tool.sources.some((pattern) => globMatch(pattern, rel.replaceAll("\\", "/"))),
  );
  if (rels.length === 0) {
    console.error(`${tool.name}: sources matched no files under ${tool.dir}`);
    process.exit(2);
  }
  return rels.map((rel) => path.join(tool.dir, rel));
}

function runTool(tool) {
  const outdir = fs.mkdtempSync(path.join(os.tmpdir(), "bundle-diff-"));
  const command = tool.run.replaceAll("OUTDIR", outdir);
  const { status, stdout, stderr } = spawnSync(command, {
    shell: true,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  const files = {};
  for (const rel of listFiles(outdir).sort()) {
    files[rel] = fs.readFileSync(path.join(outdir, rel), "utf8");
  }
  fs.rmSync(outdir, { recursive: true, force: true });
  return { status, stdout, stderr, files };
}

function firstDivergence(label, expected, actual) {
  if (expected === actual) return null;
  const e = expected.split("\n");
  const a = actual.split("\n");
  for (let i = 0; i < Math.max(e.length, a.length); i++) {
    if (e[i] !== a[i]) {
      return `${label} differs at line ${i + 1}:\n  expected: ${e[i] ?? "<eof>"}\n  actual:   ${a[i] ?? "<eof>"}`;
    }
  }
  return `${label} differs`;
}

function compare(expected, actual) {
  const diffs = [];
  if (expected.status !== actual.status) {
    diffs.push(`exit code: expected ${expected.status}, actual ${actual.status}`);
  }
  for (const stream of ["stdout", "stderr"]) {
    const d = firstDivergence(stream, expected[stream], actual[stream]);
    if (d) diffs.push(d);
  }
  const keys = new Set([...Object.keys(expected.files), ...Object.keys(actual.files)]);
  for (const key of [...keys].sort()) {
    if (!(key in actual.files)) {
      diffs.push(`output file missing after minify: ${key}`);
    } else if (!(key in expected.files)) {
      diffs.push(`unexpected output file after minify: ${key}`);
    } else {
      const d = firstDivergence(`file ${key}`, expected.files[key], actual.files[key]);
      if (d) diffs.push(d);
    }
  }
  return diffs;
}

function saveArtifacts(tool, expected, actual, minifiedPaths) {
  const dir = path.join("bundle-diff-artifacts", tool.name);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "expected-stdout.txt"), expected.stdout);
  fs.writeFileSync(path.join(dir, "actual-stdout.txt"), actual.stdout);
  fs.writeFileSync(path.join(dir, "expected-stderr.txt"), expected.stderr);
  fs.writeFileSync(path.join(dir, "actual-stderr.txt"), actual.stderr);
  for (const [rel, content] of Object.entries(expected.files)) {
    const p = path.join(dir, "expected-files", rel);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, content);
  }
  for (const [rel, content] of Object.entries(actual.files)) {
    const p = path.join(dir, "actual-files", rel);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, content);
  }
  for (const file of minifiedPaths) {
    const p = path.join(dir, "minified", path.basename(file));
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.copyFileSync(file, p);
  }
  console.log(`${tool.name}: artifacts saved to ${dir}`);
}

let failed = false;

for (const tool of selected) {
  console.log(`=== ${tool.name}`);
  const expected = runTool(tool);
  if (expected.status !== 0) {
    console.error(`${tool.name}: expected (unminified) run failed with exit ${expected.status}`);
    console.error(expected.stderr);
    process.exit(2);
  }
  const second = runTool(tool);
  const nondet = compare(expected, second);
  if (nondet.length > 0) {
    console.log(`::warning::${tool.name} is nondeterministic unminified — skipped, not an oxc bug`);
    for (const d of nondet) console.log(`  ${d}`);
    continue;
  }

  const files = bundleFiles(tool);
  const backups = files.map((file) => {
    const backup = `${file}.bundle-diff-backup`;
    fs.copyFileSync(file, backup);
    return [file, backup];
  });

  try {
    const minify = spawnSync(bin, ["runtime-minify", "--config", configPath, "--name", tool.name, "--dir", tool.dir], {
      stdio: "inherit",
    });
    if (minify.status !== 0) {
      console.error(`${tool.name}: runtime-minify failed (exit ${minify.status})`);
      failed = true;
      continue;
    }
    const actual = runTool(tool);
    const diffs = compare(expected, actual);
    if (diffs.length > 0) {
      console.error(`${tool.name}: minified output DIFFERS — minifier/codegen bug`);
      for (const d of diffs) console.error(`  ${d}`);
      saveArtifacts(tool, expected, actual, files);
      failed = true;
    } else {
      console.log(
        `${tool.name}: OK (${Object.keys(expected.files).length} output files, stdout ${expected.stdout.length} bytes)`,
      );
    }
  } finally {
    for (const [file, backup] of backups) {
      fs.copyFileSync(backup, file);
      fs.rmSync(backup);
    }
  }
}

process.exit(failed ? 1 : 0);
```

- [ ] **Step 4: Add `bundle-diff-artifacts/` to `.gitignore`**

Append the line `bundle-diff-artifacts/` to `.gitignore` (create the file only if it truly doesn't exist — check first; this repo has one).

- [ ] **Step 5: Build the binary and run the green path**

```bash
cargo build --release
MONITOR_OXC_BIN=./target/release/monitor-oxc node scripts/bundle-diff-test.mjs typescript; echo "exit=$?"
```

Expected: `=== typescript`, then `typescript: OK (4 output files, stdout 0 bytes)` (shapes.js, shapes.d.ts, registry.js, registry.d.ts; no diagnostics), `exit=0`. Then confirm restoration:

```bash
node node_modules/typescript/lib/_tsc.js --version   # still prints Version <x>
head -c 200 node_modules/typescript/lib/_tsc.js      # NOT minified (original header/comments)
ls node_modules/typescript/lib/*.bundle-diff-backup 2>/dev/null   # no leftovers
```

- [ ] **Step 6: Verify the red path and the nondeterminism path with a scratch config**

The red path needs a tool whose output provably changes under minification. A program that prints its own file length is guaranteed to diff:

```bash
SCRATCH="$(mktemp -d)"
mkdir -p "$SCRATCH/pkg"
cat > "$SCRATCH/pkg/tool.js" <<'EOF'
const fs = require("node:fs");
console.log(fs.readFileSync(__filename, "utf8").length);
EOF
cat > "$SCRATCH/config.json" <<EOF
[
  { "name": "selftest", "dir": "$SCRATCH/pkg", "sources": ["tool.js"], "run": "node $SCRATCH/pkg/tool.js" },
  { "name": "nondet", "dir": "$SCRATCH/pkg", "sources": ["tool.js"], "run": "node -e 'console.log(Math.random())'" }
]
EOF
MONITOR_OXC_BIN=./target/release/monitor-oxc node scripts/bundle-diff-test.mjs --config "$SCRATCH/config.json" selftest; echo "exit=$?"
```

Expected: `selftest: minified output DIFFERS`, a `stdout differs at line 1` line, `selftest: artifacts saved to bundle-diff-artifacts/selftest`, `exit=1`. Confirm restore + artifacts:

```bash
grep -c 'node:fs' "$SCRATCH/pkg/tool.js"           # 1 — original restored
ls bundle-diff-artifacts/selftest/                  # expected-*/actual-* + minified/
rm -rf bundle-diff-artifacts
```

Nondeterminism path:

```bash
MONITOR_OXC_BIN=./target/release/monitor-oxc node scripts/bundle-diff-test.mjs --config "$SCRATCH/config.json" nondet; echo "exit=$?"
# Expected: "::warning::nondet is nondeterministic unminified — skipped, not an oxc bug", exit=0
node scripts/bundle-diff-test.mjs --config "$SCRATCH/config.json" bogus; echo "exit=$?"
# Expected: "no tool named `bogus`", exit=2
```

- [ ] **Step 7: Commit**

```bash
git add scripts/bundle-diff-test.mjs bundle-tools.json fixtures/bundle/ts-project .gitignore
git commit -m "feat: add bundle diff oracle with typescript entry"
```

---

### Task 2: prettier, sass, rollup entries

**Files:**
- Modify: `bundle-tools.json` (append three entries)
- Create: `fixtures/bundle/prettier/input.ts`, `fixtures/bundle/prettier/input.css`
- Create: `fixtures/bundle/sass/main.scss`, `fixtures/bundle/sass/_mixins.scss`
- Create: `fixtures/bundle/rollup/main.js`, `fixtures/bundle/rollup/util.js`

**Interfaces:**
- Consumes: the Task 1 script CLI (`node scripts/bundle-diff-test.mjs [name ...]`, `MONITOR_OXC_BIN`), config entry schema (`name`/`dir`/`sources`/`run`/`notes`, `OUTDIR` token), and its vetting behavior (nondeterminism → warn+skip; expected-run failure → exit 2).
- Produces: the final committed 4-entry `bundle-tools.json`.

**Starting entries** (each vetted per Step 3; adjust invocation/sources based on what the installed version actually ships, and record every adjustment in `notes`):

| name | dir | sources (starting) | run (starting) |
|---|---|---|---|
| prettier | `node_modules/prettier` | resolve at vetting — v3 ships `index.cjs` + `plugins/*.js`/`*.mjs`; v2 ships `index.js` + `parser-*.js` | `node <resolved bin> --no-config --no-color fixtures/bundle/prettier/input.ts fixtures/bundle/prettier/input.css` |
| sass | `node_modules/sass` | `["sass.dart.js"]` | `node node_modules/sass/sass.js --no-color --no-source-map fixtures/bundle/sass/main.scss` |
| rollup | `node_modules/rollup` | `["dist/*.js", "dist/**/*.js"]` | `node node_modules/rollup/dist/bin/rollup --input fixtures/bundle/rollup/main.js --format es --silent` |

Resolve prettier first:

```bash
node -e "const p=require('./node_modules/prettier/package.json'); console.log(p.version, JSON.stringify(p.bin)); console.log(require('node:fs').readdirSync('node_modules/prettier').join('\n'))"
```

Use the printed bin path in `run`, and set `sources` to the large bundled `.js`/`.cjs`/`.mjs` files it ships (main entry + parsers/plugins). Note: `dist/bin/rollup` has no `.js` extension so the rollup `sources` globs never match it — the CLI entry stays unminified, which is fine (it requires the dist bundles that ARE minified).

- [ ] **Step 1: Write the fixtures**

`fixtures/bundle/prettier/input.ts` (deliberately messy formatting):

```ts
export   type   Deep<T> = { [K in keyof T] : T[K] extends object ? Deep<T[K]> : T[K] }
export function pick<T,K extends keyof T>(obj:T, ...keys:K[]):Pick<T,K>{ const out={} as Pick<T,K>;for(const k of keys){out[k]=obj[k]}return out}
const   greeting=`hello ${ "world" }`;export default greeting
```

`fixtures/bundle/prettier/input.css`:

```css
.card{display:flex;flex-direction:column;gap:4px}
.card:hover{box-shadow:0 1px 2px rgba(0,0,0,.2)}
@media (min-width:600px){.card{flex-direction:row}}
```

`fixtures/bundle/sass/_mixins.scss`:

```scss
@mixin elevated($level: 1) {
  box-shadow: 0 #{$level}px #{$level * 2}px rgba(0, 0, 0, 0.2);
}

@function double($n) {
  @return $n * 2;
}
```

`fixtures/bundle/sass/main.scss`:

```scss
@use "mixins";

$base: 8px;

.card {
  padding: $base;
  margin: mixins.double($base);
  @include mixins.elevated(2);

  &__title {
    font-weight: bold;
  }
}

@each $name, $size in (small: 4px, large: 16px) {
  .gap-#{$name} {
    gap: $size;
  }
}
```

`fixtures/bundle/rollup/util.js`:

```js
export function fib(n) {
  return n < 2 ? n : fib(n - 1) + fib(n - 2);
}

export const unused = () => {
  throw new Error("should be tree-shaken");
};
```

`fixtures/bundle/rollup/main.js`:

```js
import { fib } from "./util.js";

export const answer = fib(10);
```

- [ ] **Step 2: Append the three entries to `bundle-tools.json`**

Use the table's starting values (with prettier's resolved paths), each with a `notes` field describing the bundle and any flag needed for determinism.

- [ ] **Step 3: Vet each tool**

For each of prettier, sass, rollup, in order:

```bash
MONITOR_OXC_BIN=./target/release/monitor-oxc node scripts/bundle-diff-test.mjs <name>; echo "exit=$?"
```

- `<name>: OK`, `exit=0` → vetted.
- Expected-run failure (exit 2) → fix the `run` command (wrong bin path, missing flag) and retry.
- Nondeterminism warning → find the source (timestamps, hash-based ordering, version banners); prefer a flag that removes it (like `--no-source-map`); if no flag exists, drop the tool with the reason in a `notes` entry in the commit message. Max 3 attempts per tool.
- `DIFFERS` → this is a REAL oxc minifier bug found at vetting time: keep the artifacts, reduce, report to the user immediately (do not silently drop the tool). The tool can be committed only with its bundle excluded or the bug fixed.

Then the full run:

```bash
MONITOR_OXC_BIN=./target/release/monitor-oxc node scripts/bundle-diff-test.mjs; echo "exit=$?"
```

Expected: four `OK` lines (or documented drops), `exit=0`, no `.bundle-diff-backup` files left anywhere:

```bash
find node_modules -name '*.bundle-diff-backup' | head; echo done
```

- [ ] **Step 4: Commit**

```bash
git add bundle-tools.json fixtures/bundle
git commit -m "feat: add prettier, sass, rollup bundle diff entries"
```

---

### Task 3: CI job + README

**Files:**
- Modify: `.github/workflows/ci.yml` (add one job after the `runtime` job; extend `record.needs`)
- Modify: `README.md` (new section after `### Runtime Correctness`)

**Interfaces:**
- Consumes: `scripts/bundle-diff-test.mjs` CLI (no args = all tools; artifacts land in `bundle-diff-artifacts/` on failure), the `build` job's `monitor-oxc` artifact, `oxc-project/setup-node` (which installs pnpm deps, providing the tool packages).

- [ ] **Step 1: Add the job to ci.yml**

Insert directly after the `runtime:` job (before `isolated_declarations:`):

```yaml
  runtime-bundles:
    name: Test Runtime Bundles
    needs: build
    # See `test` — break the transitive skip from the guard.
    if: ${{ !cancelled() && needs.build.result == 'success' }}
    timeout-minutes: 30
    runs-on: ubuntu-latest
    steps:
      - uses: taiki-e/checkout-action@7d1e50e93dc4fb3bba58f85018fadf77898aee8b # v1.4.2

      - uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1
        with:
          name: monitor-oxc

      - run: chmod +x ./monitor-oxc

      - uses: oxc-project/setup-node@ab97f03642370d79a7e96dd286bd02a1be40e0ba # v1.3.0

      - run: node scripts/bundle-diff-test.mjs
        env:
          RUST_BACKTRACE: "1"

      - name: Upload mismatch artifacts
        if: failure()
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1
        with:
          if-no-files-found: ignore
          name: bundle-diff-artifacts
          path: bundle-diff-artifacts/
```

- [ ] **Step 2: Extend the record job**

Change `record.needs` from:

```yaml
    needs: [build, test, isolated_declarations, test262, runtime]
```

to:

```yaml
    needs: [build, test, isolated_declarations, test262, runtime, runtime-bundles]
```

- [ ] **Step 3: Sanity-check the workflow**

```bash
node --input-type=module -e "import fs from 'node:fs'; import yaml from 'js-yaml'; yaml.load(fs.readFileSync('.github/workflows/ci.yml','utf8')); console.log('yaml ok')"
# Expected: yaml ok
```

- [ ] **Step 4: Add the README section**

After the `### Runtime Correctness` section in `README.md`, add:

```markdown
### Runtime Bundles

* minify production mega-bundles already in node_modules ([bundle-tools.json](./bundle-tools.json): tsc, prettier, sass, rollup)
* run each tool on fixed fixtures before and after minification
* byte-diff the outputs — differential by construction, so no pins or baselines needed
```

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml README.md
git commit -m "ci: diff bundled CLI tool output before and after minify"
```

---

## Final integration verification

After the branch lands on a PR: confirm the `Test Runtime Bundles` row appears and is green, and that a forced failure (if ever needed for debugging) uploads the `bundle-diff-artifacts` artifact.
