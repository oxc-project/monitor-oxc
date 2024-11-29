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
