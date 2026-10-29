// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// A callback every frame for editor UI that follows the world (the Scene view's icons, a
// live hierarchy): one hidden object with a component whose `onLateUpdate` (after
// scripts and animation have moved things) calls the listeners.

type Listener = () => void;
const listeners = new Set<Listener>();

class EditorFrame extends Component {
  override onLateUpdate(): void {
    for (const f of [...listeners]) f();
  }
}

// Components are registered by name; this one is only added here, never spawned on its own.
register({ EditorFrame }, { spawn: false });

let host: GameObject | null = null;

/** Calls `fn` every frame after `onLateUpdate`; returns a function that stops it. */
export function onEditorFrame(fn: Listener): () => void {
  if (!host || host.destroyed) {
    host = new GameObject("Editor Frame");
    host.hideFlags = HideFlags.HideAndDontSave;
    GameObject.dontDestroyOnLoad(host);
    host.addComponent(EditorFrame);
  }
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}
