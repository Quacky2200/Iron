//! Paint placement shared by the renderers (GPU and CPU): where a background tile goes in its
//! box, how it repeats, and a gradient's colour stops resolved to positions along its line.

use crate::Rect;
use crate::paint::{
    BackgroundPosition, BackgroundRepeat, BackgroundSize, BorderImageRepeat, ColorStop, Len,
    RepeatAxis,
};

/// Where one tile of an image of `natural` size goes in `area`, following CSS
/// `background-size` and `background-position`.
pub fn tile_rect(
    area: Rect,
    natural: (f32, f32),
    size: BackgroundSize,
    position: BackgroundPosition,
) -> Rect {
    let (iw, ih) = (natural.0.max(1.0), natural.1.max(1.0));
    let (w, h) = (area.width, area.height);
    let (tw, th) = match size {
        BackgroundSize::Cover => {
            let s = (w / iw).max(h / ih);
            (iw * s, ih * s)
        }
        BackgroundSize::Contain => {
            let s = (w / iw).min(h / ih);
            (iw * s, ih * s)
        }
        BackgroundSize::Explicit(sw, sh) => match (sw, sh) {
            (None, None) => (iw, ih),
            (Some(sw), None) => {
                let tw = sw.resolve(w);
                (tw, tw * ih / iw)
            }
            (None, Some(sh)) => {
                let th = sh.resolve(h);
                (th * iw / ih, th)
            }
            (Some(sw), Some(sh)) => (sw.resolve(w), sh.resolve(h)),
        },
    };
    // Percentages line up the same point of the image and the box.
    let offset = |len: Len, space: f32| match len {
        Len::Percent(f) => space * f,
        Len::Px(px) => px,
    };
    Rect::new(
        area.x + offset(position.x, w - tw),
        area.y + offset(position.y, h - th),
        tw.max(0.0),
        th.max(0.0),
    )
}

/// Applies `background-repeat` to one placed tile in `area`: `round` rescales the
/// tile so whole tiles fit, `space` keeps its size and spreads the leftover space
/// as gaps. Returns the tile, the gap per axis and whether each axis repeats.
pub fn repeat_tiles(
    tile: Rect,
    area: Rect,
    repeat: BackgroundRepeat,
) -> (Rect, [f32; 2], [bool; 2]) {
    let axis = |pos: f32, len: f32, start: f32, space: f32, mode: RepeatAxis| match mode {
        RepeatAxis::Repeat => (pos, len, 0.0, true),
        RepeatAxis::NoRepeat => (pos, len, 0.0, false),
        RepeatAxis::Round if len > 0.0 => {
            let n = (space / len).round().max(1.0);
            (start, space / n, 0.0, true)
        }
        RepeatAxis::Space if len > 0.0 && (space / len).floor() >= 2.0 => {
            let n = (space / len).floor();
            (start, len, (space - n * len) / (n - 1.0), true)
        }
        // Too small to round, or room for one tile only: placed once.
        RepeatAxis::Round | RepeatAxis::Space => (pos, len, 0.0, false),
    };
    let (x, w, gx, rx) = axis(tile.x, tile.width, area.x, area.width, repeat.x);
    let (y, h, gy, ry) = axis(tile.y, tile.height, area.y, area.height, repeat.y);
    (Rect::new(x, y, w, h), [gx, gy], [rx, ry])
}

/// CSS colour-stop fix-up: missing first/last positions become 0%/100%, positions
/// never go backwards, and runs of missing positions are spaced evenly.
/// Returns (straight-alpha sRGB colour, position as a fraction of the gradient line).
pub fn resolve_stops(stops: &[ColorStop], line_length: f32) -> Vec<([f32; 4], f32)> {
    let n = stops.len();
    let mut pos: Vec<Option<f32>> = stops
        .iter()
        .map(|s| {
            s.position.map(|p| match p {
                Len::Percent(f) => f,
                Len::Px(px) => px / line_length,
            })
        })
        .collect();
    if n == 0 {
        return Vec::new();
    }
    pos[0] = Some(pos[0].unwrap_or(0.0));
    pos[n - 1] = Some(pos[n - 1].unwrap_or(1.0));

    let mut max_so_far = f32::NEG_INFINITY;
    for p in pos.iter_mut().flatten() {
        *p = p.max(max_so_far);
        max_so_far = *p;
    }

    let mut i = 0;
    while i < n {
        if pos[i].is_some() {
            i += 1;
            continue;
        }
        let start = i - 1; // pos[0] is always set
        let end = (i..n).find(|&j| pos[j].is_some()).expect("last is set");
        let (a, b) = (pos[start].unwrap(), pos[end].unwrap());
        for (k, slot) in pos.iter_mut().enumerate().take(end).skip(start + 1) {
            *slot = Some(a + (b - a) * (k - start) as f32 / (end - start) as f32);
        }
        i = end;
    }

    stops
        .iter()
        .zip(pos)
        .map(|(s, p)| ([s.color.r, s.color.g, s.color.b, s.color.a], p.unwrap()))
        .collect()
}

/// Whether `url` names an SVG image.
pub fn is_svg(url: &str) -> bool {
    let url = url.trim();
    if url.starts_with("data:") {
        return url.starts_with("data:image/svg+xml");
    }
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.to_ascii_lowercase().ends_with(".svg")
}

/// Lays tiles of size `tile` along one axis of a border image piece starting at
/// `start` with length `len`: (tile start, tile size, gap, repeats), or `None`
/// when `space` can't fit a single tile (CSS then draws nothing).
pub fn border_image_axis(
    start: f32,
    len: f32,
    tile: f32,
    mode: BorderImageRepeat,
) -> Option<(f32, f32, f32, bool)> {
    if tile <= 0.0 {
        return Some((start, len, 0.0, false));
    }
    Some(match mode {
        BorderImageRepeat::Stretch => (start, len, 0.0, false),
        // Centred, so the cut-off tiles at both ends match.
        BorderImageRepeat::Repeat => (start + (len - tile) / 2.0, tile, 0.0, true),
        BorderImageRepeat::Round => {
            let n = (len / tile).round().max(1.0);
            (start, len / n, 0.0, true)
        }
        BorderImageRepeat::Space => {
            let n = (len / tile).floor();
            if n < 1.0 {
                return None;
            }
            // The leftover space goes around the tiles, ends included.
            let gap = (len - n * tile) / (n + 1.0);
            (start + gap, tile, gap, true)
        }
    })
}
