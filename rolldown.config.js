import { defineConfig } from 'rolldown'

let modulesCount = 0;

export default defineConfig({
  input: 'src/static.test.mjs',
  platform: 'node',
  // [UNLOADABLE_DEPENDENCY] Error: Could not load ... .node
  external: [
    "fsevents",
    "@swc/core",
    "oxc-resolver",
    "ssh2",
    "@parcel/watcher",
    "jest-resolve",
    "@unrs/resolver-binding-linux-x64-musl",
    "@oxc-parser/binding-linux-x64-musl"
  ],
  resolve: {
    extensions: [".js", ".cjs", ".mjs", ".json"]
  },
  plugins: [
    {
      name: "counter",
      transform() {
        modulesCount += 1;
      },
      buildEnd() {
        console.log("Total number of modules processed: " + modulesCount);
      }
    }
  ]
})
