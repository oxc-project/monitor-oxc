const fs = require("node:fs");
const babel = require("@babel/core");

const source = fs.readFileSync("fixtures/bundle/babel/input.mjs", "utf8");
const result = babel.transformSync(source, {
  filename: "input.mjs",
  configFile: false,
  babelrc: false,
  presets: [[require.resolve("@babel/preset-env"), { targets: { ie: "11" }, modules: "commonjs" }]],
});
process.stdout.write(result.code);
