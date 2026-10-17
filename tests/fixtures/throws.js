(() => {
  // ../../js/engine/src/audio.ts
  var playing = new Map;
  var cache = new Map;
  globalThis.__iron_audio = (ended) => {
    for (const id of ended) {
      const p = playing.get(id);
      if (!p)
        continue;
      playing.delete(id);
      p.playing = false;
      try {
        p.onEnded?.();
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[audio] error in onEnded: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
      }
    }
  };
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
  var bodies = new Map;
  var colliders = new Map;
  var queries = new Map;
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
    for (let i = 0;i + 4 < contacts.length; i += 5) {
      const a = colliders.get(contacts[i + 1]);
      const b = colliders.get(contacts[i + 2]);
      if (!a || !b)
        continue;
      const started = contacts[i] === 1;
      const sensor = contacts[i + 3] === 1;
      const speed = contacts[i + 4];
      for (const [self, other] of [
        [a, b],
        [b, a]
      ]) {
        const body = self.body;
        if (!body)
          continue;
        report(started ? body.onCollisionStart : body.onCollisionEnd, { collider: self, other, sensor, speed });
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

  // src/throws.ts
  class Hello extends Component {
    onUpdate(_dt) {
      console.log(`tick ${this.id}`);
    }
  }

  class Broken extends Component {
    onUpdate(_dt) {
      throw new Error("broken on purpose");
    }
  }
  register({ Hello, Broken });
})();
