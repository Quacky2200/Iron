// The Iron editor's UI (M5.e): a menubar, docked panels (Hierarchy, Scene, Inspector,
// Console) and a status bar, built with @iron/editor-ui. `iron_editor` loads this
// bundle (editor/dist). The Scene and Inspector are placeholders for now.
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
  ProjectBrowser,
  parseLayout,
  serializeLayout,
  split,
  type TreeNode,
  type TreeState,
  TreeView,
  useScreenSize,
} from "@iron/editor-ui";
import { Application, PlayerPrefs } from "@iron/engine";
import { createRoot, Text } from "@iron/react";
import { useMemo, useState } from "react";

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
  kind: "scene" | "object" | "light" | "camera" | "mesh";
}

// A demo hierarchy, big enough to show the virtual list at work (about 20,000 nodes).
function demoHierarchy(): TreeNode<Thing>[] {
  const kinds: Thing["kind"][] = ["object", "mesh", "light", "mesh", "object"];
  const children = (prefix: string, depth: number, width: number): TreeNode<Thing>[] =>
    Array.from({ length: width }, (_, i) => {
      const id = `${prefix}/${i}`;
      const kind = kinds[(i + depth) % kinds.length]!;
      return {
        id,
        label: `${kind === "mesh" ? "Mesh" : kind === "light" ? "Light" : "GameObject"} ${i + 1}`,
        data: { kind },
        children: depth < 3 ? children(id, depth + 1, width) : undefined,
      };
    });
  return [
    {
      id: "scene",
      label: "Main Scene",
      data: { kind: "scene" },
      children: [
        { id: "camera", label: "Main Camera", data: { kind: "camera" } },
        { id: "sun", label: "Directional Light", data: { kind: "light" } },
        { id: "world", label: "World", data: { kind: "object" }, children: children("w", 0, 12) },
        { id: "player", label: "Player", data: { kind: "object" }, children: children("p", 2, 3) },
      ],
    },
  ];
}

const ICONS: Record<Thing["kind"], string> = { scene: "◆", object: "■", light: "☀", camera: "◉", mesh: "▲" };

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
  const [roots, setRoots] = useState(demoHierarchy);
  const [tree, setTree] = useState<TreeState>(() => ({ ...emptyTreeState(), expanded: new Set(["scene", "world"]) }));
  const [status, setStatus] = useState("Ready");

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
        { label: "Show Grid", checked: true, action: todo("Grid") },
        { label: "Show Gizmos", checked: true, action: todo("Gizmos") },
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
        { label: "About Iron", action: () => say(`${Application.productName} ${Application.version}`) },
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
              setRoots((r) => rename(r, node.id, name));
              say(`Renamed ${node.label} to ${name}`);
            }}
          />
        ),
      },
      project: { title: TITLES.project!, render: (s) => <ProjectBrowser {...s} /> },
      scene: { title: TITLES.scene!, render: (s) => <Placeholder title="Scene" note="Scene view: coming with the editor's camera" {...s} /> },
      game: { title: TITLES.game!, render: (s) => <Placeholder title="Game" note="Play mode: later in M5.e" {...s} /> },
      inspector: {
        title: TITLES.inspector!,
        render: (s) => (
          <Placeholder
            title="Inspector"
            note={tree.selected.size === 0 ? "Nothing selected" : `${tree.selected.size} selected: ${[...tree.selected].slice(0, 3).join(", ")}`}
            {...s}
          />
        ),
      },
      console: { title: TITLES.console!, render: (s) => <Console {...s} /> },
    }),
    [roots, tree],
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

function rename<T>(nodes: TreeNode<T>[], id: string, label: string): TreeNode<T>[] {
  return nodes.map((n) =>
    n.id === id ? { ...n, label } : n.children ? { ...n, children: rename(n.children, id, label) } : n,
  );
}

createRoot().render(<App />);
console.log("Iron editor started (UI: editor/src/main.tsx:1)");
console.warn("The Scene, Game and Inspector panels are placeholders (TODO.md:1180)");
