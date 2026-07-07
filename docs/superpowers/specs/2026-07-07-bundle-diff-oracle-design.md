# Mega-Bundle Differential CLI Oracle — Design

Date: 2026-07-07
Status: Approved (roadmap chosen after rethink; phase A of A→B→D(+C))

## Problem

The runtime correctness suites (see `2026-07-07-minifier-runtime-correctness-design.md`)
minify library sources per file. Real-world oxc minifier consumption is
overwhelmingly **bundled output**: one inlined scope with thousands of
symbols, cross-module code, bundler idioms, giant literals. Nothing exercises
that input class against oxc main at runtime — oxc_minifier has no exec tests,
and rolldown only consumes released oxc.

## Key insight

Real production mega-bundles are **already installed** in monitor-oxc's
node_modules (`typescript/lib/_tsc.js` ~9MB single scope, prettier, sass's
`sass.dart.js`, rollup's dist). And a CLI tool's output on fixed inputs is a
deterministic oracle — no test suites, no repo pins, no browsers:

```
run tool on fixtures (unminified)   → expected output
minify the tool's bundle in place   → runtime-minify (oxc main)
run tool on fixtures again          → actual output
byte-diff expected vs actual        → any difference = minifier bug
```

The oracle is **differential by construction**: expected output is generated
in-run from the same tool version, so floating package versions, upstream
changes, and environment differences can never cause a false red. This is a
strictly stronger property than the repo-suite oracle (which needs pins and a
baseline-rerun classifier).

## Design

### 1. Tool set and fixtures

Candidates (each vetted during implementation for output determinism and a
diff-able invocation; drop with recorded reason if nondeterministic):

| tool | bundle minified | invocation | compared output |
|---|---|---|---|
| typescript | `lib/_tsc.js` | `node …/_tsc.js -p fixtures/bundle/ts-project` | emitted `.js`/`.d.ts` files + captured stdout (diagnostics) |
| prettier | its bundled dist/parsers | format `fixtures/bundle/prettier/**` to stdout | formatted output |
| sass | `sass.dart.js` | compile `fixtures/bundle/sass/*.scss` | emitted CSS |
| rollup | `dist/**/*.js` | bundle `fixtures/bundle/rollup/main.js` | emitted bundle + warnings |

Fixture inputs are small, committed under `fixtures/bundle/`, and chosen to
produce meaningful work (type errors + emit for tsc, varied syntax for
prettier, imports for rollup). Tool stdout/stderr is normalized only where
vetting proves necessary (e.g. strip version banners, absolute paths); every
normalization is recorded in the runner config with a comment.

### 2. Runner: `scripts/bundle-diff-test.mjs`

One script drives all tools:

1. For each tool: run invocation → save expected outputs.
2. Back up the bundle files (plain file copy — node_modules is not a git
   repo), then `./monitor-oxc runtime-minify --config bundle-tools.json
   --name <tool> --dir node_modules/<tool>` (the existing subcommand and
   config schema work unchanged: `name` + `sources` globs).
3. Re-run invocation → actual outputs.
4. Byte-diff. On mismatch: print the diff, keep the minified bundle and both
   outputs as files for artifact upload, exit 1.
5. Restore the backup (always, success or failure, so later tools and cases
   see pristine node_modules).

`bundle-tools.json` holds per-tool config: `name`, `sources` (bundle globs,
consumed by runtime-minify), `run` (command array), `outputs` (files/dirs to
diff in addition to stdout), optional `normalize` notes.

### 3. CI: one `Test Runtime Bundles` job in ci.yml

Not a matrix — each tool run is seconds, the whole set well under a minute of
tool time; one runner beats N. Steps: checkout + download binary +
setup-node + `pnpm install` (same as the existing Test job rows) →
`node scripts/bundle-diff-test.mjs`. `Test ` name prefix keeps it in the
status comment. `record.needs` gains the job. On failure, upload the kept
mismatch artifacts.

No pinning machinery, no weekly bump, no baseline classifier — none of it is
needed for this oracle.

### 4. What red means

A diff means the minified bundle computed something different — by
construction not an upstream or environment issue. The failing tool + input
is in the log; reproduction is `pnpm install` + the same three commands
locally. Localization inside a 9MB bundle is phase-2 (function-level
bisection would reuse the backlog's reduction ideas).

## Vetting rules (during implementation)

- Two consecutive unminified runs must be byte-identical (determinism gate)
  before a tool is added.
- The minified run must produce meaningful output (not an early crash being
  "identical" to an early crash — the runner asserts exit code 0 on the
  expected run).
- A tool that fails the minified run at vetting time with a semantic
  difference is a REAL oxc bug: reduce, file upstream, and either hold the
  tool out or note the issue — same policy as the repo-suite vetting.

## Roadmap context (approved order, later specs)

- **B — UglifyJS `expect_stdout` corpus** (esbuild-style): 2,768 sandboxed
  stdout-diff cases; pre-minimized repros; compressor semantics.
- **D — Reverse ecosystem CI**: build rolldown with `[patch.crates-io]` →
  oxc main; run rolldown-vite playground `test-build` (Playwright-exec'd
  production builds). Separate daily workflow; compile-fail = warn.
- **C — Seeded differential fuzzing** (terser ufuzz-style), reusing B's
  sandbox; fixed seed in CI, random nightly.
