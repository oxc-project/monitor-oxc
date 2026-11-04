# Monitor Oxc

### Coverage matrix

The npm corpus covers source extensions inferred by Oxc, excluding declaration files,
Flow and the known incompatible paths in `src/lib.rs`. Each variant checks generated
syntax and idempotency; transformer output is reparsed as JavaScript. Variant suites
report every failing configuration and never rewrite dependency files. Target presets
accept Oxc's expected warnings for unsupported BigInt, arbitrary namespace exports
and top-level await; parse errors and other diagnostics still fail.

| Command | Configurations / oracle |
| --- | --- |
| `codegen` | Default codegen, idempotency and package imports after rewriting JS |
| `codegen-variants` | 16 combinations of quote style, whitespace, ASCII escaping and source maps; seven comment policies; space indentation with an initial indent |
| `transformer` | Existing enable-all preset, idempotency and package imports; async lowering and Refresh disabled |
| `transformer-variants` | ES2015/2018/2020/2022/next targets; automatic/classic JSX in production/development; preserved JSX; TS import removal, enum optimization and assignment fields; all transforms with external helpers (including async lowering); import-extension rewrite/remove; custom JSX pragmas; decorators without metadata; Fast Refresh (reparse only) |
| `compressor`, `dce`, `mangler`, `whitespace`, `minifier` | Existing individual passes and full minifier; idempotency and package imports except compressor |
| `minifier-variants` | All eight compress/mangle/whitespace combinations; safest compressor; mangler keep-names, disabled top-level mangling and debug names |
| `formatter`, `formatter_dcr` | Default formatting idempotency (classified against Prettier) and code-removal detection |
| `id` | Isolated declarations against Vue |

CI runs all these corpus commands. On a binary-cache miss, `cargo test --lib` also
runs focused fixtures for JS, ESM, CommonJS, JSX, TS, TSX, MTS, CTS and declaration
syntax. They check source-map contents and legal-comment outputs, assert that async
lowering and Refresh actually run, and compare original/generated stdout in Node for
codegen, minifier combinations, and transformer targets with and without minification.
The fixtures need Node but no installed npm packages.

This is a matrix of supported modes, not the Cartesian product of every Oxc option.
Browser-specific target combinations, arbitrary compiler assumptions, destructive
compressor settings (such as dropping console calls), formatter options and isolated
declaration options are not exhaustively crossed. Inline helper loading is not
implemented by Oxc. External-helper and preserved-JSX corpus output gets syntax checks;
runtime-helper semantics for async lowering and JSX runtimes are not compared by
these dependency-free fixtures. The existing runtime suites exercise the shared
runtime preset, which still disables async lowering.

### Runtime Correctness

* clone pinned popular repos ([runtime-repos.json](./runtime-repos.json))
* minify their sources in place (transform + compress + mangle + whitespace)
* run each repo's own test suite against the minified sources
* a failure with a green unminified baseline = real minifier/transformer bug

### Runtime Bundles

* minify production mega-bundles already in node_modules ([bundle-tools.json](./bundle-tools.json): tsc, prettier, sass, rollup, vite, vue-compiler-sfc, jiti, terser, webpack, vitest, jest-core, jest-runtime, jest-circus, babel-standalone, babel-parser, babel-core)
* run each tool on fixed fixtures before and after minification
* byte-diff the outputs — differential by construction, so no pins or baselines needed

### Minify Size

* only for oxc PRs that change `crates/oxc_minifier`, `oxc_mangler`, `oxc_codegen` or `oxc_ecmascript` (the `Minify Size Gate` job checks the PR's diff against its merge-base)
* build monitor-oxc twice, against the PR head and its merge-base, and run `monitor-oxc size --out <file>.tsv` with each: one full minify (compress + mangle + whitespace) of every JS file in `node_modules`, writing input, output, gzip (-9) and brotli (quality 5) bytes per file without touching the files
* `scripts/size-compare.mjs` diffs the two TSVs: raw, gzip and brotli totals, top files and packages each way, `*.min.js` reported separately, and files that minified on base but fail on head; file and package names link to their source on [npmx.dev](https://npmx.dev)
* for the 20 files that shrank most and the 20 that grew most, `monitor-oxc size-emit` writes readable outputs from both binaries (compressor output pretty-printed and unmangled, then the full minify output); the full report inlines diffs of the top 5 each way and links the full diffs (`minify-size-diffs` artifact)
* the oxc PR gets a single `## Minify Size` comment with the summary (totals and new failures), updated in place on later runs; it links the run, whose summary page holds the full report (top files and packages, output diffs)
* output is deterministic, so every byte of difference comes from the PR

### Uglify Corpus

* run UglifyJS's `test/compress` cases ([uglify-corpus.json](./uglify-corpus.json) pins the corpus)
* execute each tiny case before and after minification in a vm sandbox
* diff the output — failures are pre-minimized compressor-semantics repros

## Top 3000 npm packages from [npm-high-impact](https://github.com/wooorm/npm-high-impact)

(check out our [package.json](https://github.com/oxc-project/monitor-oxc/blob/main/package.json) 😆)

For all js / ts files in `node_modules`, apply idempotency test. 

Read more about our [test infrastrucutre](https://oxc.rs/docs/learn/architecture/test.html)

## Development

```
rm -rf node_modules && pnpm i
cargo test --lib
cargo run --release
cargo run --release -- codegen-variants
cargo run --release -- transformer-variants
cargo run --release -- minifier-variants
```

### Generate packages

```bash
pnpm run generate
```

# [Sponsored By](https://oxc.rs/sponsor)

<p align="center">
  <a href="https://oxc.rs/sponsor">
    <img src="https://raw.githubusercontent.com/oxc-project/sponsors/main/sponsors.svg" alt="Our sponsors" />
  </a>
</p>
