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

## ❤ Who's [Sponsoring Oxc](https://github.com/sponsors/Boshen)?

<p align="center">
  <a href="https://github.com/sponsors/Boshen">
    <img src="https://raw.githubusercontent.com/Boshen/sponsors/main/sponsors.svg" alt="Our sponsors" />
  </a>
</p>
