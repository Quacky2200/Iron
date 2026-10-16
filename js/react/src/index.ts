/**
 * @iron/react — render React components into Iron's engine-owned UI tree.
 *
 * ```tsx
 * import { createRoot } from "@iron/react";
 * createRoot().render(<box style={{ width: 100, height: 100, backgroundColor: "teal" }} />);
 * ```
 */

import "@iron/engine";
import { createElement, type ReactElement, type ReactNode } from "react";
import { ConcurrentRoot } from "react-reconciler/constants";

import { type Container, reconciler } from "./host.ts";
import { ROOT_ID } from "./opcodes.ts";
import type { IronStyle } from "./style.ts";

export type { IronStyle } from "./style.ts";

export interface BoxProps {
  style?: IronStyle;
  children?: ReactNode;
  key?: string | number | null;
}

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      box: BoxProps;
    }
  }
}

export interface TextProps {
  style?: IronStyle;
  children?: ReactNode;
}

// React's own JSX types already define <text> (SVG) and <span> (HTML), so Iron's
// text elements are exposed as components. CSS type selectors still use `text`
// and `span`.

/** A block of text. Strings and `Span`s inside it flow and wrap together. */
export function Text({ style, children }: TextProps): ReactElement {
  return createElement("text" as never, { style }, children);
}

/** An inline run inside `Text` with its own style (colour, weight, ...). */
export function Span({ style, children }: TextProps): ReactElement {
  return createElement("span" as never, { style }, children);
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
