import fs from "node:fs";

import packageJson from "./package.json" with { type: "json" };
import { npmHighImpact } from "npm-high-impact";

const COUNT = 3000;

const ignoreList = new Set([
  // CLIs don't work
  "npm", "yarn", "pnpm", "nx", "vitest", "turbo",
  // NO ESM export
  "@babel/compat-data", "@babel/runtime", "@babel/runtime-corejs3", "@esbuild/linux-x64", "@graphql-typed-document-node/core",
  "@jest/globals", "@octokit/openapi-types", "@rushstack/eslint-patch",
  "@testing-library/jest-dom", "assert", "babel-runtime", "constants-browserify", "csstype", "devtools-protocol",
  "es-iterator-helpers", "eslint-module-utils", "ext", "fbjs", "file-system-cache", "language-subtag-registry",
  "node-releases", "octokit/types", "readdir-glob", "source-map-support", "spdx-exceptions", "spdx-license-ids",
  "@tokenizer/token", "css-color-names", "eslint-config-next", "extract-files", "jest-watch-typeahead",
  "limiter", "react-app-polyfill", "react-dev-utils", "react-error-overlay",
  "timers-ext", "unfetch", "bare-path", "bare-os", "bare-fs",
  "@noble/hashes", "chromium-bidi", "pg-cloudflare", "react-scripts", "sanitize.css", "vue-template-compiler", "@csstools/normalize.css",
  // broken in node
  "bootstrap", "@vitest/expect", "wait-on", "metro-symbolicate", "react-devtools-core",
  "nice-napi", "envdb", "@babel/cli", "webpack-subresource-integrity", "opencollective-postinstall",
  // types
  "type", "type-fest", "types-registry", "undici-types", "@octokit/types", "@schematics/angular",
  "@react-types/shared",
  // flow
  "ast-types-flow", "react-native",
  // not compatible with linux
  "fsevents",
  // breaks rolldown
  "eslint-plugin-import", "event-emitter", "d", "memoizee", "next", "nx",
  // not strict mode
  "js-beautify",
  // need transform to cjs
  "fast-json-patch",
  // Deprecated: use `rehype-external-links`
  "remark-external-links", "remark-slug"
]);

const ignorePrefixes = [
  "@types", "@tsconfig", "@tsconfig", "@next", "@esbuild", "@nrwl", "@rollup", "@mui", "workbox",
  "@swc", "esbuild-", "es6-", "es5-", "@nx", "@firebase", "@angular", "turbo-", "@storybook", "metro"
];

const vue = [
  "language-tools",
  "naive-ui",
  "nuxt",
  "pinia",
  // "primevue",
  "quasar",
  "radix-vue",
  "router",
  "test-utils",
  "vant",
  "@vitejs/plugin-vue",
  "vitepress",
  "vue-i18n",
  "unplugin-vue-macros",
  "vue-simple-compiler",
  "vuetify",
  "@vueuse/core",
];

const data = [
  ...new Set(
    npmHighImpact
      .filter((key) => !ignorePrefixes.some((p) => key.startsWith(p)))
      .filter((key) => !ignoreList.has(key))
      .slice(0, COUNT)
      .concat(vue),
  ),
].sort();

packageJson.devDependencies = {};
data.map((name) => {
  packageJson.devDependencies[name] = "latest";
});

fs.writeFileSync("./package.json", JSON.stringify(packageJson, null, 2));

let dynamicTestFile = 'import test from "node:test"\nimport assert from "node:assert";\n';
data.forEach((key) => {
  dynamicTestFile += `test("${key}", () => import("${key}").then(assert.ok));\n`;
});
fs.writeFileSync("./src/dynamic.test.mjs", dynamicTestFile);

let staticTestFile = 'import test from "node:test"\nimport assert from "node:assert";\n';
data.forEach((key, i) => {
  staticTestFile += `import * as _${i} from "${key}";\ntest("${key}", () => assert.ok(_${i}));\n`;
});
fs.writeFileSync("./src/static.test.mjs", staticTestFile);
