import { Kind, type Shape, Circle, Rect } from "./shapes";

export class Registry<T extends Shape> {
  private items = new Map<string, T>();

  register(id: string, item: T): this {
    if (this.items.has(id)) {
      throw new Error(`duplicate: ${id}`);
    }
    this.items.set(id, item);
    return this;
  }

  totalArea(): number {
    let sum = 0;
    for (const item of this.items.values()) {
      sum += item.area();
    }
    return sum;
  }
}

export function defaults(): Registry<Shape> {
  return new Registry<Shape>()
    .register("c", new Circle(2))
    .register("r", new Rect(3, 4));
}

export async function* areas(r: Registry<Shape>): AsyncGenerator<number> {
  yield r.totalArea();
}

export const KINDS: readonly Kind[] = [Kind.Circle, Kind.Rect];
