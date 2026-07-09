const fs = require("node:fs");
const { parse, compileTemplate } = require("@vue/compiler-sfc");

const source = fs.readFileSync("fixtures/bundle/vue/App.vue", "utf8");
const { descriptor } = parse(source, { filename: "App.vue" });
const result = compileTemplate({
  source: descriptor.template.content,
  filename: "App.vue",
  id: "fixture",
});
process.stdout.write(result.code);
