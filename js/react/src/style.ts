// Style properties understood by the engine (crates/iron_ui/src/style.rs).
// Numbers are logical pixels; strings are CSS values.

type Length = number | string;

export type BorderStyle =
  | "none"
  | "hidden"
  | "solid"
  | "dashed"
  | "dotted"
  | "double"
  | "groove"
  | "ridge"
  | "inset"
  | "outset";

export type BlendMode =
  | "normal"
  | "multiply"
  | "screen"
  | "overlay"
  | "darken"
  | "lighten"
  | "color-dodge"
  | "color-burn"
  | "hard-light"
  | "soft-light"
  | "difference"
  | "exclusion"
  | "hue"
  | "saturation"
  | "color"
  | "luminosity";

export interface IronStyle {
  // Layout
  /** `contents` drops the box: children join the parent's layout. */
  display?: "flex" | "grid" | "block" | "none" | "contents";
  /** `fixed` places the element against the window and paints it above the tree. */
  position?: "relative" | "absolute" | "fixed";
  /** Iron defaults to `border-box` (sizes include padding and border), unlike CSS. */
  boxSizing?: "border-box" | "content-box";
  width?: Length;
  height?: Length;
  minWidth?: Length;
  minHeight?: Length;
  maxWidth?: Length;
  maxHeight?: Length;
  aspectRatio?: number;
  top?: Length;
  right?: Length;
  bottom?: Length;
  left?: Length;
  inset?: Length;
  margin?: Length;
  marginTop?: Length;
  marginRight?: Length;
  marginBottom?: Length;
  marginLeft?: Length;
  padding?: Length;
  paddingTop?: Length;
  paddingRight?: Length;
  paddingBottom?: Length;
  paddingLeft?: Length;
  // Logical properties (horizontal, left-to-right): block = top/bottom, inline = left/right.
  marginBlock?: Length | string;
  marginInline?: Length | string;
  marginBlockStart?: Length;
  marginBlockEnd?: Length;
  marginInlineStart?: Length;
  marginInlineEnd?: Length;
  paddingBlock?: Length | string;
  paddingInline?: Length | string;
  paddingBlockStart?: Length;
  paddingBlockEnd?: Length;
  paddingInlineStart?: Length;
  paddingInlineEnd?: Length;
  insetBlock?: Length | string;
  insetInline?: Length | string;
  insetBlockStart?: Length;
  insetBlockEnd?: Length;
  insetInlineStart?: Length;
  insetInlineEnd?: Length;
  blockSize?: Length;
  inlineSize?: Length;
  minBlockSize?: Length;
  minInlineSize?: Length;
  maxBlockSize?: Length;
  maxInlineSize?: Length;
  borderWidth?: Length;
  borderTopWidth?: Length;
  borderRightWidth?: Length;
  borderBottomWidth?: Length;
  borderLeftWidth?: Length;
  gap?: Length;
  rowGap?: Length;
  columnGap?: Length;

  // Interaction
  pointerEvents?: "auto" | "none";
  /** CSS cursor keyword (`pointer`, `text`, `grab`, `not-allowed`, ...); inherited. */
  cursor?: Cursor;

  // Flexbox
  flexDirection?: "row" | "column" | "row-reverse" | "column-reverse";
  flexWrap?: "nowrap" | "wrap" | "wrap-reverse";
  flexGrow?: number;
  flexShrink?: number;
  flexBasis?: Length;
  flex?: number | "none" | "auto";

  // Alignment
  alignItems?: string;
  alignSelf?: string;
  alignContent?: string;
  justifyItems?: string;
  justifySelf?: string;
  /** `<align> [<justify>]`; one value sets both. */
  placeItems?: string;
  placeSelf?: string;
  placeContent?: string;
  justifyContent?: string;

  // Grid
  gridTemplateColumns?: string;
  gridTemplateRows?: string;
  gridAutoFlow?: string;
  gridColumn?: number | string;
  gridRow?: number | string;
  gridRowStart?: number | string;
  gridRowEnd?: number | string;
  gridColumnStart?: number | string;
  gridColumnEnd?: number | string;
  /** An area name, or `row-start / column-start / row-end / column-end`. */
  gridArea?: string;
  /** Quoted rows of area names: `'"head head" "side main"'`. */
  gridTemplateAreas?: string;
  /** `rows / columns`, or area strings with row sizes: `'"a b" 40px "c c" 1fr / 1fr 2fr'`. */
  gridTemplate?: string;
  gridAutoRows?: string;
  gridAutoColumns?: string;

  // Paint
  background?: string;
  backgroundColor?: string;
  /** Comma-separated layers, top first; the other background properties take matching lists. */
  backgroundImage?: string;
  backgroundSize?: string;
  backgroundPosition?: string;
  backgroundPositionX?: string;
  backgroundPositionY?: string;
  /** `repeat`, `no-repeat`, `repeat-x`, `repeat-y`, `round`, `space`, or one per axis. */
  backgroundRepeat?: string;
  /** `border-box` (default), `padding-box`, `content-box` or `text` (gradient text). */
  backgroundClip?: string;
  /** How each background layer blends with the layers and colour beneath it. */
  backgroundBlendMode?: string;
  /** How the element blends with what's beneath it. */
  mixBlendMode?: BlendMode;
  /** Width, style and colour, e.g. `"2px dashed red"`. No style means no border, like CSS. */
  border?: string;
  borderTop?: string;
  borderRight?: string;
  borderBottom?: string;
  borderLeft?: string;
  /** 1-4 colours (top, right, bottom, left). */
  borderColor?: string;
  borderTopColor?: string;
  borderRightColor?: string;
  borderBottomColor?: string;
  borderLeftColor?: string;
  /** 1-4 styles. Iron's default is `solid`, so a width alone draws a border. */
  borderStyle?: string;
  borderTopStyle?: BorderStyle;
  borderRightStyle?: BorderStyle;
  borderBottomStyle?: BorderStyle;
  borderLeftStyle?: BorderStyle;
  /** A 9-slice image or gradient over the border, e.g. `"url(frame.png) 12 fill / 8px round"`. */
  borderImage?: string;
  borderImageSource?: string;
  /** 1-4 numbers (image px) or percentages, plus `fill` to draw the middle. */
  borderImageSlice?: string;
  /** 1-4 multiples of the border width, lengths, percentages or `auto`. */
  borderImageWidth?: string;
  borderImageOutset?: string;
  /** `stretch`, `repeat`, `round` or `space`; one or two (horizontal, vertical). */
  borderImageRepeat?: string;
  /** Drawn outside the border box, over the content; takes no space. E.g. `"2px solid white"`. */
  outline?: string;
  outlineWidth?: Length;
  /** Default `none`; `auto` draws solid. */
  outlineStyle?: Exclude<BorderStyle, "hidden"> | "auto";
  outlineColor?: string;
  /** Gap between the border box and the outline; negative draws it inside. */
  outlineOffset?: Length;
  /** 1-4 radii, optionally `/` and vertical radii for elliptical corners (`"40px / 12px"`). */
  borderRadius?: Length;
  borderTopLeftRadius?: Length;
  borderTopRightRadius?: Length;
  borderBottomRightRadius?: Length;
  borderBottomLeftRadius?: Length;
  boxShadow?: string;
  filter?: string;
  backdropFilter?: string;
  opacity?: number;

  // Images
  /** How an `Image` fills its content box. Default `fill`. */
  objectFit?: "fill" | "contain" | "cover" | "none" | "scale-down";
  /** Where the image sits in its content box, like `backgroundPosition`. Default `50% 50%`. */
  objectPosition?: string;

  // Ordering
  /** Flex and grid item order (layout and painting; Tab order stays in document order). */
  order?: number;
  /** Paint and hit-test order among siblings. */
  zIndex?: number | "auto";
  /** Accepted; every z-indexed element already stacks on its own. */
  isolation?: "auto" | "isolate";

  // Clipping
  /** `visible`, `hidden`, `clip`, `scroll` or `auto`; one value or `x y`. Clips children to the padding box. */
  overflow?: string;
  overflowX?: Overflow;
  overflowY?: Overflow;
  /** Clips the element and its children: `inset(...)`, `circle(...)`, `ellipse(...)`, and/or a box keyword. */
  /** `inset()`, `circle()`, `ellipse()`, `polygon()` or `path()`, and/or a reference box. */
  clipPath?: string;
  /** Legacy `rect(top, right, bottom, left)`; absolutely positioned elements only. Prefer `clipPath`. */
  clip?: string;
  /** `auto`, `thin` or `none` (hidden but still scrollable). */
  scrollbarWidth?: "auto" | "thin" | "none";
  /** `auto` or `<thumb colour> <track colour>`; inherited. */
  scrollbarColor?: string;

  // Text (inherited, as in CSS). `currentColor` in colours means `color`.
  color?: string;
  fontFamily?: string;
  fontSize?: number | string;
  fontWeight?: number | "normal" | "bold";
  fontStyle?: "normal" | "italic" | "oblique";
  lineHeight?: number | string;
  letterSpacing?: number | string;
  wordSpacing?: number | string;
  textAlign?: "start" | "end" | "left" | "right" | "center" | "justify";
  direction?: "ltr" | "rtl";
  textDecoration?: string;
  whiteSpace?: "normal" | "nowrap" | "pre" | "pre-wrap";
  textShadow?: string;
  textTransform?: "none" | "uppercase" | "lowercase" | "capitalize";
  /** Not inherited. `ellipsis` cuts overflowing lines when `overflow` clips (use with `whiteSpace: "nowrap"`). */
  textOverflow?: "clip" | "ellipsis";
}

type StyleValue = string | number | null;

/** Flattens a style object into `[prop, value, ...]` pairs, skipping `undefined`. */
export function styleEntries(style: IronStyle | undefined): StyleValue[] {
  const out: StyleValue[] = [];
  if (!style) return out;
  for (const [prop, value] of Object.entries(style)) {
    if (value !== undefined) out.push(prop, value as StyleValue);
  }
  return out;
}

/**
 * Props whose values changed between two style objects, as `[prop, value, ...]`.
 * Removed props are sent as `null`, which resets them to their defaults.
 */
export function styleDiff(prev: IronStyle | undefined, next: IronStyle | undefined): StyleValue[] {
  const out: StyleValue[] = [];
  const a = (prev ?? {}) as Record<string, unknown>;
  const b = (next ?? {}) as Record<string, unknown>;
  for (const [prop, value] of Object.entries(b)) {
    if (value !== undefined && value !== a[prop]) out.push(prop, value as StyleValue);
  }
  for (const [prop, value] of Object.entries(a)) {
    if (value !== undefined && b[prop] === undefined) out.push(prop, null);
  }
  return out;
}

export type Overflow = "visible" | "hidden" | "clip" | "scroll" | "auto";

/** CSS `cursor` keywords (crates/iron_ui/src/style.rs `CURSORS`). */
export type Cursor =
  | "auto"
  | "default"
  | "none"
  | "context-menu"
  | "help"
  | "pointer"
  | "progress"
  | "wait"
  | "cell"
  | "crosshair"
  | "text"
  | "vertical-text"
  | "alias"
  | "copy"
  | "move"
  | "no-drop"
  | "not-allowed"
  | "grab"
  | "grabbing"
  | "all-scroll"
  | "col-resize"
  | "row-resize"
  | "n-resize"
  | "e-resize"
  | "s-resize"
  | "w-resize"
  | "ne-resize"
  | "nw-resize"
  | "se-resize"
  | "sw-resize"
  | "ew-resize"
  | "ns-resize"
  | "nesw-resize"
  | "nwse-resize"
  | "zoom-in"
  | "zoom-out";
