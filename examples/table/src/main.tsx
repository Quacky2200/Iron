// SPDX-License-Identifier: Apache-2.0

// A table built from existing layout: a CSS grid whose rows are `display: contents`. Columns are
// sized from every row's content, the long note wraps inside its `1fr` column, rows stripe and
// highlight on hover through descendant selectors, headers sort, a click selects a row, and
// `grid-column` / `grid-row` spans act as colspan / rowspan.
import "./ui.css";

type Item = { name: string; kind: string; note: string; qty: number; price: number };

const ITEMS: Item[] = [
  { name: "Iron sword", kind: "Weapon", note: "Forged at the river forge; blunts quickly on stone.", qty: 1, price: 120 },
  { name: "Potion", kind: "Consumable", note: "Heals.", qty: 12, price: 15 },
  {
    name: "Oak shield",
    kind: "Armour",
    note: "A long note that has to wrap inside its column because the table is only as wide as the page lets it be.",
    qty: 3,
    price: 80,
  },
  { name: "Rope", kind: "Tool", note: "50 m", qty: 200, price: 2 },
  { name: "Gem", kind: "Treasure", note: "Very shiny", qty: 7, price: 400 },
];

const COLUMNS: { key: keyof Item; title: string; numeric?: boolean }[] = [
  { key: "name", title: "Item" },
  { key: "kind", title: "Kind" },
  { key: "note", title: "Notes" },
  { key: "qty", title: "Qty", numeric: true },
  { key: "price", title: "Price", numeric: true },
];

function App() {
  const [sortBy, setSortBy] = useState<keyof Item>("name");
  const [selected, setSelected] = useState("Potion");
  const rows = [...ITEMS].sort((a, b) => (a[sortBy] < b[sortBy] ? -1 : a[sortBy] > b[sortBy] ? 1 : 0));
  const total = ITEMS.reduce((sum, item) => sum + item.qty * item.price, 0);
  return (
    <box className="page">
      <Text className="title">Table</Text>
      <Text className="hint">Click a header to sort and a row to select it.</Text>
      <box className="table">
        <box className="row head">
          {COLUMNS.map((c) => (
            <box key={c.key} className={c.numeric ? "cell num" : "cell"} onClick={() => setSortBy(c.key)}>
              <Text>{c.title + (c.key === sortBy ? " ▾" : "")}</Text>
            </box>
          ))}
        </box>
        {rows.map((item) => (
          <box key={item.name} className={item.name === selected ? "row selected" : "row"} onClick={() => setSelected(item.name)}>
            {COLUMNS.map((c) => (
              <box key={c.key} className={c.numeric ? "cell num" : "cell"}>
                <Text>{String(item[c.key])}</Text>
              </box>
            ))}
          </box>
        ))}
        <box className="row foot">
          <box className="cell span-3">
            <Text>Total value</Text>
          </box>
          <box className="cell num span-2">
            <Text>{String(total)}</Text>
          </box>
        </box>
      </box>
    </box>
  );
}

createRoot().render(<App />);
