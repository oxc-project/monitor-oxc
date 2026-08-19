import { once } from "node:events";

// This is built from the Oxc revision the monitor is checking. CI supplies it as an artifact;
// locally, build it with `pnpm --dir ../oxc --filter oxc-codegen run build`.
import { printSync } from "../../oxc/packages/codegen/dist/index.js";

let input = Buffer.alloc(0);

process.stdout.on("error", (error: Error & { code?: string }) => {
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
      const { program, ts, jsx, sourceFilename, sourceText } = JSON.parse(payload.toString("utf8"));
      const { code, map } = printSync(program, {
        ts,
        jsx,
        sourcemap: true,
        sourceFilename,
        sourceText,
      });
      if (map === null) throw new Error("JS codegen did not return a source map");
      await send("OK", code, JSON.stringify(map));
    } catch (error) {
      await send("ERROR", error instanceof Error ? error.stack ?? error.message : String(error));
    }
  }
}

if (input.length !== 0) {
  throw new Error("Incomplete codegen request at end of input");
}

async function send(status: "OK" | "ERROR", code: string, map = ""): Promise<void> {
  const codeBytes = Buffer.from(code);
  const mapBytes = Buffer.from(map);
  await write(`${status} ${codeBytes.byteLength} ${mapBytes.byteLength}\n`);
  await write(codeBytes);
  await write(mapBytes);
}

async function write(output: string | Uint8Array): Promise<void> {
  if (!process.stdout.write(output)) {
    await once(process.stdout, "drain");
  }
}
