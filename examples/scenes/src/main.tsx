// Scenes: the game starts in `scenes/level1.scene.json` (`iron.startScene` in
// project.json). The menu loads the other level from its file (single: everything else
// goes), adds or removes a bonus room (additive), copies a spinner (`instantiate`) and
// saves the active scene to the log. The `Keeper` survives every load
// (`dontDestroyOnLoad`) and counts them. Hot reload keeps all of it (try editing a
// spinner's code while it runs).

/** Turns its object (scene files set the speed and axis). */
class Spinner extends Component {
  @property speed = 45;
  @property axis = Vector3.up;
  override onUpdate(dt: number) {
    this.transform.rotate(this.axis.scale(this.speed * dt));
  }
}

/** Bobs up and down around where it was loaded. */
class Bobber extends Component {
  @property height = 0.3;
  @property speed = 1;
  private base = Vector3.zero;
  override onStart() {
    this.base = this.transform.localPosition;
  }
  override onUpdate() {
    this.transform.localPosition = this.base.add(new Vector3(0, Math.sin(Time.time * this.speed * Math.PI) * this.height, 0));
  }
}

let refresh = () => {};

/**
 * Lives through every scene load and counts them. A hot reload keeps it and its
 * `loads` (fields come back with it) but not its listeners: `onReload` re-attaches them.
 */
class Keeper extends Component {
  loads = 0;
  override onAwake() {
    GameObject.dontDestroyOnLoad(this.gameObject);
    this.listen();
  }
  override onReload() {
    this.listen();
    refresh();
  }
  private listen() {
    SceneManager.onSceneLoaded(() => {
      this.loads++;
      refresh();
    });
    SceneManager.onSceneUnloaded(() => refresh());
  }
}

// Scene-only components: registered so files can name them, not started on their own.
register({ Spinner, Bobber }, { spawn: false });
register({ Keeper });

let bonus: Scene | null = null;

async function loadLevel(path: string) {
  bonus = null;
  await SceneManager.loadSceneAsync(path);
}

async function toggleBonus() {
  if (bonus) {
    SceneManager.unloadScene(bonus);
    bonus = null;
  } else {
    bonus = await SceneManager.loadSceneAsync("scenes/bonus.scene.json", "additive");
  }
  refresh();
}

function copySpinner() {
  const spinners = SceneManager.getActiveScene()
    .getRootGameObjects()
    .filter((g) => g.getComponent(Spinner));
  const original = spinners[Math.floor(Math.random() * spinners.length)];
  if (!original) return;
  const at = original.transform.position.add(new Vector3(Math.random() * 4 - 2, 1.5 + Math.random(), -2 - Math.random() * 2));
  const copy = GameObject.instantiate(original, { position: at });
  copy.getComponent(Spinner)!.speed *= -2;
  refresh();
}

function save() {
  const data = SceneManager.getActiveScene().serialize();
  Log.info(JSON.stringify(data));
  Log.info(`saved ${data.name}: ${data.objects.length} root objects`);
}

function Button(props: { label: string; onClick: () => void }) {
  return (
    <box
      onClick={props.onClick}
      style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", backgroundColor: "rgba(255,255,255,0.12)" }}
    >
      <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.label}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  useEffect(() => {
    refresh = () => setTick((t) => t + 1);
  }, []);
  const scenes = Array.from({ length: SceneManager.sceneCount }, (_, i) => SceneManager.getSceneAt(i));
  const active = SceneManager.getActiveScene();
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box
        style={{
          pointerEvents: "auto",
          alignSelf: "flex-start",
          padding: 14,
          gap: 8,
          borderRadius: 10,
          backgroundColor: "rgba(12, 14, 20, 0.78)",
          flexDirection: "column",
        }}
      >
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Scenes</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>
          {`${scenes.map((s) => (s === active ? `${s.name} (active, ${s.rootCount})` : `${s.name} (${s.rootCount})`)).join(", ")} · ${GameObject.findObjectOfType(Keeper)?.loads ?? 0} loads`}
        </text>
        <box style={{ flexDirection: "row", gap: 6 }}>
          <Button label="Courtyard" onClick={() => void loadLevel("scenes/level1.scene.json")} />
          <Button label="Night garden" onClick={() => void loadLevel("scenes/level2.scene.json")} />
          <Button label={bonus ? "Remove bonus room" : "Add bonus room"} onClick={() => void toggleBonus()} />
        </box>
        <box style={{ flexDirection: "row", gap: 6 }}>
          <Button label="Copy a spinner" onClick={copySpinner} />
          <Button label="Save to log" onClick={save} />
        </box>
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
