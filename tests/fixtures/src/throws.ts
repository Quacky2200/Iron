// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { Component, register } from "@platypus/engine";

class Hello extends Component {
  override onUpdate(_dt: number) {
    console.log(`tick ${this.id}`);
  }
}

class Broken extends Component {
  override onUpdate(_dt: number) {
    throw new Error("broken on purpose");
  }
}

register({ Hello, Broken });
