const path = require("node:path");
const webpack = require("webpack");

const outdir = process.argv[2];
webpack(
  {
    mode: "production",
    context: path.resolve("fixtures/bundle/webpack"),
    entry: "./src/main.js",
    output: { path: outdir, filename: "main.js" },
    optimization: { minimize: false },
    stats: "none",
  },
  (err, stats) => {
    if (err) {
      console.error(err.message);
      process.exit(1);
    }
    const json = stats.toJson({ all: false, assets: true, errors: true });
    for (const e of json.errors ?? []) console.error(e.message);
    for (const a of (json.assets ?? []).sort((x, y) => x.name.localeCompare(y.name))) {
      console.log(`${a.name} ${a.size}`);
    }
    process.exit(stats.hasErrors() ? 1 : 0);
  },
);
