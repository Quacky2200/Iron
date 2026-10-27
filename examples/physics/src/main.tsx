// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// M4 physics showcase, seen from the side: crates and balls fall onto a floor and a
// ramp (rapier in Rust), drawn as UI boxes from each body's last-tick pose. Click to
// drop another; a sensor zone counts what passes through it. Impacts knock (kira),
// louder when harder and panned to where they happen.

/** Pixels per metre, and the world origin on screen (bottom centre). */
const SCALE = 60;
const ORIGIN = { x: 640, y: 660 };

/** A falling crate or ball: it knocks when it lands, louder when harder and panned to where it is. */
class Thing extends Component {
  shape = "box";
  half = 0.25;
  color = "#ffffff";
  onCollisionEnter(c: Collision) {
    const speed = c.relativeVelocity.magnitude;
    if (speed < 0.8) return;
    audio.play("assets/thud.wav", {
      volume: Math.min(1, speed / 8),
      rate: 0.8 + (1 - this.half) * 0.5,
      pan: Math.max(-1, Math.min(1, this.transform.position.x / 8)),
    });
  }
}

/** A sensor zone: counts what passes through. */
class Zone extends Component {
  onTriggerEnter(other: Collider) {
    passes++;
  }
}

const things: Thing[] = [];
let passes = 0;

/** Ground, a ramp tilted 20° about z, and a trigger zone over the floor. */
function buildLevel() {
  const ground = new GameObject("Ground", { position: new Vector3(0, -0.5, 0) });
  ground.addComponent(BoxCollider, { size: new Vector3(20, 1, 4) });
  const ramp = new GameObject("Ramp", { position: new Vector3(-4, 3, 0), rotation: Quaternion.euler(0, 0, -20) });
  ramp.addComponent(BoxCollider, { size: new Vector3(6, 0.3, 4), dynamicFriction: 0.2 });
  const zone = new GameObject("Zone", { position: new Vector3(3, 1.5, 0) });
  zone.addComponent(BoxCollider, { size: new Vector3(2, 3, 4), isTrigger: true });
  zone.addComponent(Zone);
}

function drop(x: number, y: number, i: number) {
  const ball = i % 3 === 0;
  const half = 0.25 + (i % 4) * 0.08;
  // Bodies stay in the z = 0 plane: the scene is 2D, seen from the side.
  const go = new GameObject("Thing", { position: new Vector3(x, y, 0) });
  if (ball) go.addComponent(SphereCollider, { radius: half, bounciness: 0.5 });
  else go.addComponent(BoxCollider, { size: new Vector3(half * 2, half * 2, half * 2), bounciness: 0.1 });
  go.addComponent(Rigidbody, {
    angularDrag: 0.2,
    constraints: RigidbodyConstraints.FreezePositionZ | RigidbodyConstraints.FreezeRotationX | RigidbodyConstraints.FreezeRotationY,
  });
  things.push(go.addComponent(Thing, { shape: ball ? "ball" : "box", half, color: `hsl(${(i * 47) % 360} 70% 60%)` }));
}

class Spawner extends Component {
  override onStart() {
    buildLevel();
    for (let i = 0; i < 14; i++) drop(-5 + (i % 7) * 1.4, 6 + Math.floor(i / 7) * 1.5, i);
  }
}
register({ Spawner });

/** Re-renders every tick, so boxes follow their bodies. */
function useFrame() {
  const [, setN] = useState(0);
  useEffect(() => {
    let id = 0;
    const loop = () => {
      setN((n) => n + 1);
      id = setTimeout(loop, 16);
    };
    id = setTimeout(loop, 16);
    return () => clearTimeout(id);
  }, []);
}

/** Rotation about z, in degrees, from a quaternion (screen y points down). */
const angle = (q: Quaternion) => (-2 * Math.atan2(q.z, q.w) * 180) / Math.PI;

function World() {
  useFrame();
  return (
    <box
      style={{ width: "100%", height: "100%", backgroundImage: "linear-gradient(#1b2433, #0c1018)" }}
      onClick={(e) => drop((e.clientX - ORIGIN.x) / SCALE, (ORIGIN.y - e.clientY) / SCALE, things.length)}
    >
      <box style={{ position: "absolute", left: ORIGIN.x - 10 * SCALE, top: ORIGIN.y, width: 20 * SCALE, height: 30, backgroundColor: "#3a4559" }} />
      <box
        style={{
          position: "absolute",
          left: ORIGIN.x - 7 * SCALE,
          top: ORIGIN.y - 3 * SCALE - 0.15 * SCALE,
          width: 6 * SCALE,
          height: 0.3 * SCALE,
          backgroundColor: "#56627a",
          transform: "rotate(20deg)",
        }}
      />
      <box
        style={{
          position: "absolute",
          left: ORIGIN.x + 2 * SCALE,
          top: ORIGIN.y - 3 * SCALE,
          width: 2 * SCALE,
          height: 3 * SCALE,
          border: "2px dashed rgba(120,220,160,0.6)",
          backgroundColor: "rgba(120,220,160,0.08)",
        }}
      />
      {things.map((t) => {
        const size = t.half * 2 * SCALE;
        return (
          <box
            key={t.id}
            style={{
              position: "absolute",
              left: ORIGIN.x + t.transform.position.x * SCALE - size / 2,
              top: ORIGIN.y - t.transform.position.y * SCALE - size / 2,
              width: size,
              height: size,
              backgroundColor: t.color,
              borderRadius: t.shape === "ball" ? "50%" : 4,
              boxShadow: "0 4px 10px rgba(0,0,0,0.4)",
              transform: `rotate(${angle(t.transform.rotation)}deg)`,
            }}
          />
        );
      })}
      <text style={{ position: "absolute", left: 24, top: 20, color: "#d8e0ee", fontSize: 20 }}>
        {`Physics — ${things.length} bodies, ${passes} passes through the zone. Click to drop more.`}
      </text>
    </box>
  );
}

createRoot().render(<World />);
