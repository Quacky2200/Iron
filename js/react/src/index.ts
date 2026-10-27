// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

/**
 * @platypus/react — render React components into Platypus's engine-owned UI tree.
 *
 * ```tsx
 * import { createRoot } from "@platypus/react";
 * createRoot().render(<box style={{ width: 100, height: 100, backgroundColor: "teal" }} />);
 * ```
 */

import { _loadingGame } from "@platypus/engine";
import { createElement, type ReactElement, type ReactNode, type Ref } from "react";
import { ConcurrentRoot } from "react-reconciler/constants";

import "./events.ts";
import type {
  PlatypusAnimationEvent,
  PlatypusDragEvent,
  PlatypusFocusEvent,
  PlatypusInputEvent,
  PlatypusKeyboardEvent,
  PlatypusPointerEvent,
  PlatypusScrollEvent,
  PlatypusTransitionEvent,
  PlatypusWheelEvent,
} from "./events.ts";
import { containerFor, type Instance, reconciler, screenContainer } from "./host.ts";
import type { PlatypusStyle } from "./style.ts";

export type { PlatypusStyle } from "./style.ts";
export {
  type DroppedFile,
  PlatypusAnimationEvent,
  PlatypusDataTransfer,
  PlatypusDragEvent,
  PlatypusEvent,
  PlatypusFocusEvent,
  PlatypusInputEvent,
  PlatypusKeyboardEvent,
  PlatypusPointerEvent,
  PlatypusScrollEvent,
  PlatypusTransitionEvent,
  PlatypusWheelEvent,
} from "./events.ts";
export {
  Instance as PlatypusElement,
  type ScrollBehavior,
  type ScrollIntoViewOptions,
  type ScrollLogicalPosition,
  type ScrollToOptions,
} from "./host.ts";

type PointerHandler = (event: PlatypusPointerEvent) => void;

/**
 * Event handlers, run React-style: `*Capture` handlers from the root down to the
 * target, then the others from the target up. Enter/leave don't bubble.
 */
export interface EventProps {
  onClick?: PointerHandler;
  onClickCapture?: PointerHandler;
  onDoubleClick?: PointerHandler;
  onDoubleClickCapture?: PointerHandler;
  onContextMenu?: PointerHandler;
  onContextMenuCapture?: PointerHandler;
  onPointerDown?: PointerHandler;
  onPointerDownCapture?: PointerHandler;
  onPointerUp?: PointerHandler;
  onPointerUpCapture?: PointerHandler;
  onPointerMove?: PointerHandler;
  onPointerMoveCapture?: PointerHandler;
  onPointerEnter?: PointerHandler;
  onPointerLeave?: PointerHandler;
  onMouseDown?: PointerHandler;
  onMouseDownCapture?: PointerHandler;
  onMouseUp?: PointerHandler;
  onMouseUpCapture?: PointerHandler;
  onMouseMove?: PointerHandler;
  onMouseMoveCapture?: PointerHandler;
  onMouseEnter?: PointerHandler;
  onMouseLeave?: PointerHandler;
  onWheel?: (event: PlatypusWheelEvent) => void;
  onWheelCapture?: (event: PlatypusWheelEvent) => void;
  /** Keys go to the focused element (see `tabIndex`) and bubble. */
  onKeyDown?: (event: PlatypusKeyboardEvent) => void;
  onKeyDownCapture?: (event: PlatypusKeyboardEvent) => void;
  onKeyUp?: (event: PlatypusKeyboardEvent) => void;
  onKeyUpCapture?: (event: PlatypusKeyboardEvent) => void;
  /** Focus arrived on this element or a descendant (bubbles, like React DOM). */
  onFocus?: (event: PlatypusFocusEvent) => void;
  onFocusCapture?: (event: PlatypusFocusEvent) => void;
  /** Focus left this element or a descendant. */
  onBlur?: (event: PlatypusFocusEvent) => void;
  onBlurCapture?: (event: PlatypusFocusEvent) => void;
  /** The element scrolled (by the wheel, keys, its scrollbar or `scrollTo`). Doesn't bubble. */
  onScroll?: (event: PlatypusScrollEvent) => void;
  /** A transition was created, passed its delay, finished, or was replaced / removed. Bubble. */
  onTransitionRun?: (event: PlatypusTransitionEvent) => void;
  onTransitionStart?: (event: PlatypusTransitionEvent) => void;
  onTransitionEnd?: (event: PlatypusTransitionEvent) => void;
  onTransitionCancel?: (event: PlatypusTransitionEvent) => void;
  /** A `@keyframes` animation started, began another iteration, finished, or was removed. Bubble. */
  onAnimationStart?: (event: PlatypusAnimationEvent) => void;
  onAnimationIteration?: (event: PlatypusAnimationEvent) => void;
  onAnimationEnd?: (event: PlatypusAnimationEvent) => void;
  onAnimationCancel?: (event: PlatypusAnimationEvent) => void;
  /** This element took or lost pointer capture (`ref.setPointerCapture`). */
  onGotPointerCapture?: PointerHandler;
  onLostPointerCapture?: PointerHandler;
  /**
   * Drag and drop. On a `draggable` element: `onDragStart` (prevent to cancel; put data
   * in `dataTransfer`), `onDrag` while it moves, `onDragEnd` after (`dropped`). On the
   * element under the pointer: `onDragEnter`, `onDragOver` (prevent to accept a drop),
   * `onDragLeave`, `onDrop`. Files from the system arrive as `dataTransfer.files`.
   */
  onDragStart?: (event: PlatypusDragEvent) => void;
  onDragStartCapture?: (event: PlatypusDragEvent) => void;
  onDrag?: (event: PlatypusDragEvent) => void;
  onDragCapture?: (event: PlatypusDragEvent) => void;
  onDragEnter?: (event: PlatypusDragEvent) => void;
  onDragEnterCapture?: (event: PlatypusDragEvent) => void;
  onDragOver?: (event: PlatypusDragEvent) => void;
  onDragOverCapture?: (event: PlatypusDragEvent) => void;
  onDragLeave?: (event: PlatypusDragEvent) => void;
  onDragLeaveCapture?: (event: PlatypusDragEvent) => void;
  onDrop?: (event: PlatypusDragEvent) => void;
  onDropCapture?: (event: PlatypusDragEvent) => void;
  onDragEnd?: (event: PlatypusDragEvent) => void;
  onDragEndCapture?: (event: PlatypusDragEvent) => void;
}

/** Attributes that CSS selectors can match. */
export interface ElementAttrs {
  className?: string;
  id?: string;
  disabled?: boolean;
  checked?: boolean;
  /** A press and move starts dragging the element (`onDragStart`, ...). */
  draggable?: boolean;
  /**
   * Makes the element focusable: 0 joins the Tab order in tree order, positive values
   * come first (ascending), negative values focus by click or `ref.focus()` only.
   */
  tabIndex?: number;
  /** Text direction, as `direction` in CSS (which wins when both are set). */
  dir?: "ltr" | "rtl";
  [data: `data-${string}`]: string | number | boolean | undefined;
}

export interface BoxProps extends ElementAttrs, EventProps {
  style?: PlatypusStyle;
  children?: ReactNode;
  key?: string | number | null;
  ref?: Ref<Instance>;
}

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      box: BoxProps;
      /** A list: element children are items with markers (`listStyle*`, `::marker`). */
      list: BoxProps;
    }
  }
}

export interface TextProps extends ElementAttrs, EventProps {
  style?: PlatypusStyle;
  children?: ReactNode;
  ref?: Ref<Instance>;
}

// React's own JSX types already define <text> (SVG) and <span> (HTML), so Platypus's
// text elements are exposed as components. CSS type selectors still use `text`
// and `span`.

/** A block of text. Strings and `Span`s inside it flow and wrap together. */
export function Text({ children, ...props }: TextProps): ReactElement {
  return createElement("text" as never, props, children);
}

/** Props shared by `Input` and `TextArea`. */
export interface FieldProps extends Omit<ElementAttrs, "checked">, EventProps {
  style?: PlatypusStyle;
  /** Controlled text: the field shows this and `onChange` reports edits. */
  value?: string;
  /** Initial text of an uncontrolled field. */
  defaultValue?: string;
  /** Shown dimmed while the field is empty. */
  placeholder?: string;
  readOnly?: boolean;
  /** Every edit (typing, deleting, pasting); `e.target.value` is the new text. */
  onChange?: (event: PlatypusInputEvent) => void;
  onInput?: (event: PlatypusInputEvent) => void;
  ref?: Ref<Instance>;
}

/** A one-line text field (CSS type selector `input`). Focusable by default. */
export function Input(props: FieldProps): ReactElement {
  return createElement("input" as never, props);
}

/** A multi-line text field (CSS type selector `textarea`); `rows` sets its default height. */
export function TextArea(props: FieldProps & { rows?: number }): ReactElement {
  return createElement("textarea" as never, props);
}

export interface ImageProps extends ElementAttrs, EventProps {
  style?: PlatypusStyle;
  /** A path inside the game folder (like `loadFont`) or a `data:` URL. */
  src?: string;
  ref?: Ref<Instance>;
}

/**
 * An image (CSS type selector `image`). Without a set size it takes the image's
 * natural size; `objectFit` and `objectPosition` fit it into its content box.
 */
export function Image(props: ImageProps): ReactElement {
  return createElement("image" as never, props);
}

/**
 * A list (CSS type selector `list`): a column whose element children are list items, each
 * with a marker set by `listStyleType` / `listStylePosition` / `listStyleImage` and styled
 * with `::marker`.
 */
export function List({ children, ...props }: BoxProps): ReactElement {
  return createElement("list" as never, props, children);
}

export interface DialogProps extends ElementAttrs, EventProps {
  style?: PlatypusStyle;
  children?: ReactNode;
  /** Shown while true; closed dialogs are `display: none`. */
  open?: boolean;
  /**
   * A modal dialog goes in the top layer above everything (`position: fixed`, over a
   * window-sized `::backdrop`), takes focus and keeps Tab inside until it closes.
   */
  modal?: boolean;
  /**
   * What else may close it: `"closerequest"` (Escape; the default), `"any"` (Escape and
   * clicks on the backdrop) or `"none"`. Each sends `onCancel` (prevent it to stay
   * open), then `onClose`, where you set `open` to false.
   */
  closedBy?: "any" | "closerequest" | "none";
  onCancel?: (event: PlatypusPointerEvent) => void;
  onClose?: (event: PlatypusPointerEvent) => void;
  ref?: Ref<Instance>;
}

/**
 * A dialog (CSS type selector `dialog`; `dialog[open]`, `dialog[modal]` and
 * `dialog::backdrop` select it). Centred in its containing block, or the window when
 * `modal`; unstyled otherwise.
 */
export function Dialog({ children, ...props }: DialogProps): ReactElement {
  return createElement("dialog" as never, props, children);
}

/** An inline run inside `Text` with its own style (colour, weight, ...). */
export function Span({ children, ...props }: TextProps): ReactElement {
  return createElement("span" as never, props, children);
}

export interface Root {
  /** Renders `element`, committing synchronously so the UI updates this frame. */
  render(element: ReactNode): void;
  unmount(): void;
}

const rootsCreated = new Set<number>();
/** Live roots by target, for reloads in place. */
const roots = new Map<number, Root>();

const reportError = (kind: string) => (error: unknown) => {
  const e = error instanceof Error ? error : new Error(String(error));
  console.error(`[react] ${kind}: ${e.message}${e.stack ? `\n${e.stack}` : ""}`);
};

/** The UI target ids of windows: `WINDOW_TARGET | handle` (`platypus_core::WINDOW_TARGET`); the main window is the screen. */
const WINDOW_TARGET = 0x4000_0000;

/** What `createRoot` takes: the screen (nothing, or `Window.main`), a world panel (its component or id) or a window. */
export type RootTarget = { readonly id: number } | { readonly handle: number } | number;

function targetId(target?: RootTarget): number {
  if (typeof target === "number") return target;
  if (target === undefined) return 0;
  if ("id" in target) return target.id;
  // A `Window` (its handle: 1 is the main window, which is the screen).
  return target.handle === 1 ? 0 : WINDOW_TARGET | target.handle;
}

function rootError(id: number): string {
  if (id === 0) return "createRoot() can only be called once per game";
  if (id >= WINDOW_TARGET) return `window ${id - WINDOW_TARGET} already has a root`;
  return `panel ${id} already has a root`;
}

/**
 * Creates a UI root: the screen's (no argument), or a world panel's (pass the
 * `WorldPanel` component, or its id) to draw React UI on a plane in the world. Each
 * target has one root; render more UI by composing components.
 */
export function createRoot(target?: RootTarget): Root {
  const id = targetId(target);
  // The game's code running again (a dev reload in place): its root is the old one, so
  // React keeps the tree and Fast Refresh updates it.
  const existing = roots.get(id);
  if (existing && _loadingGame()) return existing;
  if (rootsCreated.has(id)) {
    throw new Error(rootError(id));
  }
  rootsCreated.add(id);

  const container = id === 0 ? screenContainer : containerFor(id);
  let rendered = false;
  const root = reconciler.createContainer(
    container,
    ConcurrentRoot,
    null,
    false,
    null,
    "platypus",
    reportError("uncaught error"),
    reportError("caught error"),
    reportError("recoverable error"),
    () => {},
    null,
  );

  const handle: Root = {
    render(element) {
      // The game's code loading again (a reload in place): the tree stays, and Fast
      // Refresh updates the components whose code changed, keeping their state.
      if (_loadingGame() && rendered) return;
      rendered = true;
      reconciler.updateContainerSync(element, root, null, null);
      reconciler.flushSyncWork();
    },
    unmount() {
      reconciler.updateContainerSync(null, root, null, null);
      reconciler.flushSyncWork();
      rootsCreated.delete(id);
      roots.delete(id);
    },
  };
  roots.set(id, handle);
  return handle;
}

/** Lets tests create fresh roots. Not part of the game API. */
export function _resetRootForTests() {
  rootsCreated.clear();
  roots.clear();
}
