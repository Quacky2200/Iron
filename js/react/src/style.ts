// Style properties understood by the engine (crates/iron_ui/src/style.rs).
// Numbers are logical pixels; strings are CSS values.

type Length = number | string;

export interface IronStyle {
  // Layout
  display?: "flex" | "grid" | "block" | "none";
  position?: "relative" | "absolute";
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
  justifyContent?: string;

  // Grid
  gridTemplateColumns?: string;
  gridTemplateRows?: string;
  gridAutoFlow?: string;
  gridColumn?: number | string;
  gridRow?: number | string;

  // Paint
  background?: string;
  backgroundColor?: string;
  backgroundImage?: string;
  backgroundSize?: string;
  backgroundPosition?: string;
  backgroundRepeat?: "repeat" | "no-repeat" | "repeat-x" | "repeat-y";
  border?: string;
  borderColor?: string;
  borderRadius?: Length;
  borderTopLeftRadius?: Length;
  borderTopRightRadius?: Length;
  borderBottomRightRadius?: Length;
  borderBottomLeftRadius?: Length;
  boxShadow?: string;
  filter?: string;
  backdropFilter?: string;
  opacity?: number;

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
  textDecoration?: string;
  whiteSpace?: "normal" | "nowrap" | "pre" | "pre-wrap";
  textShadow?: string;
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
