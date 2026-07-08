import { describe, expect, it } from "vitest";

describe("math", () => {
  it("adds", () => {
    expect(1 + 2).toBe(3);
  });
  it("compares objects", () => {
    expect({ a: [1, 2] }).toEqual({ a: [1, 2] });
  });
  it("async resolves", async () => {
    await expect(Promise.resolve("ok")).resolves.toBe("ok");
  });
});
