# Monitor Oxc

### Transformer

* Parse + transform idempotency test
* transform and override all `j|tsx?` files
* run `./src/main.test.mjs`

### Codegen

* Parse + codegen idempotency test
* codegen and override all js files
* run `./src/main.test.mjs`

### Mangler

* Parse + mangle idempotency test
* mangle and override all js files
* run `./src/main.test.mjs`

### Compressor

* Parse + compress idempotency test
* compress and override all js files
* run `./src/main.test.mjs`

### Isolated Declarations

* Test against vue

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
cargo run --release
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
