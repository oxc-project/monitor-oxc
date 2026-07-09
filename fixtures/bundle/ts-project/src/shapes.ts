export enum Kind {
  Circle = "circle",
  Rect = "rect",
}

export interface Shape {
  readonly kind: Kind;
  area(): number;
}

export class Circle implements Shape {
  readonly kind = Kind.Circle;
  constructor(private radius: number) {}
  area(): number {
    return Math.PI * this.radius ** 2;
  }
}

export class Rect implements Shape {
  readonly kind = Kind.Rect;
  constructor(
    private w: number,
    private h: number,
  ) {}
  area(): number {
    return this.w * this.h;
  }
}

export type ShapeOf<K extends Kind> = K extends Kind.Circle ? Circle : Rect;
