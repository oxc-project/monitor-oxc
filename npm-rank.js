const raw = require('./raw.json');
const unordered = {};

raw.slice(0, 200).map((x) => {
  unordered[x.name] = "latest"
});


const ordered = Object.keys(unordered).sort().reduce(
  (obj, key) => {
    obj[key] = unordered[key];
    return obj;
  },
  {}
);

// console.log(JSON.stringify(ordered, null, 2));

const ignoreList = ["@babel/runtime", "type-fest", "undici-types", "@testing-library/jest-dom"];

Object.keys(ordered)
  .filter((key) => {
    return !ignoreList.includes(key) && !key.startsWith("@types")
  })
  .forEach((key) => {
  const ns = key.replace(/^@/, "").replace(/\/|-|\./g, "_");
  console.log(`
import * as ${ns} from "${key}";
test("${key}", () => {
  assert.ok(${ns});
});`)
})
