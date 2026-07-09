class Registry {
  #items = new Map();

  static create() {
    return new Registry();
  }

  add(key, value = key?.toString?.() ?? "unknown") {
    this.#items.set(key, value);
    return this;
  }

  *entries() {
    yield* this.#items;
  }
}

const registry = Registry.create().add(1).add("two", "TWO");

export const summary = [...registry.entries()]
  .map(([k, v]) => `${k}=${v}`)
  .join("&");

export async function delayed() {
  await Promise.resolve();
  return summary.length;
}
