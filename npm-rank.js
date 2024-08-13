const raw = require("./raw.json");

const SIZE = 1050;

const ignoreList = [
  // NO ESM export
  "babel-runtime", "@babel/runtime", "type-fest", "undici-types", "@testing-library/jest-dom",
  "assert", "@babel/compat-data", "csstype", "@jest/globals", "source-map-support",
  "npm", "es-iterator-helpers", "spdx-exceptions", "spdx-license-ids", "yarn",
  "language-subtag-registry",
  // crashed vitest
  "eslint-module-utils", "node-releases",
  // flow
  "ast-types-flow",
];

const data = raw
  .slice(0, SIZE)
  .map((data) => data.name)
  .filter((key) => !key.startsWith("@types/"))
  .filter((key) => !key.startsWith("@angular/"))
  .filter((key) => !ignoreList.includes(key))
  .sort();

const packageJson = {};

data.map((name) => {
  packageJson[name] = "latest";
});

// console.log(JSON.stringify(packageJson, null, 2));

const map = new Map();

data.forEach((key) => {
  const ns = key.replace(/^@/, "").replace(/\/|-|\./g, "_");
  if (!map.get(ns)) {
    map.set(ns, key);
  }
});

map.forEach((key, ns) => {
  console.log(`test("${key}", () => import("${key}").then(assert.ok));`);
});
