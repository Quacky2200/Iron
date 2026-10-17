// A desktop-style menubar drawn by Iron's UI: click or Alt+letter opens a menu, hover
// moves between open menus, arrows/Home/End/Enter/Escape navigate, a press outside
// closes, and items' accelerators work from anywhere (menu.ts has the logic).

import { input } from "@iron/engine";
import { type IronElement, type IronKeyboardEvent, type IronPointerEvent, Text } from "@iron/react";
import { useEffect, useRef, useState } from "react";

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
  const bar = useRef<IronElement>(null);
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

  const onKeyDown = (e: IronKeyboardEvent) => {
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

  const renderList = (items: MenuItem[], level: number) => (
    <box
      className={level === 0 ? "iron-menu" : "iron-menu iron-submenu"}
      onPointerDown={(e: IronPointerEvent) => {
        // Not the menubar item's toggle, and the bar keeps focus.
        e.stopPropagation();
        e.preventDefault();
      }}
    >
      {items.map((item, i) => {
        if (item.separator) return <box key={i} className="iron-menu-separator" />;
        const highlighted = state.path[level] === i;
        const classes = ["iron-menu-item", highlighted && "highlighted", item.disabled && "disabled"].filter(Boolean).join(" ");
        return (
          <box
            key={i}
            className={classes}
            onPointerEnter={() => {
              if (!selectable(item)) return;
              setState((s) => ({ ...s, path: [...s.path.slice(0, level), i] }));
            }}
            onClick={(e: IronPointerEvent) => {
              e.stopPropagation();
              choose(item);
            }}
          >
            <Text className="iron-menu-check">{item.checked ? "✓" : ""}</Text>
            <Text className="iron-menu-label">{item.label ?? ""}</Text>
            {item.submenu ? (
              <box className="iron-chevron" />
            ) : (
              <Text className="iron-menu-accel">{item.accelerator ? acceleratorLabel(item.accelerator) : ""}</Text>
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
      className={className ? `iron-menubar ${className}` : "iron-menubar"}
      tabIndex={-1}
      onKeyDown={onKeyDown}
      onBlur={() => {
        // Focus moved elsewhere (a click in the app, Tab): the menus close.
        if (latest.current.state.top >= 0) setState(CLOSED);
      }}
    >
      {state.open ? (
        // Covers the window behind the menus: a press anywhere else closes them.
        <box className="iron-menu-backdrop" onPointerDown={close} />
      ) : null}
      <box className="iron-menubar-items">
        {menus.map((menu, i) => (
          <box
            key={menu.label}
            className={i === state.top ? "iron-menubar-item active" : "iron-menubar-item"}
            onPointerDown={(e: IronPointerEvent) => {
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
