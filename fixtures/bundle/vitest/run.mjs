import { startVitest } from "vitest/node";

const vitest = await startVitest("test", [], {
  root: "fixtures/bundle/vitest",
  watch: false,
  reporters: [{ onInit() {} }],
});

// Walk the suite/test tree to leaf tests only (file.tasks is [suite], not
// [test] directly -- describe() nests one level).
function collect(task, prefix, out) {
  const name = prefix ? `${prefix} > ${task.name}` : task.name;
  if (task.type === "test") {
    out.push(`${name}: ${task.result?.state ?? "unknown"}`);
  } else if (task.tasks) {
    for (const t of task.tasks) collect(t, name, out);
  }
}

const files = vitest.state.getFiles();
const lines = [];
for (const file of [...files].sort((a, b) => a.name.localeCompare(b.name))) {
  collect(file, "", lines);
}
lines.sort();
for (const line of lines) console.log(line);

const failed = files.some((f) => f.result?.state === "fail");
await vitest.close();
process.exit(failed ? 1 : 0);
