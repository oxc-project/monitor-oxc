import fs from "node:fs";

import packageJson from "./package.json" with { type: "json" };
import { npmHighImpact } from "npm-high-impact";

const COUNT = 3000;

const ignoreList = new Set([
  // CLIs don't work
  "npm", "yarn", "pnpm", "nx", "vitest", "turbo", "@anthropic-ai/claude-code",
  // NO ESM export
  "@babel/compat-data", "@babel/runtime", "@babel/runtime-corejs3", "@graphql-typed-document-node/core",
  "@jest/globals", "@octokit/openapi-types", "@rushstack/eslint-patch",
  "@testing-library/jest-dom", "assert", "babel-runtime", "constants-browserify", "csstype", "devtools-protocol",
  "es-iterator-helpers", "eslint-module-utils", "ext", "fbjs", "file-system-cache", "language-subtag-registry",
  "node-releases", "readdir-glob", "source-map-support", "spdx-exceptions", "spdx-license-ids",
  "@tokenizer/token", "css-color-names", "eslint-config-next", "extract-files", "jest-watch-typeahead",
  "limiter", "react-app-polyfill", "react-dev-utils", "react-error-overlay",
  "timers-ext", "unfetch", "bare-path", "bare-os", "bare-fs",
  "@noble/hashes", "chromium-bidi", "pg-cloudflare", "react-scripts", "sanitize.css", "vue-template-compiler", "@csstools/normalize.css",
  "@eslint/core", "pn", "dir-glob", "globby", "teeny-request",
  "@babel/helper-globals",
  // broken in node
  "bootstrap", "@vitest/expect", "wait-on", "react-devtools-core",
  "nice-napi", "@babel/cli", "webpack-subresource-integrity", "opencollective-postinstall",
  "react-dropzone",
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
  // Requires `node-gyp`
  "@datadog/pprof", "cpu-features",
  // has template files
  "@nestjs/schematics",
  // breaks node.js > v22.7.0
  "esm",
  // contains a global `var hasOwnProperty = Object.prototype.hasOwnProperty;` which got polluted from some where.
  "react-shallow-renderer",
  // pollutes protytype
  "pirates", "dd-trace", "harmony-reflect",
  // No versions available
  "contenthook",
  // broken
  "@sentry/cli-linux-x64",
  "@tailwindcss/oxide-linux-x64-gnu", "dunder-proto", "html-tags", "hyperdyperid",
  "math-intrinsics", "storybook", "victory-vendor", "@noble/curves", "canvas",
  "http-deceiver", "pdfjs-dist", "spdy"
]);

const ignorePrefixes = [
  "@types", "@tsconfig", "@tsconfig", "@next", "@esbuild", "@nrwl", "@rollup", "@mui", "workbox", "@react-native",
  "@swc", "esbuild-", "es6-", "es5-", "@nx", "@firebase", "@angular", "turbo-", "@storybook", "metro",
  "@img", "@parcel",
  "@smithy", "@aws-sdk", "@google-cloud",
  "@contenthook",
  "@modelcontextprotocol",
  "@tailwindcss/oxide-linux",
  "@unrs/resolver-binding-linux",
  "lightningcss-linux"
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
