import { createInterface } from "node:readline";

// This is built from the Oxc revision the monitor is checking. CI supplies it as an artifact;
// locally, build it with `pnpm --dir ../oxc --filter oxc-codegen run build`.
import { printSync } from "../../oxc/packages/codegen/dist/index.js";

const input = createInterface({ input: process.stdin, crlfDelay: Infinity });

for await (const line of input) {
  if (line.length === 0) continue;

  try {
    const { program, ts, jsx } = JSON.parse(line);
    send("OK", printSync(program, { ts, jsx }).code);
  } catch (error) {
    send("ERROR", error instanceof Error ? error.stack ?? error.message : String(error));
  }
}

function send(status, output) {
  const bytes = Buffer.from(output);
  process.stdout.write(`${status} ${bytes.byteLength}\n`);
  process.stdout.write(bytes);
}
