function fibonacci(n) {
  if (n < 2) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

const unusedHelper = () => {
  console.log("dead code");
};

console.log(fibonacci(12));
