// SPDX-License-Identifier: Apache-2.0

//! The React front end of the examples (`M9-TS-1600..1649`): what `examples/*` write, compiled as
//! written. These tests only compile; running is the host's business.

fn compile(src: &str, manifests: &[String]) -> Result<(), String> {
    iron_ts::compile_with_manifests(src, "speed", "rf_", 100, manifests).map(|_| ())
}

fn ok(src: &str) {
    if let Err(e) = compile(src, &[]) {
        panic!("{e}");
    }
}

// M9-TS-1600
#[test]
fn an_effect_with_a_cleanup_that_calls_a_void_function_compiles() {
    ok(r#"
function useTicker(): number {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const id = setTimeout(() => setTick(tick + 1), 1000);
    return () => clearTimeout(id);
  }, [tick]);
  return tick;
}
export function App(): UiElement { const t = useTicker(); return <box>{t}</box>; }
"#);
}

// M9-TS-1601
#[test]
fn parameter_types_come_from_their_defaults() {
    ok(r#"
function wave(phase = 0, amp = 1.5, name = "x", on = false): number { return on ? phase * amp : name.length; }
export function f(): number { return wave() + wave(2); }
"#);
}

// M9-TS-1602
#[test]
fn a_class_named_like_an_engine_handle_shadows_it() {
    let manifests = vec![
        r#"{"namespace":"Eng","functions":[{"name":"f","symbol":"eng_f","params":[{"name":"s","type":"handle:Scene"}],"returns":"void"}]}"#
            .to_string(),
    ];
    let src = "class Scene { n: number = 1; get twice(): number { return this.n * 2; } }
        export function g(): number { const s = new Scene(); return s.twice; }";
    if let Err(e) = compile(src, &manifests) {
        panic!("{e}");
    }
}

// M9-TS-1603
#[test]
fn capitalised_react_elements_are_the_intrinsic_ones() {
    ok(r#"
export function App(): UiElement {
  return (
    <Text className="t">hi <Span>there</Span><Input value="a" /><TextArea /><Image src="x.png" /></Text>
  );
}
"#);
}

// M9-TS-1604
#[test]
fn a_program_component_named_text_keeps_its_name() {
    ok(r#"
function Text(props: { n: number }): UiElement { return <box>{props.n}</box>; }
export function App(): UiElement { return <Text n={3} />; }
"#);
}

// M9-TS-1605
#[test]
fn text_numbers_and_lists_are_children_of_a_component() {
    ok(r#"
function Card(props: { title: string; children?: ReactNode }): UiElement {
  return <box><text>{props.title}</text>{props.children}</box>;
}
export function App(): UiElement {
  const xs = [1, 2, 3];
  return <Card title="t">Reset {1 + 2} <text>x</text>{xs.map((x) => <text key={x}>{x}</text>)}{xs.length > 2 && <text>many</text>}</Card>;
}
"#);
}

// M9-TS-1606
#[test]
fn numeric_keys_and_boolean_children_compile() {
    ok(r#"
export function App(on: boolean): UiElement {
  return <box key={4}>{on}{on && <text>x</text>}{on || <text>y</text>}</box>;
}
"#);
}

// M9-TS-1607
#[test]
fn handlers_may_take_no_parameter_or_be_undefined() {
    ok(r#"
function Btn(props: { onPress?: () => void }): UiElement {
  return <box onClick={props.onPress}>b</box>;
}
export function App(): UiElement {
  const f = () => { console.log("x"); };
  const on = true;
  return <box onClick={f}><Btn onPress={f} /><Btn /><box onKeyDown={on ? undefined : f} /></box>;
}
"#);
}

// M9-TS-1608
#[test]
fn style_values_may_be_undefined_or_number_or_string() {
    ok(r#"
function w(big: boolean): number | string { return big ? 100 : "50%"; }
export function App(big: boolean): UiElement {
  const cursor = big ? "pointer" : undefined;
  return <box style={{ width: w(big), cursor, height: 3, color: "red" }} />;
}
"#);
}

// M9-TS-1609
#[test]
fn use_state_takes_a_function_and_holds_number_arrays() {
    ok(r#"
function read(): number[] { return [1, 2, 3]; }
export function App(): UiElement {
  const [a, setA] = useState(() => 5);
  const [b, setB] = useState(read);
  const [xs, setXs] = useState<number[]>([]);
  useEffect(() => { setXs([1, 2]); setA((v) => v + 1); setB([0]); }, []);
  return <box>{xs.length + a + b.length}</box>;
}
"#);
}

// M9-TS-1610
#[test]
fn ambient_react_types_resolve() {
    ok(r#"
const card: IronStyle = { width: 100, padding: "4px", color: "red" };
function Row(props: { label: string; children: React.ReactNode; style?: IronStyle }): UiElement {
  return <box style={{ ...card, ...props.style, flexGrow: 1 }}><text>{props.label}</text>{props.children}</box>;
}
const blob = (left: number, size: number): IronStyle => ({ left, width: size });
export function App(): UiElement {
  return <Row label="a" style={blob(1, 2)}>x</Row>;
}
"#);
}

// M9-TS-1611
#[test]
fn events_carry_modifiers_and_the_target_value() {
    ok(r#"
export function App(): UiElement {
  const [s, setS] = useState("");
  return (
    <box
      onKeyDown={(e: IronKeyboardEvent) => { if (e.shiftKey && e.code === "Enter" && !e.ctrlKey) setS(e.key); }}
      onWheel={(e) => { setS(String(e.deltaY)); }}
      onScroll={(e) => { setS(String(e.scrollTop)); }}
      onChange={(e) => setS(e.target.value)}
    >{s}</box>
  );
}
"#);
}

// M9-TS-1612
#[test]
fn a_node_ref_can_focus_blur_and_scroll() {
    ok(r#"
export function App(): UiElement {
  const r = useRef<IronElement>(null);
  useEffect(() => { r.current?.focus(); r.current?.scrollTo(0, 10); r.current?.blur(); }, []);
  return <box ref={r} />;
}
"#);
}

// M9-TS-1613
#[test]
fn a_static_getter_reads_like_a_field() {
    ok(r#"
class Game {
  static n: number = 4;
  static get twice(): number { return Game.n * 2; }
}
export function f(): number { return Game.twice; }
"#);
}

// M9-TS-1614
#[test]
fn a_component_may_return_null() {
    ok(r#"
function Maybe(props: { on: boolean }) {
  if (!props.on) return null;
  return <box>hi</box>;
}
export function App(): UiElement { return <box><Maybe on={true} /></box>; }
"#);
}

// M9-TS-1615
#[test]
fn create_root_render_mounts_the_element_from_a_top_level_statement() {
    let out = iron_ts::compile_with_manifests(
        "function App(): UiElement { return <box>hi</box>; }
createRoot().render(<App />);",
        "speed",
        "rf_",
        100,
        &[],
    )
    .expect("compiles");
    // The init function calls the host's mount function.
    let needle = b"ts_ui_mount";
    assert!(
        out.object.windows(needle.len()).any(|w| w == needle),
        "the object imports ts_ui_mount"
    );
}

// M9-TS-1616
#[test]
fn tuples_of_one_type_are_arrays() {
    ok(r#"
function wave(n: number): [number, number][] {
  const out: [number, number][] = [];
  for (let i = 0; i < n; i++) out.push([i, i * 2]);
  return out;
}
export function f(): number { const w = wave(3); return w.length + w[1]![1]!; }
"#);
}

// M9-TS-1617
#[test]
fn a_record_over_a_numeric_enum_is_indexed_by_the_enum() {
    ok(r#"
enum Weather { Clear, Rain, Stormy }
const NAMES: Record<Weather, string> = {
  [Weather.Clear]: "Clear",
  [Weather.Rain]: "Rain",
  [Weather.Stormy]: "Stormy",
};
export function name(w: Weather): string { return NAMES[w]!; }
"#);
}

// M9-TS-1618
#[test]
fn a_const_arrow_may_call_itself_and_a_let_arrow_may_be_replaced() {
    ok(r#"
let refresh = () => {};
function App(): UiElement {
  const [n, setN] = useState(0);
  refresh = () => setN((c) => c + 1);
  useEffect(() => {
    let id = 0;
    const loop = () => {
      setN((c) => c + 1);
      id = setTimeout(loop, 16);
    };
    id = setTimeout(loop, 16);
    return () => clearTimeout(id);
  }, []);
  return <box onClick={() => refresh()}>{n}</box>;
}
"#);
}
