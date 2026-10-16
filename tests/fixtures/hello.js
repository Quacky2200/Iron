(() => {
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

  // src/hello.ts
  class Hello extends Component {
    onStart() {
      console.log("hello");
    }
    onUpdate(_dt) {
      console.log("tick");
    }
  }
  register({ Hello });
})();
