export function compute(n) {
  return Array.from({ length: n }, (_, i) => i * i).reduce((a, b) => a + b, 0);
}

export const dead = () => "should be tree-shaken";
