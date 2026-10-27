// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The vendor bundle of dev builds (`scripts/dev.ts`): React, the reconciler, the Fast
// Refresh runtime, `@platypus/engine` and `@platypus/react`, loaded once and kept while the
// game's own code (`game.js`, built with those as externals) reloads in place. The game's
// `require(...)` calls get these modules through `__platypus_require`.
import { performRefresh } from "./refresh.ts";
import * as engine from "@platypus/engine";
import * as React from "react";
import * as jsxDevRuntime from "react/jsx-dev-runtime";
import * as jsxRuntime from "react/jsx-runtime";
import * as platypusReact from "./index.ts";

const modules: Record<string, unknown> = {
  react: React,
  "react/jsx-dev-runtime": jsxDevRuntime,
  "react/jsx-runtime": jsxRuntime,
  "@platypus/engine": engine,
  "@platypus/react": platypusReact,
};

declare global {
  var __platypus_require: (name: string) => unknown;
}

globalThis.__platypus_require = (name: string) => {
  const m = modules[name];
  if (!m) throw new Error(`module ${name} isn't in the vendor bundle (only react, @platypus/engine and @platypus/react are)`);
  return m;
};

// After each load of the game's code: swap its component classes, then refresh React.
engine._onHotEnd(performRefresh);
