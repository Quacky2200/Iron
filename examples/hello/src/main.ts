// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { greeting } from "greet";

// Plain code and a timer, no scene: the game stays on the light UI renderer.
console.log(greeting("world", 1));

setInterval(() => {
  console.log("tick");
}, 16);
