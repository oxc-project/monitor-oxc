# Minifier Runtime Correctness Suites Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run curated real-world repos' own test suites against their oxc-minified sources (transform → compress → mangle → whitespace) as a 3-hourly CI oracle for runtime semantic bugs.

**Architecture:** A `runtime-repos.json` config pins each repo (slug, SHA, source globs, install/test commands). A new `runtime-minify` binary subcommand rewrites a clone's source files in place through the existing `Driver` (extension preserved, so JS-in-`.ts` passes through vitest/jest untouched). A `Test Runtime` CI matrix job clones each pin, minifies, and classifies failures differentially via `scripts/runtime-test.mjs` (minified-red + baseline-green = real bug; baseline-red = warn but stay green). A weekly workflow bumps pins whose HEAD baseline passes, via PR.

**Tech Stack:** Rust (oxc `CompilerInterface` driver, pico-args, walkdir, rayon; new deps: serde, serde_json, globset), Node ESM scripts, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-07-07-minifier-runtime-correctness-design.md`

## Global Constraints

- CI runs on `ubuntu-latest` only — no Namespace or other custom runners (repo policy).
- All GitHub Actions must be pinned to full commit SHAs, copied from existing workflow usages where the same action appears.
- A local pre-commit hook runs per-crate clippy with `-D warnings` (pedantic enabled via `Cargo.toml` lints). Fix lints structurally. **Never** commit with `--no-verify`.
- This repo has no `cargo test` infrastructure (`test = false` on both targets) — verification is by running the built binary/scripts with expected output, exactly as written in each step.
- Commit subjects: conventional-commit style, imperative, ≤ 72 chars (match `git log`).
- Matrix job display names must start with `Test ` — the status-comment script in ci.yml tracks jobs by that prefix.
- The `oxc` path dependency requires a sibling `../oxc` checkout; `Cargo.lock` may show drift from local oxc — commit `Cargo.lock` changes together with `Cargo.toml` changes.
- Node scripts are ESM (`"type": "module"` in package.json).

---

### Task 1: `runtime-minify` subcommand

**Files:**
- Modify: `Cargo.toml` (add serde, serde_json, globset)
- Modify: `src/transformer.rs` (extract `transform_options()`)
- Create: `src/runtime.rs`
- Modify: `src/lib.rs:1-13` (register module)
- Modify: `src/main.rs:31-34` (wire subcommand next to the `id` branch)

**Interfaces:**
- Consumes: existing `Driver` (`crate::Driver`), `Diagnostic` (`crate::Diagnostic`).
- Produces:
  - CLI: `monitor-oxc runtime-minify --name <name> --dir <path> [--config <path>]` (config defaults to `runtime-repos.json` in the cwd). Exit 0 = all matched files rewritten; nonzero = diagnostics printed.
  - `pub fn transformer::transform_options() -> TransformOptions` — reused by Task 1's driver and the existing `TransformerRunner`.
  - Config schema consumed here: `name: string`, `sources: string[]` (globs relative to `--dir`), `ignore?: string[]`. Other JSON fields (`repo`, `sha`, `install`, `test`, `notes`) are ignored by Rust and consumed by Tasks 2/4/5.

- [ ] **Step 1: Add dependencies**

In `Cargo.toml` `[dependencies]`, after `project-root = "0.2.2"` add:

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
globset = "0.4"
```

Run: `cargo check` — expect success (Cargo.lock updates).

- [ ] **Step 2: Extract shared transform options**

In `src/transformer.rs`, replace the body of `fn driver()` with a call to a new public function so `runtime-minify` uses the identical options:

```rust
/// Transform options shared by the transformer case and the `runtime-minify`
/// command.
pub fn transform_options() -> TransformOptions {
    let mut options = TransformOptions::enable_all();
    // Turns off the refresh plugin because it is never idempotent
    options.jsx.refresh = None;
    // Enables `only_remove_type_imports` avoiding removing all unused imports
    options.typescript.only_remove_type_imports = true;

    // These two injects helper in esm format, which breaks cjs files.
    options.env.es2018.async_generator_functions = false;
    options.env.es2017.async_to_generator = false;

    options
}
```

and:

```rust
    fn driver(&self) -> Driver {
        Driver { transform: Some(transform_options()), ..Driver::default() }
    }
```

(The comments move with the code; delete them from `driver()`.)

- [ ] **Step 3: Create `src/runtime.rs`**

```rust
use std::{
    fs,
    panic::catch_unwind,
    path::{Path, PathBuf},
    process::ExitCode,
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use serde::Deserialize;
use walkdir::WalkDir;

use oxc::{minifier::CompressOptions, span::SourceType};

use crate::{Diagnostic, Driver, transformer};

/// Entry in `runtime-repos.json`. Only the fields the minifier needs are
/// deserialized; `repo` / `sha` / `install` / `test` / `notes` are consumed
/// by CI and `scripts/runtime-test.mjs`.
#[derive(Deserialize)]
struct RepoConfig {
    name: String,
    sources: Vec<String>,
    #[serde(default)]
    ignore: Vec<String>,
}

/// Minify (transform + compress + mangle + remove whitespace) every file in
/// `dir` matched by the named config entry, rewriting each file in place and
/// keeping its extension, so JS emitted into a `.ts` file still flows through
/// the repo's own test transpilation untouched.
pub fn run(config_path: &Path, name: &str, dir: &Path) -> ExitCode {
    let config = load_config(config_path, name);
    let sources = build_globset(&config.sources);
    let ignore = build_globset(&config.ignore);

    let paths = WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let rel =
                path.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/");
            if rel.contains("node_modules") || !sources.is_match(&rel) || ignore.is_match(&rel)
            {
                return None;
            }
            let source_type = SourceType::from_path(path).ok()?;
            if source_type.is_typescript_definition() {
                return None;
            }
            Some((path.to_path_buf(), source_type))
        })
        .collect::<Vec<_>>();

    if paths.is_empty() {
        println!("No files matched the source globs for {name} in {}.", dir.display());
        return ExitCode::FAILURE;
    }

    let diagnostics = paths
        .par_iter()
        .filter_map(|(path, source_type)| {
            let source_text = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("{e:?}\n{}", path.display()));
            match catch_unwind(|| minify_driver().run(path, &source_text, *source_type)) {
                Ok(Ok(minified)) => {
                    fs::write(path, minified).unwrap();
                    None
                }
                Ok(Err(diagnostics)) => Some(diagnostics),
                Err(err) => Some(vec![Diagnostic {
                    case: "RuntimeMinify",
                    path: path.clone(),
                    message: format!("{err:?}"),
                }]),
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    println!("Processed {} files.", paths.len());
    if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        for diagnostic in &diagnostics {
            println!(
                "{}\n{}\n{}",
                diagnostic.case,
                diagnostic.path.to_string_lossy(),
                diagnostic.message
            );
        }
        println!("{} Failed.", diagnostics.len());
        ExitCode::FAILURE
    }
}

fn load_config(path: &Path, name: &str) -> RepoConfig {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let mut repos: Vec<RepoConfig> = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
    let index = repos
        .iter()
        .position(|repo| repo.name == name)
        .unwrap_or_else(|| panic!("no repo named `{name}` in {}", path.display()));
    repos.swap_remove(index)
}

fn build_globset(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder
            .add(Glob::new(pattern).unwrap_or_else(|e| panic!("bad glob `{pattern}`: {e}")));
    }
    builder.build().unwrap()
}

fn minify_driver() -> Driver {
    Driver {
        transform: Some(transformer::transform_options()),
        compress: Some(CompressOptions::default()),
        mangle: true,
        remove_whitespace: true,
        ..Driver::default()
    }
}
```

- [ ] **Step 4: Register the module and wire the CLI**

`src/lib.rs` — add to the `pub mod` list (alphabetical, after `remove_whitespace`):

```rust
pub mod runtime;
```

`src/main.rs` — directly after the `if matches!(task, "id") { ... }` block (line 34), add:

```rust
    if matches!(task, "runtime-minify") {
        let config: PathBuf = args
            .opt_value_from_str("--config")
            .unwrap()
            .unwrap_or_else(|| PathBuf::from("runtime-repos.json"));
        let name: String = args.value_from_str("--name").unwrap();
        let dir: PathBuf = args.value_from_str("--dir").unwrap();
        return monitor_oxc::runtime::run(&config, &name, &dir);
    }
```

(`PathBuf` is already imported in main.rs.)

- [ ] **Step 5: Build and lint**

Run: `cargo clippy` then `cargo build`
Expected: no warnings, successful build.

- [ ] **Step 6: Verify against a fixture directory**

```bash
FIXTURE="$(mktemp -d)"
mkdir -p "$FIXTURE/src"
cat > "$FIXTURE/src/add.ts" <<'EOF'
export enum Kind {
  A = 1,
  B = 2,
}
export function add(a: number, b: number): number {
  return a + b;
}
EOF
cat > "$FIXTURE/src/mul.js" <<'EOF'
export const mul = (a, b) => a * b;
EOF
cat > "$FIXTURE/config.json" <<'EOF'
[
  {
    "name": "fixture",
    "sources": ["src/**/*.ts", "src/**/*.js"]
  }
]
EOF
cargo run -- runtime-minify --config "$FIXTURE/config.json" --name fixture --dir "$FIXTURE"
```

Expected: `Processed 2 files.`, exit 0. Then check:

```bash
cat "$FIXTURE/src/add.ts"
# Expected: single-line minified JS; no `enum` keyword, no type annotations
# (enum lowered by the transformer, e.g. `var Kind=...`), still exports.
node -e "import('file://$FIXTURE/src/mul.js').then(m => { if (m.mul(6, 7) !== 42) process.exit(1); console.log('mul ok'); })"
# Expected: mul ok
```

Parse-error path:

```bash
printf 'const x = ;\n' > "$FIXTURE/src/bad.ts"
cargo run -- runtime-minify --config "$FIXTURE/config.json" --name fixture --dir "$FIXTURE"; echo "exit=$?"
```

Expected: a diagnostic mentioning `bad.ts`, `1 Failed.`, `exit=1`.

Unknown-name path:

```bash
cargo run -- runtime-minify --config "$FIXTURE/config.json" --name nope --dir "$FIXTURE"; echo "exit=$?"
```

Expected: panic message ``no repo named `nope` ``, nonzero exit.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/main.rs src/runtime.rs src/transformer.rs
git commit -m "feat: add runtime-minify subcommand"
```

---

### Task 2: Differential classifier script

**Files:**
- Create: `scripts/runtime-test.mjs`

**Interfaces:**
- Consumes: `runtime-repos.json` entries — fields `name`, `install` (shell command), `test` (shell command).
- Produces: CLI `node scripts/runtime-test.mjs <install|baseline|test> <name> <dir> [config-path]` (config defaults to `runtime-repos.json` in the cwd; commands run with `cwd = <dir>`, `shell: true`, inherited stdio).
  - `install` — exit = install command's success.
  - `baseline` — run the suite once on untouched sources; exit = its success.
  - `test` — differential classification: minified green → 0; red-then-green rerun (flake) → 0; red + baseline green → **1** (real oxc bug); red + baseline red → 0 with a `::warning::` annotation. `<dir>` must be a git clone — restore is `git checkout -- .`.

- [ ] **Step 1: Write the script**

Create `scripts/runtime-test.mjs`:

```js
// Differential runtime harness for `runtime-repos.json` entries.
//
// Usage: node scripts/runtime-test.mjs <install|baseline|test> <name> <dir> [config]
//
//   install  — run the repo's install command.
//   baseline — run the repo's test suite once (untouched sources).
//   test     — classify a minified run:
//                minified green                -> exit 0
//                red once, green on rerun      -> exit 0 (flake)
//                red twice, baseline green     -> exit 1 (oxc bug)
//                red twice, baseline red       -> exit 0 + ::warning:: (rot)
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const [mode, name, dir, configPath = "runtime-repos.json"] = process.argv.slice(2);

if (!["install", "baseline", "test"].includes(mode) || !name || !dir) {
  console.error(
    "usage: node scripts/runtime-test.mjs <install|baseline|test> <name> <dir> [config]",
  );
  process.exit(2);
}

const repos = JSON.parse(fs.readFileSync(configPath, "utf8"));
const repo = repos.find((r) => r.name === name);
if (!repo) {
  console.error(`no repo named \`${name}\` in ${configPath}`);
  process.exit(2);
}

function run(command) {
  console.log(`$ ${command}`);
  const { status } = spawnSync(command, { cwd: dir, shell: true, stdio: "inherit" });
  return status === 0;
}

switch (mode) {
  case "install":
    process.exit(run(repo.install) ? 0 : 1);
  case "baseline":
    process.exit(run(repo.test) ? 0 : 1);
  case "test": {
    if (run(repo.test)) process.exit(0);
    console.log("Minified suite failed once; rerunning to rule out a flake.");
    if (run(repo.test)) process.exit(0);
    console.log("Restoring original sources for the baseline run.");
    if (!run("git checkout -- .")) process.exit(2);
    if (run(repo.test)) {
      console.log(
        `Baseline passes but the minified suite fails: minifier/transformer bug (${name}).`,
      );
      process.exit(1);
    }
    console.log(
      `::warning::Baseline suite failed for ${name} — upstream flake or environment rot, not an oxc bug.`,
    );
    process.exit(0);
  }
}
```

- [ ] **Step 2: Verify all four classification paths with a fixture git repo**

```bash
WORK="$(mktemp -d)"
git init -q "$WORK/repo"
cat > "$WORK/repo/lib.js" <<'EOF'
module.exports = 1;
EOF
cat > "$WORK/repo/check.js" <<'EOF'
process.exit(require("./lib.js") === 1 ? 0 : 1);
EOF
git -C "$WORK/repo" add . && git -C "$WORK/repo" -c user.email=t@t -c user.name=t commit -qm init
cat > "$WORK/config.json" <<'EOF'
[
  { "name": "ok",  "install": "true",  "sources": [], "test": "node check.js" },
  { "name": "rot", "install": "false", "sources": [], "test": "node missing.js" }
]
EOF
```

Green path:

```bash
node scripts/runtime-test.mjs test ok "$WORK/repo" "$WORK/config.json"; echo "exit=$?"
# Expected: exit=0
```

Real-bug path (simulate a bad minification, then confirm the file was restored):

```bash
echo 'module.exports = 2;' > "$WORK/repo/lib.js"
node scripts/runtime-test.mjs test ok "$WORK/repo" "$WORK/config.json"; echo "exit=$?"
# Expected: "Baseline passes but the minified suite fails" ... exit=1
grep -q 'module.exports = 1' "$WORK/repo/lib.js" && echo restored
# Expected: restored
```

Rot path (suite fails even on originals):

```bash
node scripts/runtime-test.mjs test rot "$WORK/repo" "$WORK/config.json"; echo "exit=$?"
# Expected: "::warning::Baseline suite failed for rot" ... exit=0
```

Install + usage paths:

```bash
node scripts/runtime-test.mjs install rot "$WORK/repo" "$WORK/config.json"; echo "exit=$?"
# Expected: exit=1 (install command is `false`)
node scripts/runtime-test.mjs bogus x y; echo "exit=$?"
# Expected: usage line, exit=2
```

- [ ] **Step 3: Commit**

```bash
git add scripts/runtime-test.mjs
git commit -m "feat: add differential runtime test classifier"
```

---

### Task 3: Vet candidates and write `runtime-repos.json`

**Files:**
- Create: `runtime-repos.json` (repo root)

**Interfaces:**
- Consumes: `runtime-minify` CLI (Task 1), `scripts/runtime-test.mjs` (Task 2).
- Produces: the committed config with only repos that pass vetting. Full schema per entry: `name`, `repo` (owner/name), `sha` (40-hex pinned commit), `sources`, `ignore?`, `install`, `test`, `notes?`. Keep the file jq-formatted (`jq .`) so Task 5's programmatic bumps produce minimal diffs.

**Candidates and starting configs** (adjust commands during vetting; the goal is a suite that exercises the *shipped source*, skipping lint/coverage gates that legitimately reject minified code):

| name | repo | sources | ignore | install | test (starting point) |
|---|---|---|---|---|---|
| es-toolkit | toss/es-toolkit | `["src/**/*.ts"]` | `["**/*.spec.ts"]` | `yarn install` | `yarn vitest run --coverage=false` |
| zod | colinhacks/zod | `["packages/zod/src/**/*.ts"]` | `["**/*.test.ts", "**/tests/**"]` | `pnpm install --frozen-lockfile` | `pnpm --filter zod test` |
| dayjs | iamkun/dayjs | `["src/**/*.js"]` | `[]` | `npm install` | `npx jest` |
| js-yaml | nodeca/js-yaml | `["lib/**/*.js", "index.js"]` | `[]` | `npm install` | `npx mocha` |
| ms | vercel/ms | `["src/**/*.ts"]` | `["**/*.test.ts"]` | `npm install` | `npx jest` |
| debug | debug-js/debug | `["src/**/*.js"]` | `[]` | `npm install` | `npx mocha` |
| semver | npm/semver | `["classes/**/*.js", "functions/**/*.js", "internal/**/*.js", "ranges/**/*.js", "index.js"]` | `[]` | `npm install` | `npx tap --disable-coverage` |

Known traps to expect while vetting: `npm test` often chains eslint (fails on minified style — run the test runner directly); tap enforces coverage thresholds (`--disable-coverage`); dayjs tests may need `TZ` env vars (check its package.json `test` script and replicate only the jest part); zod is a pnpm monorepo (test filter must target the `zod` package).

- [ ] **Step 1: Build the release binary once**

Run: `cargo build --release`
Expected: `target/release/monitor-oxc` exists.

- [ ] **Step 2: Vet each candidate** (repeat this block per candidate, in order)

```bash
VET="$(mktemp -d)"
git clone --depth 1 https://github.com/<repo> "$VET/<name>"
git -C "$VET/<name>" rev-parse HEAD   # record: this becomes `sha`
```

Write/extend a scratch config `"$VET/config.json"` with the candidate's starting entry from the table (including the recorded `sha`), then:

```bash
cd "$VET/<name>" && corepack enable 2>/dev/null; cd -   # pnpm/yarn repos only
node scripts/runtime-test.mjs install <name> "$VET/<name>" "$VET/config.json"
node scripts/runtime-test.mjs baseline <name> "$VET/<name>" "$VET/config.json"
# Baseline red? Adjust the `test` command (bypass lint/coverage/TZ wrappers) and retry.
./target/release/monitor-oxc runtime-minify --config "$VET/config.json" --name <name> --dir "$VET/<name>"
node scripts/runtime-test.mjs test <name> "$VET/<name>" "$VET/config.json"
```

Keep/drop criteria:
- **Keep** if: baseline green, minified green, suite ≤ ~10 minutes.
- **Adjust and retry** if red is caused by lint/coverage/`fn.name`/`toString` assertions — narrow `test` to the runner, or exclude the offending source file via `ignore` with a `notes` explanation.
- **Real oxc bug** (baseline green, minified red, failure is semantic): this is the tool working — reduce it, file it against oxc-project/oxc, and still include the repo **only if** the bug is fixed or the affected file is `ignore`d with a note referencing the issue. Report any such find to the user immediately.
- **Drop** if it cannot be made green within 3 attempts; record the reason in the plan-execution notes.

- [ ] **Step 3: Write the final `runtime-repos.json`**

Assemble kept entries (full schema, real SHAs, vetted commands), then normalize formatting:

```bash
jq . runtime-repos.json > tmp.json && mv tmp.json runtime-repos.json
```

Validate against both consumers:

```bash
node -e "JSON.parse(require('node:fs').readFileSync('runtime-repos.json','utf8')).forEach(r => { for (const k of ['name','repo','sha','sources','install','test']) if (r[k] == null) throw new Error(r.name + ' missing ' + k); if (!/^[0-9a-f]{40}$/.test(r.sha)) throw new Error(r.name + ' bad sha'); }); console.log('config ok')"
# Expected: config ok
```

- [ ] **Step 4: Commit**

```bash
git add runtime-repos.json
git commit -m "feat: add pinned runtime correctness repos"
```

---

### Task 4: `Test Runtime` CI matrix job + README

**Files:**
- Modify: `.github/workflows/ci.yml` (add `runtime-setup` + `runtime` jobs; extend `record.needs`)
- Modify: `README.md` (new suite section)

**Interfaces:**
- Consumes: `runtime-repos.json` (Task 3), `runtime-minify` (Task 1), `scripts/runtime-test.mjs` (Task 2), existing `build` job's `monitor-oxc` artifact.
- Produces: matrix jobs named `Test Runtime (<name>)` (tracked by the status comment's `Test ` prefix filter); `record` gains the `runtime` job so the green-marker guard still certifies everything.

- [ ] **Step 1: Add the two jobs to ci.yml**

Insert between the `test:` job and `isolated_declarations:`:

```yaml
  runtime-setup:
    name: Runtime Setup
    needs: build
    # See `test` — break the transitive skip from the guard.
    if: ${{ !cancelled() && needs.build.result == 'success' }}
    runs-on: ubuntu-latest
    outputs:
      matrix: ${{ steps.repos.outputs.matrix }}
    steps:
      - uses: taiki-e/checkout-action@7d1e50e93dc4fb3bba58f85018fadf77898aee8b # v1.4.2

      - id: repos
        run: echo "matrix=$(jq -c 'map({name, repo, sha})' runtime-repos.json)" >> "$GITHUB_OUTPUT"

  runtime:
    name: Test Runtime (${{ matrix.name }})
    needs: [build, runtime-setup]
    if: ${{ !cancelled() && needs.build.result == 'success' && needs.runtime-setup.result == 'success' }}
    timeout-minutes: 30
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        include: ${{ fromJSON(needs.runtime-setup.outputs.matrix) }}
    steps:
      - uses: taiki-e/checkout-action@7d1e50e93dc4fb3bba58f85018fadf77898aee8b # v1.4.2

      - uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1
        with:
          name: monitor-oxc

      - run: chmod +x ./monitor-oxc

      - name: Checkout ${{ matrix.repo }}
        uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
        with:
          persist-credentials: false
          repository: ${{ matrix.repo }}
          ref: ${{ matrix.sha }}
          path: target-repo

      - uses: oxc-project/setup-node@ab97f03642370d79a7e96dd286bd02a1be40e0ba # v1.3.0

      - run: corepack enable

      - name: Cache target repo node_modules
        uses: actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6.1.0
        with:
          path: target-repo/node_modules
          key: runtime-${{ matrix.name }}-${{ matrix.sha }}

      - run: node scripts/runtime-test.mjs install ${{ matrix.name }} target-repo

      - run: ./monitor-oxc runtime-minify --name ${{ matrix.name }} --dir target-repo

      - run: node scripts/runtime-test.mjs test ${{ matrix.name }} target-repo
        env:
          RUST_BACKTRACE: "1"
```

- [ ] **Step 2: Extend the record job**

Change `record.needs` from:

```yaml
    needs: [build, test, isolated_declarations, test262]
```

to:

```yaml
    needs: [build, test, isolated_declarations, test262, runtime]
```

(No guard change: pins live in `runtime-repos.json`, which is part of the monitor-oxc SHA already in the guard key.)

- [ ] **Step 3: Sanity-check the workflow file and matrix jq**

```bash
node --input-type=module -e "import fs from 'node:fs'; import yaml from 'js-yaml'; yaml.load(fs.readFileSync('.github/workflows/ci.yml','utf8')); console.log('yaml ok')"
# Expected: yaml ok
jq -c 'map({name, repo, sha})' runtime-repos.json
# Expected: compact JSON array with one object per vetted repo
```

- [ ] **Step 4: Add a README section**

In `README.md`, after the `### Isolated Declarations` section, add:

```markdown
### Runtime Correctness

* clone pinned popular repos ([runtime-repos.json](./runtime-repos.json))
* minify their sources in place (transform + compress + mangle + whitespace)
* run each repo's own test suite against the minified sources
* a failure with a green unminified baseline = real minifier/transformer bug
```

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml README.md
git commit -m "ci: run pinned repo test suites against minified sources"
```

---

### Task 5: Weekly pin auto-bump workflow

**Files:**
- Create: `.github/workflows/bump-runtime-repos.yml`

**Interfaces:**
- Consumes: `runtime-repos.json`, `scripts/runtime-test.mjs` (`install` / `baseline` modes), the `APP_ID` / `APP_PRIVATE_KEY` secrets already used by ci.yml's status job.
- Produces: a weekly PR titled `chore: bump runtime repo pins` updating `sha` fields for repos whose HEAD baseline passes; repos whose baseline fails are listed in the PR body and keep their pins.

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/bump-runtime-repos.yml`:

```yaml
name: Bump Runtime Repos

permissions: {}

on:
  workflow_dispatch:
  schedule:
    - cron: "0 5 * * 1" # Mondays 05:00 UTC.

jobs:
  setup:
    name: Setup
    runs-on: ubuntu-latest
    outputs:
      matrix: ${{ steps.repos.outputs.matrix }}
    steps:
      - uses: taiki-e/checkout-action@7d1e50e93dc4fb3bba58f85018fadf77898aee8b # v1.4.2

      - id: repos
        run: echo "matrix=$(jq -c 'map({name, repo, sha})' runtime-repos.json)" >> "$GITHUB_OUTPUT"

  check:
    name: Check ${{ matrix.name }}
    needs: setup
    timeout-minutes: 30
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        include: ${{ fromJSON(needs.setup.outputs.matrix) }}
    steps:
      - uses: taiki-e/checkout-action@7d1e50e93dc4fb3bba58f85018fadf77898aee8b # v1.4.2

      - name: Resolve HEAD of ${{ matrix.repo }}
        id: head
        run: echo "sha=$(gh api repos/${{ matrix.repo }}/commits/HEAD --jq .sha)" >> "$GITHUB_OUTPUT"
        env:
          GH_TOKEN: ${{ github.token }}

      - name: Skip if unchanged
        id: skip
        run: echo "unchanged=${{ steps.head.outputs.sha == matrix.sha }}" >> "$GITHUB_OUTPUT"

      - name: Checkout ${{ matrix.repo }}
        if: steps.skip.outputs.unchanged != 'true'
        uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
        with:
          persist-credentials: false
          repository: ${{ matrix.repo }}
          ref: ${{ steps.head.outputs.sha }}
          path: target-repo

      - uses: oxc-project/setup-node@ab97f03642370d79a7e96dd286bd02a1be40e0ba # v1.3.0
        if: steps.skip.outputs.unchanged != 'true'

      - run: corepack enable
        if: steps.skip.outputs.unchanged != 'true'

      - name: Baseline at HEAD
        if: steps.skip.outputs.unchanged != 'true'
        run: |
          mkdir -p results
          if node scripts/runtime-test.mjs install "${{ matrix.name }}" target-repo \
            && node scripts/runtime-test.mjs baseline "${{ matrix.name }}" target-repo; then
            status=green
          else
            status=red
          fi
          jq -n --arg name "${{ matrix.name }}" --arg sha "${{ steps.head.outputs.sha }}" \
            --arg status "$status" '{name: $name, sha: $sha, status: $status}' \
            > "results/${{ matrix.name }}.json"

      - uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1
        if: steps.skip.outputs.unchanged != 'true'
        with:
          if-no-files-found: error
          name: bump-${{ matrix.name }}
          path: results/

  bump:
    name: Open bump PR
    needs: check
    if: ${{ !cancelled() }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/create-github-app-token@1b10c78c7865c340bc4f6099eb2f838309f1e8c3 # v3.1.1
        id: app-token
        with:
          client-id: ${{ secrets.APP_ID }}
          private-key: ${{ secrets.APP_PRIVATE_KEY }}
          owner: oxc-project
          repositories: ${{ github.event.repository.name }}

      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
        with:
          token: ${{ steps.app-token.outputs.token }}

      - uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1
        with:
          pattern: bump-*
          merge-multiple: true
          path: results

      - name: Apply green bumps and open PR
        env:
          GH_TOKEN: ${{ steps.app-token.outputs.token }}
        run: |
          shopt -s nullglob
          bumped=()
          failed=()
          for f in results/*.json; do
            name=$(jq -r .name "$f")
            sha=$(jq -r .sha "$f")
            status=$(jq -r .status "$f")
            old=$(jq -r --arg n "$name" '.[] | select(.name == $n) | .sha' runtime-repos.json)
            if [ "$status" = green ]; then
              jq --arg n "$name" --arg s "$sha" \
                '(.[] | select(.name == $n) | .sha) = $s' runtime-repos.json > tmp.json
              mv tmp.json runtime-repos.json
              bumped+=("- \`$name\`: ${old:0:12} -> ${sha:0:12}")
            else
              failed+=("- \`$name\`: HEAD ${sha:0:12} baseline failed; keeping ${old:0:12}")
            fi
          done
          if [ "${#bumped[@]}" -eq 0 ]; then
            echo "No green bumps this week."
            exit 0
          fi
          branch="bump-runtime-repos-${{ github.run_id }}"
          git config user.name "oxc-project[bot]"
          git config user.email "oxc-project[bot]@users.noreply.github.com"
          git checkout -b "$branch"
          git add runtime-repos.json
          git commit -m "chore: bump runtime repo pins"
          git push origin "$branch"
          {
            printf 'Weekly runtime repo pin bump. CI on this PR runs the minified suites against the new pins.\n\n## Bumped\n'
            printf '%s\n' "${bumped[@]}"
            if [ "${#failed[@]}" -gt 0 ]; then
              printf '\n## Baseline failed at HEAD (kept old pin)\n'
              printf '%s\n' "${failed[@]}"
            fi
          } > body.md
          gh pr create --title "chore: bump runtime repo pins" --body-file body.md --head "$branch"

```

- [ ] **Step 2: Sanity-check yaml and the bump jq expression**

```bash
node --input-type=module -e "import fs from 'node:fs'; import yaml from 'js-yaml'; yaml.load(fs.readFileSync('.github/workflows/bump-runtime-repos.yml','utf8')); console.log('yaml ok')"
# Expected: yaml ok
```

Dry-run the sha-rewrite jq against the real config (pick any real `<name>` from runtime-repos.json):

```bash
jq --arg n "<name>" --arg s "0000000000000000000000000000000000000000" \
  '(.[] | select(.name == $n) | .sha) = $s' runtime-repos.json | jq -r --arg n "<name>" '.[] | select(.name == $n) | .sha'
# Expected: 0000000000000000000000000000000000000000 (stdout only; file unchanged)
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/bump-runtime-repos.yml
git commit -m "ci: bump runtime repo pins weekly via PR"
```

---

## Final integration verification

The matrix jobs can only fully run on GitHub. After the tasks land on a branch/PR:

1. Confirm the PR's CI shows one `Test Runtime (<name>)` row per vetted repo, all green.
2. Trigger `Bump Runtime Repos` once via `workflow_dispatch` and confirm it either opens a pin-bump PR or logs "No green bumps this week."
3. Confirm the `Record green run` job still runs on the next scheduled main run (its `needs` now includes `runtime`).
