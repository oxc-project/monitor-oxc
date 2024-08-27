import fs from 'node:fs';

import packageJson from "./package.json" with { type: "json" };
import { npmHighImpact } from 'npm-high-impact'


const ignoreList = new Set([
  // package managers don't work
  "npm", "yarn", "pnpm",
  // NO ESM export
  "babel-runtime", "@babel/runtime", "type-fest", "undici-types", "@testing-library/jest-dom",
  "assert", "@babel/compat-data", "csstype", "@jest/globals", "source-map-support",
  "es-iterator-helpers", "spdx-exceptions", "spdx-license-ids",
  "language-subtag-registry",
  "@storybook/components", "@storybook/node-logger",
  "@octokit/openapi-types",
  "@graphql-typed-document-node/core",
  "@esbuild/linux-x64",
  "types-registry",
  "type",
  "readdir-glob",
  "eslint-module-utils", "node-releases",
  "file-system-cache",
  "fbjs",
  "ext",
  "devtools-protocol",
  "constants-browserify",
  "@rushstack/eslint-patch",
  "octokit/types",
  "@babel/runtime-corejs3",
  // flow
  "ast-types-flow",
  // not compatible with linux
  "fsevents",
]);

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

const data = [...new Set(npmHighImpact
  .slice(0, 2000)
  .filter((key) => !key.startsWith("@types/"))
  .filter((key) => !key.startsWith("@tsconfig/"))
  // web3 crap
  // .filter((key) => !key.startsWith("@juigorg/"))
  // .filter((key) => !key.startsWith("@/zitterorg"))
  .filter((key) => !ignoreList.has(key)).concat(vue))]
  .sort();

packageJson.devDependencies = {};
data.map((name) => {
  packageJson.devDependencies[name] = "latest";
});

fs.writeFileSync('./package.json', JSON.stringify(packageJson, null, 2));

let dynamicTestFile = 'import test from "node:test"\nimport assert from "node:assert";\n';
data.forEach((key) => {
  dynamicTestFile += `test("${key}", () => import("${key}").then(assert.ok));\n`
});
fs.writeFileSync('./src/dynamic.test.mjs', dynamicTestFile);

let staticTestFile = 'import test from "node:test"\nimport assert from "node:assert";\n';
data.forEach((key, i) => {
  staticTestFile += `import * as _${i} from "${key}";\ntest("${key}", () => assert.ok(_${i}));\n`
});
fs.writeFileSync('./src/static.test.mjs', dynamicTestFile);
