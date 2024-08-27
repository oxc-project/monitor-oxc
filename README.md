# Monitor Oxc

## [Metrics](https://oxc-project.github.io/monitor-oxc/metrics)

* Compile time
* Binary size

## Isolated Declarations

* Test against vue

## Top 1000 npm packages from [npm-rank](https://github.com/LeoDog896/npm-rank)

(checkout our [package.json](https://github.com/oxc-project/monitor-oxc/blob/main/package.json) 😆)

For all js / ts files in `node_modules`, apply idempotency test:

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

## Developement

```
rm -rf node_modules && pnpm i
cargo run --release
```

### Generate packages

```bash
pnpm run generate
```
