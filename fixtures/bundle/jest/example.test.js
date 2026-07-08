describe("math", () => {
  test("adds", () => {
    expect(1 + 2).toBe(3);
  });
  test("mock functions", () => {
    const fn = jest.fn((x) => x * 2);
    expect(fn(21)).toBe(42);
    expect(fn).toHaveBeenCalledTimes(1);
  });
  test("async", async () => {
    await expect(Promise.resolve("ok")).resolves.toBe("ok");
  });
});
