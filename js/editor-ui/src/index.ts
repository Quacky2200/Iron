// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

/**
 * @platypus/editor-ui — the editor's UI components, drawn by Platypus's own UI: a menubar,
 * docking panels, a virtualized list, a tree view, a log console and a project
 * browser. The models (`menu.ts`, `dock.ts`, `virtual.ts`, `tree.ts`, `console.ts`,
 * `project.ts`) are plain functions,
 * usable and testable without React.
 */

import "./editor-ui.css";

import { Screen } from "@platypus/engine";
import { useSyncExternalStore } from "react";

export * from "./console.ts";
export { Console, type ConsoleProps } from "./Console.tsx";
export * from "./dock.ts";
export { onEditorFrame } from "./frame.ts";
export * from "./scene-camera.ts";
export * from "./handles.ts";
export {
  drawGizmos,
  iconKind,
  type ObjectPose,
  type SceneIcon,
  type SceneTool,
  SceneView,
  type SceneViewProps,
  visibleObjects,
} from "./SceneView.tsx";
export { Dock, type DockPanel, type DockProps } from "./Dock.tsx";
export { clearLog, logEntries, openInEditor, startLog, subscribeLog } from "./log.ts";
export * from "./menu.ts";
export { MenuBar, type MenuBarProps } from "./MenuBar.tsx";
export * from "./project.ts";
export { ProjectBrowser, type ProjectBrowserProps } from "./ProjectBrowser.tsx";
export * from "./tree.ts";
export { TreeView, type TreeViewProps } from "./TreeView.tsx";
export * from "./virtual.ts";
export { VirtualList, type VirtualListHandle, type VirtualListProps } from "./VirtualList.tsx";

let size = { width: Screen.width, height: Screen.height };
const screenSize = () => {
  if (size.width !== Screen.width || size.height !== Screen.height) size = { width: Screen.width, height: Screen.height };
  return size;
};

/** The window's size in UI pixels, re-rendering when it changes. */
export function useScreenSize(): { width: number; height: number } {
  return useSyncExternalStore(Screen.onChange, screenSize);
}
