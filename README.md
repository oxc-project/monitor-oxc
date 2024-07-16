# Monitor Oxc

## [Metrics](https://oxc-project.github.io/monitor-oxc/metrics)

* Compile time
* Binary size

## Isolated Declarations

* Test against vue

## Top 100 npm packages (goal is all 5000+ packages from [npm-rank](https://github.com/LeoDog896/npm-rank)


For all js / ts files in `node_modules`:

### Parser

* semantic check

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

