// @platypus-dev

// src/refresh.tsx
var import_engine = require("@platypus/engine");
var import_react = require("@platypus/react");
var import_react2 = require("react");
var jsx_dev_runtime = require("react/jsx-dev-runtime");
var bump = () => {};
function Counter() {
  _s();
  const [n, setN] = import_react2.useState(0);
  bump = () => setN((x) => x + 1);
  globalThis.__refresh_seen = `count ${n} (${"v2"})`;
  return /* @__PURE__ */ jsx_dev_runtime.jsxDEV("box", {
    children: /* @__PURE__ */ jsx_dev_runtime.jsxDEV("text", {
      children: `count ${n}`
    }, undefined, false, undefined, this)
  }, undefined, false, undefined, this);
}
var _s = $RefreshSig$();
_s(Counter, "M0WGCa8EjeI=");
$RefreshReg$(Counter, "src/refresh.tsx:Counter");

class Ticker extends import_engine.Component {
  ticks = 0;
  describe() {
    return `${"v2"} ticks=${this.ticks}`;
  }
}
import_engine.register({ Ticker }, { spawn: false });
import_engine.addStyleSheet(`.label { color: red }`);
import_react.createRoot().render(/* @__PURE__ */ jsx_dev_runtime.jsxDEV(Counter, {}, undefined, false, undefined, this));
var g = globalThis;
g.__refresh_bump = () => bump();
g.__refresh_spawn = () => {
  new import_engine.GameObject("ticker").addComponent(Ticker).ticks = 5;
};
g.__refresh_ticker = () => import_engine.GameObject.findObjectOfType(Ticker)?.describe() ?? "none";
