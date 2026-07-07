# Minifier Runtime Correctness Suites — Design

Date: 2026-07-07
Status: Approved

## Problem

monitor-oxc's existing oracles for the minifier pipeline are idempotency
(transform twice, diff) and an import smoke test (`static.test.mjs` asserts
every installed package still loads after its files are rewritten in place).
Neither executes package *logic*, so a semantically wrong compression, mangle,
or transform that still parses and imports goes undetected.

## Goal

Catch runtime semantic bugs in the full minifier pipeline — transform →
compress → mangle → remove whitespace, the configuration rolldown/vite ship —
by running the test suites of curated real-world repos against their own
minified sources. TS/TSX repos are included deliberately: their sources pass
through `oxc_transformer` first, so the check also catches transformer runtime
bugs.

## Non-goals (recorded phase-2 backlog)

- File-level bisection to localize which minified file broke a red suite.
- Running the UglifyJS `test/compress` `expect_stdout` corpus (esbuild-style
  run-and-compare-stdout oracle).
- Sourcemap validation.
- Per-tool option matrices.

## Design

### 1. Config: `runtime-repos.json`

Single source of truth read by both the Rust binary and CI:

```json
[
  {
    "name": "zod",
    "repo": "colinhacks/zod",
    "sha": "<40-char pinned commit>",
    "sources": ["packages/zod/src/**/*.ts"],
    "ignore": ["**/*.d.ts"],
    "install": "pnpm install --frozen-lockfile",
    "test": "pnpm test",
    "notes": "why this repo / quirks"
  }
]
```

- `sources` globs cover shipped source only, never test files: suites execute
  unminified assertions against minified implementation, mirroring how users
  consume minified code.
- Pins are deliberate: a red run always means oxc changed, never that an
  upstream main broke.
- Initial candidates (each vetted during implementation for a green baseline,
  acceptable runtime, and no `fn.name` / `Function.prototype.toString` /
  stack-trace-sensitive assertions): es-toolkit, zod (TS); dayjs, js-yaml, ms,
  debug, semver (JS). Candidates that fail vetting are dropped with the reason
  recorded in this spec or the config `notes`.

### 2. Rust: `runtime-minify` subcommand

`./monitor-oxc runtime-minify --name <name> --dir <clone>`:

- Reads the named entry from `runtime-repos.json` (in the monitor-oxc
  checkout), walks files in `<clone>` matching `sources` minus `ignore`.
- Rewrites each file **in place, keeping its original extension**, through the
  existing `Driver` configured with: transform (same options as
  `TransformerRunner`), `CompressOptions::default()`, `mangle: true`,
  `remove_whitespace: true`.
  - JS emitted into a `.ts`/`.tsx` file passes through vitest/jest
    transpilation untouched — JS is valid TS (the established twice-mangle
    extension trick).
- Per-file `catch_unwind` like `NodeModulesRunner`; parse errors or panics
  produce diagnostics and a nonzero exit.
- Single-pass by design — idempotency is already covered by existing cases.

### 3. CI: `Test Runtime` matrix job in ci.yml

- A small setup job `jq`s `runtime-repos.json` into a matrix of repo names, so
  rows render as `Test Runtime (zod)`. The `Test ` name prefix means the
  status-comment script tracks them with no changes.
- Each row: `needs: build` (with the usual `!cancelled()` anti-skip override)
  → checkout monitor-oxc + download the built binary artifact → checkout the
  target repo at its pinned SHA → setup-node + node_modules cache keyed on
  (repo SHA, lockfile hash) → run `install` → `runtime-minify` → run the
  classifier script (section 4). `timeout-minutes: 30`, `fail-fast: false`.
- `record` gains the runtime job in its `needs` so green-marker dedupe still
  certifies everything.
- No guard changes: pins live in a committed file, so the guard key
  (monitor-oxc SHA) already invalidates on pin bumps.

### 4. Failure classification: `scripts/runtime-test.mjs`

The oracle is differential — never trust "minified suite failed" alone:

1. Run the suite on minified sources → green ⇒ done.
2. Red → rerun once (flake insurance) → green ⇒ done.
3. Still red → restore originals (`git -C <clone> checkout -- .`) → run the
   baseline suite.
4. Baseline green ⇒ **real minifier/transformer bug** — job fails, with both
   logs available.
5. Baseline red ⇒ environment/flake rot (pins were vetted green) — emit a
   `::warning::` annotation, exit 0 so the monitor doesn't cry wolf. The
   weekly bump workflow surfaces persistent rot.

### 5. Weekly auto-bump: `bump-runtime-repos.yml`

Weekly cron. Per repo: checkout HEAD, install, run the **baseline** suite.
Collect repos whose HEAD baseline passes, update their `sha` in
`runtime-repos.json`, open a single PR via the existing app-token pattern.
Normal CI on that PR validates the new pins *minified* before merge. Repos
whose HEAD baseline fails keep their old pin and are listed in the PR body.

### 6. Testing the feature itself

- Fixture-based integration test for `runtime-minify`: a small directory of
  `.ts`/`.js` files is rewritten in place, output is valid JS, exit codes are
  correct for the success / parse-error paths.
- Classifier verification (manual, during development): hand-corrupt one
  source file in a clone and confirm the "real bug" path fails the job;
  force a baseline failure and confirm the warn-but-green path.

## Key decisions and rationale

| Decision | Choice | Why |
| --- | --- | --- |
| Pipeline | Full minify (transform + compress + mangle + whitespace) | Matches what rolldown/vite ship; mangler `reserved: exports/module` already handled in `Driver` |
| CI placement | Matrix rows in existing ci.yml | 3-hourly detection latency, guard dedupe and status comment for free |
| Repo set | Small curated, TS included via transformer | Trustworthy signal first; TS coverage catches transformer runtime bugs |
| Pinning | Pinned SHAs + weekly auto-bump PR | Deterministic red/green plus freshness, upstream breakage can't page us |
| Where minified | On disk, in place, extension preserved | Works with any test runner; no per-repo build integration |
