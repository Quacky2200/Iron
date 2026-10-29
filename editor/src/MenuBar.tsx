// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// A desktop-style menubar drawn by Platypus's UI: click or Alt+letter opens a menu, hover
// moves between open menus, arrows/Home/End/Enter/Escape navigate, a press outside
// closes, and items' accelerators work from anywhere (menu.ts has the logic).

import { cx } from "./classes.ts";
import {
  acceleratorLabel,
  CLOSED,
  findAccelerator,
  type Menu,
  type MenuItem,
  type MenuState,
  menuKey,
  nextIndex,
  selectable,
} from "./menu.ts";

export interface MenuBarProps {
  menus: Menu[];
  className?: string;
  /** Where to start (closed by default): for screenshots and tests. */
  initialState?: MenuState;
}

const held = (...codes: string[]) => codes.some((c) => input.isKeyDown(c));
const modifiers = () => ({
  ctrl: held("ControlLeft", "ControlRight"),
  shift: held("ShiftLeft", "ShiftRight"),
  alt: held("AltLeft", "AltRight"),
  meta: held("MetaLeft", "MetaRight", "SuperLeft", "SuperRight"),
});

function run(item: MenuItem) {
  try {
    item.action?.();
  } catch (e) {
    console.error(`menu action "${item.label}" failed: ${e instanceof Error ? (e.stack ?? e.message) : e}`);
  }
}

export function MenuBar({ menus, className, initialState = CLOSED }: MenuBarProps) {
  const [state, setState] = useState<MenuState>(initialState);
  const bar = useRef<PlatypusElement>(null);
  // Listeners below read the latest values without resubscribing.
  const latest = useRef({ menus, state });
  latest.current = { menus, state };

  const close = () => {
    setState(CLOSED);
    bar.current?.blur();
  };
  const openTop = (top: number) => {
    const first = nextIndex(menus[top]?.items ?? [], -1, 1);
    setState({ top, open: true, path: first >= 0 ? [first] : [] });
    bar.current?.focus();
  };
  const choose = (item: MenuItem) => {
    if (!selectable(item) || item.submenu) return;
    close();
    run(item);
  };

  // Accelerators and Alt+letter / F10 from anywhere (keys typed into text fields stay theirs).
  useEffect(() => {
    return input.on("keydown", (e) => {
      const { menus, state } = latest.current;
      const mods = modifiers();
      if (e.code === "F10" && !mods.ctrl && !mods.alt) {
        if (state.top >= 0) close();
        else {
          setState({ top: 0, open: false, path: [] });
          bar.current?.focus();
        }
        return;
      }
      if (mods.alt && !mods.ctrl && /^Key[A-Z]$/.test(e.code)) {
        const letter = e.code.slice(3).toLowerCase();
        const top = menus.findIndex((m) => m.label.toLowerCase().startsWith(letter));
        if (top >= 0) {
          openTop(top);
          return;
        }
      }
      const item = findAccelerator(menus, e.code, mods);
      if (item && !e.repeat) {
        close();
        run(item);
      }
    });
  }, []);

  const onKeyDown = (e: PlatypusKeyboardEvent) => {
    const result = menuKey(menus, state, e.key);
    if (!result.handled) return;
    e.preventDefault();
    if (result.state.top < 0) close();
    else setState(result.state);
    if (result.choose) {
      bar.current?.blur();
      run(result.choose);
    }
  };

  const renderList = (items: MenuItem[], level: number): ReactNode => (
    <box
      className={level === 0 ? "platypus-menu" : "platypus-menu platypus-submenu"}
      onPointerDown={(e: PlatypusPointerEvent) => {
        // Not the menubar item's toggle, and the bar keeps focus.
        e.stopPropagation();
        e.preventDefault();
      }}
    >
      {items.map((item, i) => {
        if (item.separator) return <box key={i} className="platypus-menu-separator" />;
        const highlighted = state.path[level] === i;
        const classes = cx("platypus-menu-item", highlighted ? "highlighted" : "", item.disabled ? "disabled" : "");
        return (
          <box
            key={i}
            className={classes}
            onPointerEnter={() => {
              if (!selectable(item)) return;
              setState((s) => ({ ...s, path: [...s.path.slice(0, level), i] }));
            }}
            onClick={(e: PlatypusPointerEvent) => {
              e.stopPropagation();
              choose(item);
            }}
          >
            <Text className="platypus-menu-check">{item.checked ? "✓" : ""}</Text>
            <Text className="platypus-menu-label">{item.label ?? ""}</Text>
            {item.submenu ? (
              <box className="platypus-chevron" />
            ) : (
              <Text className="platypus-menu-accel">{item.accelerator ? acceleratorLabel(item.accelerator) : ""}</Text>
            )}
            {highlighted && item.submenu && !item.disabled ? renderList(item.submenu, level + 1) : null}
          </box>
        );
      })}
    </box>
  );

  return (
    <box
      ref={bar}
      className={className ? `platypus-menubar ${className}` : "platypus-menubar"}
      tabIndex={-1}
      onKeyDown={onKeyDown}
      onBlur={() => {
        // Focus moved elsewhere (a click in the app, Tab): the menus close.
        if (latest.current.state.top >= 0) setState(CLOSED);
      }}
    >
      {state.open ? (
        // Covers the window behind the menus: a press anywhere else closes them.
        <box className="platypus-menu-backdrop" onPointerDown={close} />
      ) : null}
      <box className="platypus-menubar-items">
        {menus.map((menu, i) => (
          <box
            key={menu.label}
            className={i === state.top ? "platypus-menubar-item active" : "platypus-menubar-item"}
            onPointerDown={(e: PlatypusPointerEvent) => {
              if (e.button !== 0) return;
              // Keep the bar's focus (a press would move it to the nearest focusable).
              e.preventDefault();
              if (state.open && state.top === i) close();
              else openTop(i);
            }}
            onPointerEnter={() => {
              // Like desktop menus: once one is open, hovering another opens that one.
              if (state.open && state.top !== i) openTop(i);
            }}
          >
            <Text>{menu.label}</Text>
            {state.open && state.top === i ? renderList(menu.items, 0) : null}
          </box>
        ))}
      </box>
    </box>
  );
}
