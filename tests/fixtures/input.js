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
  var input = {
    isKeyDown(code) {
      return keys.has(code);
    },
    gamepads() {
      return [...pads.values()].filter((p) => p.connected);
    },
    gamepad(id) {
      const p = pads.get(id);
      return p?.connected ? p : undefined;
    },
    isButtonDown(button) {
      return input.gamepads().some((p) => (p.buttons[button] ?? 0) > 0.5);
    },
    on(type, fn) {
      let set = listeners[type];
      if (!set) {
        set = new Set;
        listeners[type] = set;
      }
      set.add(fn);
      return () => set.delete(fn);
    }
  };
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
  // ../../js/engine/src/index.ts
  var classes = new Map;
  var instances = new Map;
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

  // src/input.ts
  var log = [];
  input.on("gamepadconnected", (e) => log.push(`connected ${e.gamepad.name}`));
  input.on("gamepadbuttondown", (e) => log.push(`down ${e.button}`));
  input.on("gamepadaxis", (e) => log.push(`${e.axis} ${e.value}`));
  input.on("keydown", (e) => log.push(`key ${e.code}`));
  globalThis.__input_log = () => [...log, `held ${input.isButtonDown("South")}`].join(", ");
})();
