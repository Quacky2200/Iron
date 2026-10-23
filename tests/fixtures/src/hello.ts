// SPDX-License-Identifier: Apache-2.0

import { Component, register } from "@iron/engine";

class Hello extends Component {
  override onStart() {
    console.log("hello");
  }

  override onUpdate(_dt: number) {
    console.log("tick");
  }
}

register({ Hello });
