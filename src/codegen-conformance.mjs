import { once } from "node:events";

// This is built from the Oxc revision the monitor is checking. CI supplies it as an artifact;
// locally, build it with `pnpm --dir ../oxc --filter oxc-codegen run build`.
import { printSync } from "../../oxc/packages/codegen/dist/index.js";

let input = Buffer.alloc(0);

process.stdout.on("error", (error) => {
  // The Rust parent can close the read end while a CI job is being cancelled. Do not turn that
  // expected teardown into an unhandled Node exception which obscures the parent failure.
  if (error.code === "EPIPE") process.exit(0);
  throw error;
});

for await (const chunk of process.stdin) {
  input = Buffer.concat([input, chunk]);

  while (true) {
    const headerEnd = input.indexOf(0x0a);
    if (headerEnd === -1) break;

    const length = Number(input.subarray(0, headerEnd).toString("ascii"));
    if (!Number.isSafeInteger(length) || length < 0) {
      throw new Error(`Invalid codegen request length: ${input.subarray(0, headerEnd)}`);
    }

    const payloadStart = headerEnd + 1;
    const payloadEnd = payloadStart + length;
    if (input.length < payloadEnd) break;

    const payload = input.subarray(payloadStart, payloadEnd);
    input = input.subarray(payloadEnd);

    try {
      const { program, ts, jsx } = JSON.parse(payload.toString("utf8"));
      await send("OK", printSync(program, { ts, jsx }).code);
    } catch (error) {
      await send("ERROR", error instanceof Error ? error.stack ?? error.message : String(error));
    }
  }
}

if (input.length !== 0) {
  throw new Error("Incomplete codegen request at end of input");
}

async function send(status, output) {
  const bytes = Buffer.from(output);
  await write(`${status} ${bytes.byteLength}\n`);
  await write(bytes);
}

async function write(output) {
  if (!process.stdout.write(output)) {
    await once(process.stdout, "drain");
  }
}
