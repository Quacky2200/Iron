// SPDX-License-Identifier: Apache-2.0

// The vendor bundle of dev builds (`scripts/dev.ts`): React, the reconciler, the Fast
// Refresh runtime, `@iron/engine` and `@iron/react`, loaded once and kept while the
// game's own code (`game.js`, built with those as externals) reloads in place. The game's
// `require(...)` calls get these modules through `__iron_require`.
import { performRefresh } from "./refresh.ts";
import * as engine from "@iron/engine";
import * as React from "react";
import * as jsxDevRuntime from "react/jsx-dev-runtime";
import * as jsxRuntime from "react/jsx-runtime";
import * as ironReact from "./index.ts";

const modules: Record<string, unknown> = {
  react: React,
  "react/jsx-dev-runtime": jsxDevRuntime,
  "react/jsx-runtime": jsxRuntime,
  "@iron/engine": engine,
  "@iron/react": ironReact,
};

declare global {
  var __iron_require: (name: string) => unknown;
}

globalThis.__iron_require = (name: string) => {
  const m = modules[name];
  if (!m) throw new Error(`module ${name} isn't in the vendor bundle (only react, @iron/engine and @iron/react are)`);
  return m;
};

// After each load of the game's code: swap its component classes, then refresh React.
engine._onHotEnd(performRefresh);
