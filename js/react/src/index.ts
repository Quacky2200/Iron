/**
 * @iron/react — render React components into Iron's engine-owned UI tree.
 *
 * ```tsx
 * import { createRoot } from "@iron/react";
 * createRoot().render(<box style={{ width: 100, height: 100, backgroundColor: "teal" }} />);
 * ```
 */

import { _loadingGame } from "@iron/engine";
import { createElement, type ReactElement, type ReactNode, type Ref } from "react";
import { ConcurrentRoot } from "react-reconciler/constants";

import "./events.ts";
import type {
  IronAnimationEvent,
  IronDragEvent,
  IronFocusEvent,
  IronInputEvent,
  IronKeyboardEvent,
  IronPointerEvent,
  IronScrollEvent,
  IronTransitionEvent,
  IronWheelEvent,
} from "./events.ts";
import { containerFor, type Instance, reconciler, screenContainer } from "./host.ts";
import type { IronStyle } from "./style.ts";

export type { IronStyle } from "./style.ts";
export {
  type DroppedFile,
  IronAnimationEvent,
  IronDataTransfer,
  IronDragEvent,
  IronEvent,
  IronFocusEvent,
  IronInputEvent,
  IronKeyboardEvent,
  IronPointerEvent,
  IronScrollEvent,
  IronTransitionEvent,
  IronWheelEvent,
} from "./events.ts";
export {
  Instance as IronElement,
  type ScrollBehavior,
  type ScrollIntoViewOptions,
  type ScrollLogicalPosition,
  type ScrollToOptions,
} from "./host.ts";

type PointerHandler = (event: IronPointerEvent) => void;

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
  onWheel?: (event: IronWheelEvent) => void;
  onWheelCapture?: (event: IronWheelEvent) => void;
  /** Keys go to the focused element (see `tabIndex`) and bubble. */
  onKeyDown?: (event: IronKeyboardEvent) => void;
  onKeyDownCapture?: (event: IronKeyboardEvent) => void;
  onKeyUp?: (event: IronKeyboardEvent) => void;
  onKeyUpCapture?: (event: IronKeyboardEvent) => void;
  /** Focus arrived on this element or a descendant (bubbles, like React DOM). */
  onFocus?: (event: IronFocusEvent) => void;
  onFocusCapture?: (event: IronFocusEvent) => void;
  /** Focus left this element or a descendant. */
  onBlur?: (event: IronFocusEvent) => void;
  onBlurCapture?: (event: IronFocusEvent) => void;
  /** The element scrolled (by the wheel, keys, its scrollbar or `scrollTo`). Doesn't bubble. */
  onScroll?: (event: IronScrollEvent) => void;
  /** A transition was created, passed its delay, finished, or was replaced / removed. Bubble. */
  onTransitionRun?: (event: IronTransitionEvent) => void;
  onTransitionStart?: (event: IronTransitionEvent) => void;
  onTransitionEnd?: (event: IronTransitionEvent) => void;
  onTransitionCancel?: (event: IronTransitionEvent) => void;
  /** A `@keyframes` animation started, began another iteration, finished, or was removed. Bubble. */
  onAnimationStart?: (event: IronAnimationEvent) => void;
  onAnimationIteration?: (event: IronAnimationEvent) => void;
  onAnimationEnd?: (event: IronAnimationEvent) => void;
  onAnimationCancel?: (event: IronAnimationEvent) => void;
  /** This element took or lost pointer capture (`ref.setPointerCapture`). */
  onGotPointerCapture?: PointerHandler;
  onLostPointerCapture?: PointerHandler;
  /**
   * Drag and drop. On a `draggable` element: `onDragStart` (prevent to cancel; put data
   * in `dataTransfer`), `onDrag` while it moves, `onDragEnd` after (`dropped`). On the
   * element under the pointer: `onDragEnter`, `onDragOver` (prevent to accept a drop),
   * `onDragLeave`, `onDrop`. Files from the system arrive as `dataTransfer.files`.
   */
  onDragStart?: (event: IronDragEvent) => void;
  onDragStartCapture?: (event: IronDragEvent) => void;
  onDrag?: (event: IronDragEvent) => void;
  onDragCapture?: (event: IronDragEvent) => void;
  onDragEnter?: (event: IronDragEvent) => void;
  onDragEnterCapture?: (event: IronDragEvent) => void;
  onDragOver?: (event: IronDragEvent) => void;
  onDragOverCapture?: (event: IronDragEvent) => void;
  onDragLeave?: (event: IronDragEvent) => void;
  onDragLeaveCapture?: (event: IronDragEvent) => void;
  onDrop?: (event: IronDragEvent) => void;
  onDropCapture?: (event: IronDragEvent) => void;
  onDragEnd?: (event: IronDragEvent) => void;
  onDragEndCapture?: (event: IronDragEvent) => void;
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
  style?: IronStyle;
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
  style?: IronStyle;
  children?: ReactNode;
  ref?: Ref<Instance>;
}

// React's own JSX types already define <text> (SVG) and <span> (HTML), so Iron's
// text elements are exposed as components. CSS type selectors still use `text`
// and `span`.

/** A block of text. Strings and `Span`s inside it flow and wrap together. */
export function Text({ children, ...props }: TextProps): ReactElement {
  return createElement("text" as never, props, children);
}

/** Props shared by `Input` and `TextArea`. */
export interface FieldProps extends Omit<ElementAttrs, "checked">, EventProps {
  style?: IronStyle;
  /** Controlled text: the field shows this and `onChange` reports edits. */
  value?: string;
  /** Initial text of an uncontrolled field. */
  defaultValue?: string;
  /** Shown dimmed while the field is empty. */
  placeholder?: string;
  readOnly?: boolean;
  /** Every edit (typing, deleting, pasting); `e.target.value` is the new text. */
  onChange?: (event: IronInputEvent) => void;
  onInput?: (event: IronInputEvent) => void;
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
  style?: IronStyle;
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
  style?: IronStyle;
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
  onCancel?: (event: IronPointerEvent) => void;
  onClose?: (event: IronPointerEvent) => void;
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

/** The UI target ids of windows: `WINDOW_TARGET | handle` (`iron_core::WINDOW_TARGET`); the main window is the screen. */
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
    "iron",
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
