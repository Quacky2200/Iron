// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

import { greeting } from "greet";


class Hello extends Component {
  override onStart() {
    console.log(greeting("world", 1));
  }

  override onUpdate(_dt: number) {
    console.log("tick");
  }
}

register({ Hello });
