// The project browser: the project folder's files, navigable by folder. Files dragged
// in from the system are copied into the folder they're dropped on (or the one shown);
// items dragged inside it move into a folder (a row, a breadcrumb, or `..`).

import { type IronDragEvent, Text } from "@iron/react";
import { useCallback, useEffect, useState } from "react";

import {
  baseName,
  canMoveInto,
  copyIn,
  crumbs,
  formatSize,
  joinPath,
  listDir,
  moveInto,
  PROJECT_PATH_TYPE,
  type ProjectEntry,
  parentPath,
  projectFolder,
} from "./project.ts";
import { VirtualList } from "./VirtualList.tsx";

export interface ProjectBrowserProps {
  width: number;
  height: number;
  rowHeight?: number;
}

const TOOLBAR = 28;

/** What a drag over a folder would do there, if anything. */
function dropAction(e: IronDragEvent, dir: string): "copy" | "move" | null {
  if (e.dataTransfer.files.length) return "copy";
  const from = e.dataTransfer.getData(PROJECT_PATH_TYPE);
  return from && canMoveInto(from, dir) ? "move" : null;
}

export function ProjectBrowser({ width, height, rowHeight = 22 }: ProjectBrowserProps) {
  const [path, setPath] = useState("");
  const [entries, setEntries] = useState<ProjectEntry[]>([]);
  const [selected, setSelected] = useState(-1);
  const [status, setStatus] = useState("");
  // The folder a drop would go into, highlighted while dragging over it.
  const [over, setOver] = useState<string | null>(null);
  const root = projectFolder();
  const rootName = root ? baseName(root) : "Project";

  const refresh = useCallback(
    (dir = path) =>
      listDir(dir).then(
        (l) => {
          setEntries(l.entries);
          setStatus(`${l.entries.length} items`);
        },
        (e: Error) => setStatus(e.message),
      ),
    [path],
  );
  useEffect(() => {
    if (root) void refresh(path);
    else setStatus("No project folder (run iron_editor <folder>)");
  }, [path, root]);

  const open = (dir: string) => {
    setSelected(-1);
    setPath(dir);
  };

  /** Drag handlers that make `dir` a drop target. */
  const target = (dir: string) => ({
    onDragOver: (e: IronDragEvent) => {
      const action = dropAction(e, dir);
      if (!action) return;
      e.preventDefault();
      // The innermost folder wins over the list behind it.
      e.stopPropagation();
      e.dataTransfer.dropEffect = action;
      setOver(dir);
    },
    onDrop: (e: IronDragEvent) => {
      const action = dropAction(e, dir);
      if (!action) return;
      e.stopPropagation();
      setOver(null);
      const done =
        action === "copy"
          ? copyIn(
              e.dataTransfer.files.map((f) => f.path),
              dir,
            ).then((r) => `Copied ${r.created.length} item${r.created.length === 1 ? "" : "s"} into ${dir || rootName}`)
          : moveInto(e.dataTransfer.getData(PROJECT_PATH_TYPE), dir).then((r) => `Moved to ${r.path}`);
      done.then(
        (message) => {
          setStatus(message);
          console.log(message);
          void refresh();
        },
        (err: Error) => {
          setStatus(err.message);
          console.warn(`Project: ${err.message}`);
        },
      );
    },
  });

  // `..` first when inside a folder.
  const rows: (ProjectEntry & { up?: true })[] = path ? [{ name: "..", dir: true, size: 0, up: true }, ...entries] : entries;
  const listHeight = Math.max(0, height - TOOLBAR);

  return (
    <box
      className="iron-project"
      style={{ width, height }}
      onDragLeave={() => setOver(null)}
      onDragEnd={() => setOver(null)}
      {...target(path)}
    >
      <box className="iron-project-toolbar" style={{ height: TOOLBAR }}>
        {crumbs(path, rootName).map((c, i) => (
          <box key={c.path} className={over === c.path && c.path !== path ? "iron-project-crumb over" : "iron-project-crumb"} onClick={() => open(c.path)} {...target(c.path)}>
            <Text>{i === 0 ? c.name : `/ ${c.name}`}</Text>
          </box>
        ))}
        <box style={{ flexGrow: 1 }} />
        <Text className="iron-project-status">{status}</Text>
        <box className="iron-project-button" onClick={() => void refresh()}>
          <Text>Refresh</Text>
        </box>
      </box>
      <VirtualList
        className={over === path ? "iron-project-list over" : "iron-project-list"}
        count={rows.length}
        rowHeight={rowHeight}
        height={listHeight}
        selected={selected}
        onSelect={setSelected}
        empty={root ? "Empty folder: drop files here" : ""}
        onKeyDown={(e) => {
          const row = rows[selected];
          if (e.key === "Enter" && row?.dir) open(row.up ? parentPath(path) : joinPath(path, row.name));
          else if (e.key === "Backspace" && path) open(parentPath(path));
          else return;
          e.preventDefault();
        }}
        renderRow={(i) => {
          const row = rows[i]!;
          const full = row.up ? parentPath(path) : joinPath(path, row.name);
          const classes = ["iron-project-row", row.dir && "dir", row.dir && over === full && "over"].filter(Boolean).join(" ");
          return (
            <box
              className={classes}
              draggable={!row.up}
              onDragStart={(e: IronDragEvent) => {
                e.dataTransfer.setData(PROJECT_PATH_TYPE, full);
                e.dataTransfer.setData("text/plain", full);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDoubleClick={() => row.dir && open(full)}
              {...(row.dir ? target(full) : {})}
            >
              <box className={row.dir ? "iron-project-icon folder" : "iron-project-icon file"} />
              <Text className="iron-project-name">{row.name}</Text>
              <Text className="iron-project-size">{row.dir ? "" : formatSize(row.size)}</Text>
            </box>
          );
        }}
      />
    </box>
  );
}
