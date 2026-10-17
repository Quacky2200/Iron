(() => {
  // ../../js/engine/src/input.ts
  var keys = new Set;
  var pads = new Map;
  var listeners = {};
  function emit(type, event) {
    for (const fn of listeners[type] ?? []) {
      try {
        fn(event);
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[input] error in ${type} listener: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
      }
    }
  }
  function pad(id, name = "") {
    let p = pads.get(id);
    if (!p) {
      p = { id, name, connected: true, buttons: {}, axes: {} };
      pads.set(id, p);
    }
    return p;
  }
  globalThis.__iron_input = (type, ...args) => {
    switch (type) {
      case "keydown": {
        const [code, key, repeat] = args;
        keys.add(code);
        emit("keydown", { code, key, repeat: !!repeat });
        break;
      }
      case "keyup": {
        const [code, key] = args;
        keys.delete(code);
        emit("keyup", { code, key, repeat: false });
        break;
      }
      case "blur":
        keys.clear();
        for (const p of pads.values())
          for (const b of Object.keys(p.buttons))
            p.buttons[b] = 0;
        break;
      case "gamepadconnected": {
        const [id, name] = args;
        const p = pad(id, name);
        p.connected = true;
        emit("gamepadconnected", { gamepad: p });
        break;
      }
      case "gamepaddisconnected": {
        const p = pad(args[0]);
        p.connected = false;
        emit("gamepaddisconnected", { gamepad: p });
        break;
      }
      case "gamepadbutton": {
        const [id, button, value] = args;
        const p = pad(id);
        const was = (p.buttons[button] ?? 0) > 0.5;
        p.buttons[button] = value;
        const now = value > 0.5;
        if (now && !was)
          emit("gamepadbuttondown", { gamepad: p, button });
        if (!now && was)
          emit("gamepadbuttonup", { gamepad: p, button });
        break;
      }
      case "gamepadaxis": {
        const [id, axis, value] = args;
        const p = pad(id);
        p.axes[axis] = value;
        emit("gamepadaxis", { gamepad: p, axis, value });
        break;
      }
    }
  };
  // ../../js/engine/src/physics.ts
  var vec = (v) => v ? [v.x, v.y, v.z] : null;
  var quat = (q) => q ? [q.x, q.y, q.z, q.w] : null;
  var nextBody = 1;
  var nextCollider = 1;
  var nextQuery = 1;
  var bodies = new Map;
  var colliders = new Map;
  var queries = new Map;

  class Collider {
    id;
    body;
    sensor;
    constructor(body, options) {
      this.id = nextCollider++;
      this.body = body;
      this.sensor = !!options.sensor;
      const s = options.shape;
      let shape;
      let dims;
      let indices = null;
      if ("box" in s)
        [shape, dims] = ["box", [s.box.x, s.box.y, s.box.z]];
      else if ("ball" in s)
        [shape, dims] = ["ball", [s.ball]];
      else if ("capsule" in s)
        [shape, dims] = ["capsule", [s.capsule.halfHeight, s.capsule.radius]];
      else if ("cylinder" in s)
        [shape, dims] = ["cylinder", [s.cylinder.halfHeight, s.cylinder.radius]];
      else if ("convex" in s)
        [shape, dims] = ["convex", s.convex.flatMap((p) => [p.x, p.y, p.z])];
      else {
        [shape, dims] = ["mesh", s.mesh.points.flatMap((p) => [p.x, p.y, p.z])];
        indices = s.mesh.indices;
      }
      colliders.set(this.id, this);
      __iron_op("physics.collider", this.id, body?.id ?? null, shape, dims, vec(options.position), quat(options.rotation), [
        options.friction ?? 0.5,
        options.restitution ?? 0,
        options.density ?? 1,
        this.sensor,
        true,
        indices
      ]);
    }
    remove() {
      if (!colliders.delete(this.id))
        return;
      __iron_op("physics.removeCollider", this.id);
    }
  }

  class RigidBody {
    id;
    kind;
    position;
    rotation;
    velocity;
    grounded = false;
    onCollisionStart;
    onCollisionEnd;
    constructor(options = {}) {
      this.id = nextBody++;
      this.kind = options.kind ?? "dynamic";
      this.position = { x: 0, y: 0, z: 0, ...options.position };
      this.rotation = { x: 0, y: 0, z: 0, w: 1, ...options.rotation };
      this.velocity = { x: 0, y: 0, z: 0, ...options.velocity };
      bodies.set(this.id, this);
      __iron_op("physics.body", this.id, this.kind, vec(this.position), quat(this.rotation), vec(this.velocity), [
        options.gravityScale ?? 1,
        options.linearDamping ?? 0,
        options.angularDamping ?? 0,
        !!options.ccd,
        !!options.lockRotations
      ]);
    }
    addCollider(options) {
      return new Collider(this, options);
    }
    setPosition(p) {
      Object.assign(this.position, { x: p.x, y: p.y, z: p.z });
      __iron_op("physics.set", this.id, "position", vec(p));
    }
    setRotation(q) {
      Object.assign(this.rotation, { x: q.x, y: q.y, z: q.z, w: q.w });
      __iron_op("physics.set", this.id, "rotation", quat(q));
    }
    setVelocity(v) {
      Object.assign(this.velocity, { x: v.x, y: v.y, z: v.z });
      __iron_op("physics.set", this.id, "velocity", vec(v));
    }
    setAngularVelocity(v) {
      __iron_op("physics.set", this.id, "angularVelocity", vec(v));
    }
    moveTo(p) {
      __iron_op("physics.set", this.id, "target", vec(p));
    }
    applyImpulse(impulse) {
      __iron_op("physics.force", this.id, "impulse", vec(impulse));
    }
    applyTorqueImpulse(impulse) {
      __iron_op("physics.force", this.id, "torqueImpulse", vec(impulse));
    }
    addForce(force) {
      __iron_op("physics.force", this.id, "force", vec(force));
    }
    addTorque(torque) {
      __iron_op("physics.force", this.id, "torque", vec(torque));
    }
    clearForces() {
      __iron_op("physics.clearForces", this.id);
    }
    setEnabled(enabled) {
      __iron_op("physics.set", this.id, "enabled", enabled);
    }
    makeCharacter(options = {}) {
      __iron_op("physics.character", this.id, options.maxSlope ?? 45, options.snapToGround ?? null, options.stepHeight ?? null);
    }
    move(delta) {
      __iron_op("physics.move", this.id, vec(delta));
    }
    remove() {
      if (!bodies.delete(this.id))
        return;
      for (const [id, c] of colliders)
        if (c.body === this)
          colliders.delete(id);
      __iron_op("physics.removeBody", this.id);
    }
  }
  var physics = {
    createBody(options) {
      return new RigidBody(options);
    },
    addCollider(options) {
      return new Collider(null, options);
    },
    setGravity(g) {
      __iron_op("physics.gravity", vec(g));
    },
    raycast(origin, direction, maxDistance = 1000) {
      const id = nextQuery++;
      const len = Math.hypot(direction.x, direction.y, direction.z) || 1;
      const dir = { x: direction.x / len, y: direction.y / len, z: direction.z / len };
      return new Promise((resolve) => {
        queries.set(id, { origin: { ...origin }, dir, resolve });
        __iron_op("physics.raycast", id, vec(origin), vec(dir), maxDistance);
      });
    },
    body(id) {
      return bodies.get(id);
    }
  };
  function report(fn, event) {
    if (!fn)
      return;
    try {
      fn(event);
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      console.error(`[physics] error in collision handler: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
    }
  }
  globalThis.__iron_physics = (moved, contacts, rays, characters) => {
    for (let i = 0;i + 10 < moved.length; i += 11) {
      const b = bodies.get(moved[i]);
      if (!b)
        continue;
      Object.assign(b.position, { x: moved[i + 1], y: moved[i + 2], z: moved[i + 3] });
      Object.assign(b.rotation, { x: moved[i + 4], y: moved[i + 5], z: moved[i + 6], w: moved[i + 7] });
      Object.assign(b.velocity, { x: moved[i + 8], y: moved[i + 9], z: moved[i + 10] });
    }
    for (let i = 0;i + 1 < characters.length; i += 2) {
      const b = bodies.get(characters[i]);
      if (b)
        b.grounded = characters[i + 1] === 1;
    }
    for (let i = 0;i + 3 < contacts.length; i += 4) {
      const a = colliders.get(contacts[i + 1]);
      const b = colliders.get(contacts[i + 2]);
      if (!a || !b)
        continue;
      const started = contacts[i] === 1;
      const sensor = contacts[i + 3] === 1;
      for (const [self, other] of [
        [a, b],
        [b, a]
      ]) {
        const body = self.body;
        if (!body)
          continue;
        report(started ? body.onCollisionStart : body.onCollisionEnd, { collider: self, other, sensor });
      }
    }
    for (let i = 0;i + 5 < rays.length; i += 6) {
      const q = queries.get(rays[i]);
      if (!q)
        continue;
      queries.delete(rays[i]);
      const collider = colliders.get(rays[i + 1]);
      if (!collider) {
        q.resolve(null);
        continue;
      }
      const distance = rays[i + 2];
      q.resolve({
        collider,
        distance,
        point: {
          x: q.origin.x + q.dir.x * distance,
          y: q.origin.y + q.dir.y * distance,
          z: q.origin.z + q.dir.z * distance
        },
        normal: { x: rays[i + 3], y: rays[i + 4], z: rays[i + 5] }
      });
    }
  };

  // ../../js/engine/src/index.ts
  class Component {
    id;
  }
  var classes = new Map;
  var instances = new Map;
  function register(components) {
    for (const [name, cls] of Object.entries(components)) {
      classes.set(name, cls);
    }
  }
  globalThis.__iron_registered = () => [...classes.keys()];
  globalThis.__iron_create = (className, id) => {
    const cls = classes.get(className);
    if (!cls)
      throw new Error(`unknown component class '${className}'`);
    const instance = new cls;
    instance.id = id;
    instances.set(id, instance);
  };
  globalThis.__iron_dispatch = (id, method, ...args) => {
    const instance = instances.get(id);
    if (!instance)
      throw new Error(`no component with id ${id}`);
    const fn = instance[method];
    if (typeof fn === "function")
      return fn.apply(instance, args);
    return;
  };
  globalThis.__iron_destroy = (id) => {
    instances.delete(id);
  };

  // src/physics.ts
  var log = [];

  class Dropper extends Component {
    ball;
    onStart() {
      physics.addCollider({ shape: { box: { x: 10, y: 0.5, z: 10 } }, position: { x: 0, y: -0.5, z: 0 } });
      this.ball = physics.createBody({ position: { x: 0, y: 2, z: 0 } });
      this.ball.addCollider({ shape: { ball: 0.5 } });
      this.ball.onCollisionStart = (e) => {
        log.push(`hit ${e.other.body === null ? "world" : "body"} at y=${this.ball.position.y.toFixed(1)}`);
        physics.raycast({ x: 0, y: 5, z: 0 }, { x: 0, y: -1, z: 0 }).then((hit) => {
          log.push(`ray ${hit?.collider.id} ${hit?.distance.toFixed(2)}`);
        });
      };
    }
  }
  register({ Dropper });
  globalThis.__physics_log = () => log.join(", ");
})();
