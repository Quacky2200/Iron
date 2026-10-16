/**
 * @iron/react — render React components into Iron's engine-owned UI tree.
 *
 * ```tsx
 * import { createRoot } from "@iron/react";
 * createRoot().render(<box style={{ width: 100, height: 100, backgroundColor: "teal" }} />);
 * ```
 */

import "@iron/engine";
import { createElement, type ReactElement, type ReactNode, type Ref } from "react";
import { ConcurrentRoot } from "react-reconciler/constants";

import "./events.ts";
import type {
  IronFocusEvent,
  IronInputEvent,
  IronKeyboardEvent,
  IronPointerEvent,
  IronScrollEvent,
  IronWheelEvent,
} from "./events.ts";
import { type Container, type Instance, reconciler } from "./host.ts";
import { ROOT_ID } from "./opcodes.ts";
import type { IronStyle } from "./style.ts";

export type { IronStyle } from "./style.ts";
export {
  IronEvent,
  IronFocusEvent,
  IronInputEvent,
  IronKeyboardEvent,
  IronPointerEvent,
  IronScrollEvent,
  IronWheelEvent,
} from "./events.ts";
export { Instance as IronElement } from "./host.ts";

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
}

/** Attributes that CSS selectors can match. */
export interface ElementAttrs {
  className?: string;
  id?: string;
  disabled?: boolean;
  checked?: boolean;
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

/** An inline run inside `Text` with its own style (colour, weight, ...). */
export function Span({ children, ...props }: TextProps): ReactElement {
  return createElement("span" as never, props, children);
}

export interface Root {
  /** Renders `element`, committing synchronously so the UI updates this frame. */
  render(element: ReactNode): void;
  unmount(): void;
}

let rootCreated = false;

const reportError = (kind: string) => (error: unknown) => {
  const e = error instanceof Error ? error : new Error(String(error));
  console.error(`[react] ${kind}: ${e.message}${e.stack ? `\n${e.stack}` : ""}`);
};

/**
 * Creates the UI root. The engine has a single root container (node 0), so
 * there is one root per game; render more UI by composing components.
 */
export function createRoot(): Root {
  if (rootCreated) throw new Error("createRoot() can only be called once per game");
  rootCreated = true;

  const container: Container = { id: ROOT_ID, children: new Set() };
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

  return {
    render(element) {
      reconciler.updateContainerSync(element, root, null, null);
      reconciler.flushSyncWork();
    },
    unmount() {
      reconciler.updateContainerSync(null, root, null, null);
      reconciler.flushSyncWork();
    },
  };
}

/** Lets tests create fresh roots. Not part of the game API. */
export function _resetRootForTests() {
  rootCreated = false;
}
