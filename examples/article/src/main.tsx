// SPDX-License-Identifier: Apache-2.0

// A rich-text style article built from boxes: a picture floating left, a pull-quote floating right,
// the text wrapping around both, a second picture further down, and a sortable grid table between
// paragraphs. Headings `clear` the floats above them.
import "./ui.css";

const PHOTO = "assets/images/landscape.png";

const P1 =
  "The valley road climbs for six kilometres before the first village comes into view. Walkers who start at dawn " +
  "reach it with the mist still lying in the fields, and the bakery has usually sold its first batch of loaves.";
const P2 =
  "Beyond the village the track narrows to a path of packed earth and flat stones. It follows the river for most " +
  "of the afternoon, crossing it twice on wooden bridges that creak under a rucksack but have never failed anyone.";
const P3 =
  "Prices are per night in euros, hot meal included. Click a column title to sort.";
const P4 =
  "At the top the path ends at a small lake, and a second photograph is worth a minute of anyone's time: the light " +
  "there turns the water a shade of green that no camera quite catches. The descent takes half as long.";

type Hut = { name: string; metres: number; beds: number; price: number };

const HUTS: Hut[] = [
  { name: "Riverside", metres: 1180, beds: 24, price: 38 },
  { name: "Alder", metres: 1560, beds: 16, price: 44 },
  { name: "Lakeview", metres: 2140, beds: 40, price: 52 },
  { name: "Stone Cap", metres: 2390, beds: 12, price: 61 },
];

const COLUMNS: { key: keyof Hut; title: string; numeric?: boolean }[] = [
  { key: "name", title: "Hut" },
  { key: "metres", title: "Altitude (m)", numeric: true },
  { key: "beds", title: "Beds", numeric: true },
  { key: "price", title: "Price", numeric: true },
];

function App() {
  const [sortBy, setSortBy] = useState<keyof Hut>("metres");
  const rows = [...HUTS].sort((a, b) =>
    sortBy === "name"
      ? a.name.localeCompare(b.name)
      : sortBy === "metres"
        ? a.metres - b.metres
        : sortBy === "beds"
          ? a.beds - b.beds
          : a.price - b.price,
  );
  return (
    <box className="page">
      <box className="article">
        <Text className="kicker">Travel</Text>
        <Text className="headline">Six hours up the valley</Text>
        <Text className="byline">By the Iron desk</Text>

        <Image className="figure" src={PHOTO} />
        <box className="pull">
          <Text>"The light turns the water a shade of green no camera catches."</Text>
        </box>
        <Text className="p">{P1}</Text>
        <Text className="p">{P2}</Text>

        <Text className="h2">The top</Text>
        <Image className="figure-right" src={PHOTO} />
        <Text className="p">{P4}</Text>

        <Text className="h2">Where to sleep</Text>
        <Text className="p">{P3}</Text>
        <box className="table">
          <box className="row head">
            {COLUMNS.map((c) => (
              <box key={c.key} className={(c.numeric ? "cell num" : "cell") + (c.key === sortBy ? " sorted" : "")} onClick={() => setSortBy(c.key)}>
                <Text>{c.title}</Text>
              </box>
            ))}
          </box>
          {rows.map((h) => (
            <box key={h.name} className="row">
              <box className="cell"><Text>{h.name}</Text></box>
              <box className="cell num"><Text>{String(h.metres)}</Text></box>
              <box className="cell num"><Text>{String(h.beds)}</Text></box>
              <box className="cell num"><Text>{String(h.price)}</Text></box>
            </box>
          ))}
        </box>
        <Text className="caption">Photographs: the Iron desk.</Text>
      </box>
    </box>
  );
}

createRoot().render(<App />);
