const fs = require("node:fs");
const Babel = require("@babel/standalone");

const source = fs.readFileSync("fixtures/bundle/babel/input.mjs", "utf8");
const result = Babel.transform(source, {
  filename: "input.mjs",
  presets: [["env", { targets: { ie: "11" }, modules: "commonjs" }]],
});
process.stdout.write(result.code);
