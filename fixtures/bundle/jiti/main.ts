enum Level {
  Low = 1,
  High = 2,
}

interface Item {
  name: string;
  level: Level;
}

const items: Item[] = [
  { name: "a", level: Level.Low },
  { name: "b", level: Level.High },
];

async function total(list: Item[]): Promise<number> {
  return list.reduce((sum, item) => sum + item.level, 0);
}

total(items).then((n) => console.log(`total=${n}`));
