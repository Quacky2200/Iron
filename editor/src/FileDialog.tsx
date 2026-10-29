// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The open / save / folder dialog, inside the editor (file-dialog.ts has the logic): breadcrumbs,
// the folder's entries (folders first; double-click or Enter opens, Backspace goes up), a name
// field (a typed path works too), the type filter, New Folder, and an overwrite check when saving.

import { cx } from "./classes.ts";
import { type DialogMode, type FileFilter, ALL_FILES, resolveTyped, shownEntries, withExtension } from "./file-dialog.ts";
import { closeModal, pushModal } from "./modal.ts";
import { Button, confirmDialog, Modal, textDialog } from "./Modal.tsx";
import { baseName, crumbs, formatSize, joinPath, parentPath, projectFolder } from "./project.ts";
import { VirtualList } from "./VirtualList.tsx";

export interface FileDialogOptions {
  mode: DialogMode;
  title: string;
  /** The type choices (the first is chosen); all files when empty. */
  filters?: FileFilter[];
  /** The folder it starts in ("" is the project's root). */
  dir?: string;
  /** The name it starts with (save). */
  name?: string;
  ok?: string;
}

const ROW = 22;
const LIST_HEIGHT = 260;

function FileDialogView({ options, done }: { options: FileDialogOptions; done: (path: string) => void }) {
  const filters = options.filters && options.filters.length > 0 ? options.filters : [ALL_FILES];
  // A starting folder that doesn't exist yet: the project's root, except when saving (saving
  // makes the folder).
  const [dir, setDir] = useState(() => (options.dir && (options.mode === "save" || Editor.stat(options.dir).dir) ? options.dir : ""));
  const [filterIndex, setFilterIndex] = useState(0);
  const [name, setName] = useState(options.name ?? "");
  const [selected, setSelected] = useState(-1);
  const [problem, setProblem] = useState("");
  const [version, setVersion] = useState(0);
  const filter = filters[filterIndex] ?? ALL_FILES;
  const entries = useMemo(() => {
    const listed = dir === "" || Editor.exists(dir) ? Editor.listDir(dir) : [];
    return shownEntries(listed, filter, options.mode);
  }, [dir, filterIndex, version]);
  const root = projectFolder();
  const rootName = root ? baseName(root) : "Project";
  const rows: ProjectEntry[] = dir ? [{ name: "..", dir: true, size: 0 }, ...entries] : entries;
  const isUp = (i: number) => dir !== "" && i === 0;
  const pathOf = (i: number) => (isUp(i) ? parentPath(dir) : joinPath(dir, rows[i]!.name));

  const go = (to: string) => {
    setDir(to);
    setSelected(-1);
    setProblem("");
  };
  const pick = (i: number) => {
    setSelected(i);
    const row = rows[i];
    if (row && !row.dir && options.mode !== "folder") setName(row.name);
  };
  /** Ends with `path`, asking first when a save would overwrite a file. */
  const finish = (path: string): void => {
    if (options.mode !== "save" || !Editor.exists(path)) {
      done(path);
      return;
    }
    void confirmDialog({
      title: "Replace file?",
      message: `${path} already exists. Replace it?`,
      buttons: ["Replace", "Cancel"],
      cancel: "Cancel",
    }).then((answer) => {
      if (answer === "Replace") done(path);
    });
  };
  const activate = (i: number): void => {
    const row = rows[i];
    if (!row) return;
    if (row.dir) go(pathOf(i));
    else if (options.mode !== "folder") finish(pathOf(i));
  };
  const ok = () => {
    if (options.mode === "folder") {
      const row = rows[selected];
      done(row && row.dir ? pathOf(selected) : dir);
      return;
    }
    const typed = options.mode === "save" ? withExtension(name, filter) : name.trim();
    if (typed === "") {
      setProblem("Choose a file or type a name");
      return;
    }
    const path = resolveTyped(dir, typed);
    if (path === null) {
      setProblem("That path is outside the project");
      return;
    }
    const s = Editor.stat(path);
    if (s.exists && s.dir) {
      go(path);
      setName("");
      return;
    }
    if (options.mode === "open" && !s.exists) {
      setProblem(`${path} doesn't exist`);
      return;
    }
    finish(path);
  };
  const newFolder = () => {
    void textDialog({ title: "New Folder", label: "Folder name", initial: "New Folder", ok: "Create" }).then((folder) => {
      if (folder === "") return;
      const made = Editor.createDir(joinPath(dir, folder));
      if (Editor.lastError !== "") setProblem(Editor.lastError);
      else {
        setVersion((v) => v + 1);
        go(made);
      }
    });
  };

  return (
    <Modal title={options.title} width={560} onCancel={() => done("")} onEnter={ok}>
      <box className="platypus-file-crumbs">
        {crumbs(dir, rootName).map((c, i) => (
          <box key={c.path} className="platypus-project-crumb" onClick={() => go(c.path)}>
            <Text>{i === 0 ? c.name : `/ ${c.name}`}</Text>
          </box>
        ))}
        <box style={{ flexGrow: 1 }} />
        <Button label="New Folder" onClick={newFolder} />
      </box>
      <VirtualList
        className="platypus-file-list"
        count={rows.length}
        rowHeight={ROW}
        height={LIST_HEIGHT}
        selected={selected}
        onSelect={pick}
        empty="Empty folder"
        onKeyDown={(e: PlatypusKeyboardEvent) => {
          if (e.key === "Enter" && selected >= 0 && rows[selected]?.dir) activate(selected);
          else if (e.key === "Backspace" && dir) go(parentPath(dir));
          else return;
          e.preventDefault();
          e.stopPropagation();
        }}
        renderRow={(i) => {
          const row = rows[i]!;
          return (
            <box className={cx("platypus-project-row", row.dir ? "dir" : "")} onDoubleClick={() => activate(i)}>
              <box className={row.dir ? "platypus-project-icon folder" : "platypus-project-icon file"} />
              <Text className="platypus-project-name">{row.name}</Text>
              <Text className="platypus-project-size">{row.dir ? "" : formatSize(row.size)}</Text>
            </box>
          );
        }}
      />
      {options.mode === "folder" ? null : (
        <box className="platypus-file-fields">
          <Text className="platypus-modal-label">Name</Text>
          <Input
            className="platypus-field"
            value={name}
            onChange={(e) => {
              setName(e.value);
              setProblem("");
            }}
          />
        </box>
      )}
      {filters.length > 1 ? (
        <box className="platypus-file-filters">
          {filters.map((f, i) => (
            <box key={f.label} className={i === filterIndex ? "platypus-file-filter on" : "platypus-file-filter"} onClick={() => setFilterIndex(i)}>
              <Text>{f.label}</Text>
            </box>
          ))}
        </box>
      ) : (
        <Text className="platypus-modal-label">{filter.label}</Text>
      )}
      <Text className="platypus-modal-problem">{problem}</Text>
      <box className="platypus-modal-buttons">
        <Button label={options.ok ?? (options.mode === "save" ? "Save" : options.mode === "folder" ? "Choose" : "Open")} primary onClick={ok} />
        <Button label="Cancel" onClick={() => done("")} />
      </box>
    </Modal>
  );
}

/** Shows the file dialog; the chosen project path, or "" when cancelled. */
export function fileDialog(options: FileDialogOptions): Promise<string> {
  return new Promise<string>((resolve) => {
    let id = 0;
    const done = (path: string) => {
      closeModal(id);
      resolve(path);
    };
    id = pushModal(() => <FileDialogView options={options} done={done} />);
  });
}
