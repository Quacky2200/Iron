// SPDX-License-Identifier: Apache-2.0

//! A Gaussian blur and colour matrix over a region of a premultiplied RGBA8 buffer, for
//! `backdrop-filter` (the scene so far is read back once the backdrop is needed).

/// Blurs `region` (x, y, w, h in pixels) of `data` (`width` pixels per row) with standard
/// deviation `sigma`: three box blurs, which approximate a Gaussian closely. Edges repeat.
pub fn blur_region(
    data: &mut [u8],
    width: usize,
    region: (usize, usize, usize, usize),
    sigma: f32,
) {
    let (rx, ry, rw, rh) = region;
    if rw == 0 || rh == 0 || sigma <= 0.0 {
        return;
    }
    // Box sizes for three passes (the standard "boxes for Gauss" estimate).
    let n = 3.0_f32;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut lower = ideal.floor() as i32;
    if lower % 2 == 0 {
        lower -= 1;
    }
    let lower = lower.max(1);
    let upper = lower + 2;
    let m = ((12.0 * sigma * sigma - n * (lower * lower) as f32 - 4.0 * n * lower as f32 - 3.0 * n)
        / (-4.0 * lower as f32 - 4.0))
        .round() as i32;
    let sizes = [0, 1, 2].map(|i| if i < m { lower } else { upper });

    let mut buf: Vec<[u8; 4]> = Vec::with_capacity(rw * rh);
    for y in 0..rh {
        for x in 0..rw {
            let i = ((ry + y) * width + rx + x) * 4;
            buf.push([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        }
    }
    let mut tmp = buf.clone();
    for size in sizes {
        let r = ((size - 1) / 2) as usize;
        box_pass(&buf, &mut tmp, rw, rh, r, true);
        box_pass(&tmp, &mut buf, rw, rh, r, false);
    }
    for y in 0..rh {
        for x in 0..rw {
            let i = ((ry + y) * width + rx + x) * 4;
            data[i..i + 4].copy_from_slice(&buf[y * rw + x]);
        }
    }
}

/// One horizontal (`horizontal`) or vertical box blur of radius `r`, clamping at the edges.
fn box_pass(src: &[[u8; 4]], dst: &mut [[u8; 4]], w: usize, h: usize, r: usize, horizontal: bool) {
    let (lines, len) = if horizontal { (h, w) } else { (w, h) };
    let at = |line: usize, i: usize| {
        if horizontal {
            line * w + i
        } else {
            i * w + line
        }
    };
    let window = (2 * r + 1) as u32;
    for line in 0..lines {
        let clamp = |i: isize| i.clamp(0, len as isize - 1) as usize;
        let mut sum = [0u32; 4];
        for k in -(r as isize)..=(r as isize) {
            let p = src[at(line, clamp(k))];
            for c in 0..4 {
                sum[c] += u32::from(p[c]);
            }
        }
        for i in 0..len {
            let out = &mut dst[at(line, i)];
            for c in 0..4 {
                out[c] = (sum[c] / window) as u8;
            }
            let add = src[at(line, clamp(i as isize + r as isize + 1))];
            let sub = src[at(line, clamp(i as isize - r as isize))];
            for c in 0..4 {
                sum[c] = sum[c] + u32::from(add[c]) - u32::from(sub[c]);
            }
        }
    }
}

/// Applies a 4x5 colour matrix (CSS order: rows R, G, B, A; the fifth column is an offset) to
/// premultiplied pixels in `region`.
pub fn color_matrix_region(
    data: &mut [u8],
    width: usize,
    region: (usize, usize, usize, usize),
    m: &[f32; 20],
) {
    let (rx, ry, rw, rh) = region;
    for y in ry..ry + rh {
        for x in rx..rx + rw {
            let i = (y * width + x) * 4;
            let a = f32::from(data[i + 3]) / 255.0;
            // Straight colour in, matrix, premultiply again.
            let straight = if a > 0.0 {
                [0, 1, 2].map(|c| (f32::from(data[i + c]) / 255.0 / a).min(1.0))
            } else {
                [0.0; 3]
            };
            let v = [straight[0], straight[1], straight[2], a];
            let row = |r: usize| {
                (m[r * 5] * v[0]
                    + m[r * 5 + 1] * v[1]
                    + m[r * 5 + 2] * v[2]
                    + m[r * 5 + 3] * v[3]
                    + m[r * 5 + 4])
                    .clamp(0.0, 1.0)
            };
            let na = row(3);
            for c in 0..3 {
                data[i + c] = (row(c) * na * 255.0).round() as u8;
            }
            data[i + 3] = (na * 255.0).round() as u8;
        }
    }
}
