const raw = require("./raw.json");

const SIZE = 1100;

const ignoreList = [
  // package managers don't work
  "npm", "yarn", "pnpm",
  // NO ESM export
  "babel-runtime", "@babel/runtime", "type-fest", "undici-types", "@testing-library/jest-dom",
  "assert", "@babel/compat-data", "csstype", "@jest/globals", "source-map-support",
  "es-iterator-helpers", "spdx-exceptions", "spdx-license-ids",
  "language-subtag-registry",
  // crashed vitest
  "eslint-module-utils", "node-releases",
  // flow
  "ast-types-flow",
  // not compatible with linux
  "fsevents"
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

data.forEach((key) => {
  console.log(`test("${key}", () => import("${key}").then(assert.ok));`);
});
