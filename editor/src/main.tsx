// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Platypus editor's UI (M5.e): a menubar, docked panels (Hierarchy, Scene, Game,
// Inspector, Console, Project) and a status bar, built with @platypus/editor-ui.
// `platypus_editor` loads this bundle (editor/dist). The Hierarchy and Scene view show the
// editor's own world (a demo scene for now); the Inspector is a placeholder.
import "./editor.css";

import {
  Console,
  Dock,
  type DockNode,
  type DockPanel,
  emptyTreeState,
  group,
  type Menu,
  MenuBar,
  onEditorFrame,
  ProjectBrowser,
  parseLayout,
  type Rect,
  SceneView,
  serializeLayout,
  split,
  type TreeNode,
  type TreeState,
  TreeView,
  useScreenSize,
  visibleObjects,
} from "@platypus/editor-ui";
import { Application, AudioSource, Camera, GameObject, Light, MeshRenderer, PlayerPrefs, Rect as ViewRect, ReflectionProbe, Screen } from "@platypus/engine";
import { createRoot, Text } from "@platypus/react";
import { useEffect, useMemo, useState } from "react";

import { buildDemoScene } from "./demo-scene.ts";

const MENUBAR = 26;
const STATUS = 22;
const PANELS = ["hierarchy", "scene", "game", "inspector", "console", "project"];
const LAYOUT_KEY = "editor.layout";

const defaultLayout = (): DockNode =>
  split("root", "column", [
    split("top", "row", [group("left", ["hierarchy"]), group("center", ["scene", "game"]), group("right", ["inspector"])], [0.2, 0.58, 0.22]),
    group("bottom", ["console", "project"]),
  ], [0.7, 0.3]);

function savedLayout(): DockNode {
  const saved = PlayerPrefs.getString(LAYOUT_KEY, "");
  return (saved && parseLayout(saved, PANELS)) || defaultLayout();
}

interface Thing {
  kind: "object" | "light" | "camera" | "mesh" | "audio" | "probe";
  object: GameObject;
}

function kindOf(go: GameObject): Thing["kind"] {
  if (go.getComponent(Light)) return "light";
  if (go.getComponent(Camera)) return "camera";
  if (go.getComponent(MeshRenderer)) return "mesh";
  if (go.getComponent(AudioSource)) return "audio";
  if (go.getComponent(ReflectionProbe)) return "probe";
  return "object";
}

/** The world's objects as tree nodes (hidden ones left out), keyed by object id. */
function sceneTree(): TreeNode<Thing>[] {
  const shown = new Set(visibleObjects());
  const node = (go: GameObject): TreeNode<Thing> => {
    const children = go.children.filter((c) => shown.has(c));
    return {
      id: String(go.id),
      label: go.name,
      data: { kind: kindOf(go), object: go },
      children: children.length > 0 ? children.map(node) : undefined,
    };
  };
  return GameObject.roots.filter((g) => shown.has(g)).map(node);
}

/** The live hierarchy: rebuilt when objects, names or parents change. */
function useSceneTree(): TreeNode<Thing>[] {
  const [roots, setRoots] = useState(sceneTree);
  useEffect(() => {
    let last = "";
    return onEditorFrame(() => {
      const key = visibleObjects()
        .map((g) => `${g.id}:${g.parent?.id ?? ""}:${g.name}`)
        .join("|");
      if (key !== last) {
        last = key;
        setRoots(sceneTree());
      }
    });
  }, []);
  return roots;
}

const ICONS: Record<Thing["kind"], string> = { object: "■", light: "☀", camera: "◉", mesh: "▲", audio: "♪", probe: "◍" };

/** The Game panel: the main camera draws into it while it's shown. */
function GameView({ rect }: { rect: Rect }) {
  useEffect(() => {
    const camera = Camera.main;
    if (!camera) return;
    const sw = Math.max(1, Screen.width);
    const sh = Math.max(1, Screen.height);
    camera.rect = new ViewRect(rect.x / sw, rect.y / sh, rect.width / sw, rect.height / sh);
    return () => {
      if (!camera.gameObject.destroyed) camera.rect = new ViewRect(0, 0, 0, 0);
    };
  }, [rect.x, rect.y, rect.width, rect.height]);
  return <box style={{ width: rect.width, height: rect.height }} />;
}

function Placeholder({ title, note, width, height }: { title: string; note: string; width: number; height: number }) {
  return (
    <box className="placeholder" style={{ width, height }}>
      <Text className="placeholder-title">{title}</Text>
      <Text className="placeholder-note">{note}</Text>
    </box>
  );
}

function App() {
  const screen = useScreenSize();
  const [layout, setLayout] = useState<DockNode>(savedLayout);
  const roots = useSceneTree();
  const [tree, setTree] = useState<TreeState>(() => ({
    ...emptyTreeState(),
    expanded: new Set(roots.filter((r) => r.children).map((r) => r.id)),
  }));
  const [status, setStatus] = useState("Ready");
  const [showGrid, setShowGrid] = useState(true);
  const [showGizmos, setShowGizmos] = useState(true);

  const say = (text: string) => {
    setStatus(text);
    console.log(text);
  };
  const saveLayout = () => {
    PlayerPrefs.setString(LAYOUT_KEY, serializeLayout(layout));
    PlayerPrefs.save();
    say("Layout saved");
  };
  const todo = (what: string) => () => say(`${what}: not implemented yet (see TODO.md M5.e)`);

  const selectedIds = [...tree.selected];
  const byId = new Map(visibleObjects().map((g) => [String(g.id), g]));
  const selectedObjects = selectedIds.map((id) => byId.get(id)).filter((g): g is GameObject => !!g && !g.destroyed);
  const select = (go: GameObject | null, additive: boolean) => {
    if (!go) {
      if (!additive) setTree((t) => ({ ...t, selected: new Set() }));
      return;
    }
    const id = String(go.id);
    setTree((t) => {
      const selected = new Set(additive ? t.selected : []);
      if (additive && selected.has(id)) selected.delete(id);
      else selected.add(id);
      return { ...t, selected, focus: id, anchor: id };
    });
  };
  const menus: Menu[] = [
    {
      label: "File",
      items: [
        { label: "New Scene", accelerator: "Ctrl+N", action: todo("New Scene") },
        { label: "Open Scene...", accelerator: "Ctrl+O", action: todo("Open Scene") },
        {
          label: "Open Recent",
          submenu: [
            { label: "scenes/main.scene.json", action: todo("Open main.scene.json") },
            { label: "scenes/deluxe.scene.json", action: todo("Open deluxe.scene.json") },
            { separator: true },
            { label: "Clear Recent", disabled: true },
          ],
        },
        { separator: true },
        { label: "Save", accelerator: "Ctrl+S", action: todo("Save") },
        { label: "Save As...", accelerator: "Ctrl+Shift+S", action: todo("Save As") },
        { separator: true },
        { label: "Exit", accelerator: "Alt+F4", action: () => Application.quit() },
      ],
    },
    {
      label: "Edit",
      items: [
        { label: "Undo", accelerator: "Ctrl+Z", action: todo("Undo") },
        { label: "Redo", accelerator: "Ctrl+Y", action: todo("Redo") },
        { separator: true },
        { label: "Cut", accelerator: "Ctrl+X", disabled: selectedIds.length === 0, action: todo("Cut") },
        { label: "Copy", accelerator: "Ctrl+C", disabled: selectedIds.length === 0, action: todo("Copy") },
        { label: "Paste", accelerator: "Ctrl+V", action: todo("Paste") },
        { label: "Delete", accelerator: "Delete", disabled: selectedIds.length === 0, action: todo("Delete") },
        { separator: true },
        { label: "Select All", accelerator: "Ctrl+A", action: todo("Select All") },
        { label: "Preferences...", action: todo("Preferences") },
      ],
    },
    {
      label: "View",
      items: [
        {
          label: "Panels",
          submenu: PANELS.map((id) => ({ label: TITLES[id], action: () => say(`${TITLES[id]} panel`) })),
        },
        { separator: true },
        { label: "Show Grid", checked: showGrid, action: () => setShowGrid((v) => !v) },
        { label: "Show Gizmos", checked: showGizmos, action: () => setShowGizmos((v) => !v) },
      ],
    },
    {
      label: "Window",
      items: [
        { label: "Save Layout", action: saveLayout },
        {
          label: "Load Layout",
          action: () => {
            setLayout(savedLayout());
            say("Layout loaded");
          },
        },
        {
          label: "Reset Layout",
          action: () => {
            setLayout(defaultLayout());
            say("Layout reset");
          },
        },
        { separator: true },
        { label: "Log Layout JSON", action: () => console.log(serializeLayout(layout)) },
      ],
    },
    {
      label: "Help",
      items: [
        { label: "Documentation", accelerator: "F1", action: () => say("Docs: docs/editor.md:1") },
        { label: "About Platypus", action: () => say(`${Application.productName} ${Application.version}`) },
      ],
    },
  ];

  const panels: Record<string, DockPanel> = useMemo(
    () => ({
      hierarchy: {
        title: TITLES.hierarchy!,
        render: ({ height }) => (
          <TreeView
            roots={roots}
            state={tree}
            onStateChange={setTree}
            height={height}
            renderIcon={(node) => <Text className={`icon ${node.data?.kind ?? ""}`}>{ICONS[node.data?.kind ?? "object"]}</Text>}
            onRename={(node, name) => {
              if (node.data) node.data.object.name = name;
              say(`Renamed ${node.label} to ${name}`);
            }}
          />
        ),
      },
      project: { title: TITLES.project!, render: (s) => <ProjectBrowser {...s} /> },
      scene: {
        title: TITLES.scene!,
        seeThrough: true,
        render: (r) => <SceneView rect={r} selected={selectedObjects} onSelect={select} grid={showGrid} gizmos={showGizmos} />,
      },
      game: { title: TITLES.game!, seeThrough: true, render: (r) => <GameView rect={r} /> },
      inspector: {
        title: TITLES.inspector!,
        render: (s) => (
          <Placeholder
            title="Inspector"
            note={
              selectedObjects.length === 0
                ? "Nothing selected"
                : `${selectedObjects.length} selected: ${selectedObjects
                    .slice(0, 3)
                    .map((g) => g.name)
                    .join(", ")}`
            }
            {...s}
          />
        ),
      },
      console: { title: TITLES.console!, render: (s) => <Console {...s} /> },
    }),
    [roots, tree, showGrid, showGizmos],
  );

  const dockRect = { x: 0, y: MENUBAR, width: screen.width, height: Math.max(0, screen.height - MENUBAR - STATUS) };
  return (
    <box className="editor">
      <MenuBar menus={menus} />
      <Dock layout={layout} onLayoutChange={setLayout} panels={panels} rect={dockRect} />
      <box className="statusbar">
        <Text>{status}</Text>
        <box style={{ flexGrow: 1 }} />
        <Text>{`${tree.selected.size} selected`}</Text>
      </box>
    </box>
  );
}

const TITLES: Record<string, string> = {
  hierarchy: "Hierarchy",
  project: "Project",
  scene: "Scene",
  game: "Game",
  inspector: "Inspector",
  console: "Console",
};

buildDemoScene();
createRoot().render(<App />);
console.log("Platypus editor started (UI: editor/src/main.tsx:1)");
console.warn("The Inspector is a placeholder (TODO.md M5.e)");
