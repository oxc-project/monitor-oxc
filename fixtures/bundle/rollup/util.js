export function fib(n) {
  return n < 2 ? n : fib(n - 1) + fib(n - 2);
}

export const unused = () => {
  throw new Error("should be tree-shaken");
};
