import { defineConfig } from 'rolldown'

let modulesCount = 0;

export default defineConfig({
  input: 'src/dynamic.test.mjs',
  external: [
    "fsevents",
    "@swc/core"
  ],
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
