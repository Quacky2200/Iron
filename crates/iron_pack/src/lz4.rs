//! The LZ4 *block* format (no frame header), in safe Rust: a greedy single-hash compressor and a
//! bounds-checked decompressor. Output is standard LZ4 (any LZ4 block decoder reads it, and this
//! decoder reads blocks made by `lz4_flex` / liblz4), so a faster or stronger compressor can
//! replace this one later without changing packs.
//!
//! Format: sequences of `token (literal length << 4 | match length - 4)`, extra length bytes when
//! a nibble is 15 (each 255 continues), the literals, a 2-byte little-endian back offset, extra
//! match length bytes. The last sequence has literals only; the last 5 bytes are always literals
//! and the last match starts at least 12 bytes before the end.

const MIN_MATCH: usize = 4;
const LAST_LITERALS: usize = 5;
const MF_LIMIT: usize = 12;
const HASH_LOG: u32 = 14;
const MAX_OFFSET: usize = 65_535;

/// Compresses `input` (at most 4 GiB; packs give it one chunk).
pub fn compress(input: &[u8]) -> Vec<u8> {
    let n = input.len();
    let mut out = Vec::with_capacity(n + n / 255 + 16);
    // Position + 1 of the last 4-byte sequence with each hash (0: none).
    let mut table = vec![0u32; 1 << HASH_LOG];
    let mut anchor = 0;
    let mut i = 0;
    if n > MF_LIMIT {
        let limit = n - MF_LIMIT;
        let match_limit = n - LAST_LITERALS;
        // Skips ahead faster through data that does not compress (as liblz4's acceleration).
        let mut misses = 0usize;
        while i < limit {
            let seq = read_u32(input, i);
            let h = (seq.wrapping_mul(2_654_435_761) >> (32 - HASH_LOG)) as usize;
            let candidate = table[h] as usize;
            table[h] = (i + 1) as u32;
            if candidate > 0 {
                let c = candidate - 1;
                if i - c <= MAX_OFFSET && read_u32(input, c) == seq {
                    let mut len = MIN_MATCH;
                    while i + len < match_limit && input[c + len] == input[i + len] {
                        len += 1;
                    }
                    // Extend backwards over literals that also match.
                    let (mut s, mut m) = (i, c);
                    while s > anchor && m > 0 && input[s - 1] == input[m - 1] {
                        s -= 1;
                        m -= 1;
                        len += 1;
                    }
                    write_sequence(&mut out, &input[anchor..s], (s - m) as u16, len);
                    i = s + len;
                    anchor = i;
                    misses = 0;
                    continue;
                }
            }
            misses += 1;
            i += 1 + (misses >> 6);
        }
    }
    write_literals(&mut out, &input[anchor..]);
    out
}

fn read_u32(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

fn write_length(out: &mut Vec<u8>, mut n: usize) {
    while n >= 255 {
        out.push(255);
        n -= 255;
    }
    out.push(n as u8);
}

fn write_sequence(out: &mut Vec<u8>, literals: &[u8], offset: u16, match_len: usize) {
    let (ll, ml) = (literals.len(), match_len - MIN_MATCH);
    out.push(((ll.min(15) as u8) << 4) | ml.min(15) as u8);
    if ll >= 15 {
        write_length(out, ll - 15);
    }
    out.extend_from_slice(literals);
    out.extend_from_slice(&offset.to_le_bytes());
    if ml >= 15 {
        write_length(out, ml - 15);
    }
}

fn write_literals(out: &mut Vec<u8>, literals: &[u8]) {
    let ll = literals.len();
    out.push((ll.min(15) as u8) << 4);
    if ll >= 15 {
        write_length(out, ll - 15);
    }
    out.extend_from_slice(literals);
}

/// Decompresses a block that must expand to exactly `raw_size` bytes. Damaged input is an
/// error, never a panic or an out-of-bounds read.
pub fn decompress(input: &[u8], raw_size: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(raw_size);
    decompress_into(input, raw_size, &mut out)?;
    Ok(out)
}

/// As [`decompress`], appending to `out`.
pub fn decompress_into(input: &[u8], raw_size: usize, out: &mut Vec<u8>) -> Result<(), String> {
    let base = out.len();
    let end = base + raw_size;
    let mut i = 0;
    let bad = |what: &str| Err(format!("lz4: {what}"));
    loop {
        let Some(&token) = input.get(i) else {
            return bad("truncated");
        };
        i += 1;
        let mut ll = usize::from(token >> 4);
        if ll == 15 {
            ll += read_length(input, &mut i)?;
        }
        let Some(literals) = input.get(i..i.saturating_add(ll)) else {
            return bad("literals past the end");
        };
        if out.len() + ll > end {
            return bad("output too long");
        }
        out.extend_from_slice(literals);
        i += ll;
        if i == input.len() {
            break;
        }
        let Some(o) = input.get(i..i + 2) else {
            return bad("truncated offset");
        };
        let offset = usize::from(u16::from_le_bytes([o[0], o[1]]));
        i += 2;
        if offset == 0 || offset > out.len() - base {
            return bad("offset out of range");
        }
        let mut ml = usize::from(token & 15);
        if ml == 15 {
            ml += read_length(input, &mut i)?;
        }
        ml += MIN_MATCH;
        if out.len() + ml > end {
            return bad("output too long");
        }
        let start = out.len() - offset;
        if offset >= ml {
            out.extend_from_within(start..start + ml);
        } else {
            // Overlapping: the bytes from `start` repeat with period `offset`, so copying a
            // whole number of periods at a time (doubling) stays correct.
            let mut copied = 0;
            while copied < ml {
                let n = (ml - copied).min(offset + copied);
                out.extend_from_within(start..start + n);
                copied += n;
            }
        }
    }
    if out.len() != end {
        return bad("output too short");
    }
    Ok(())
}

fn read_length(input: &[u8], i: &mut usize) -> Result<usize, String> {
    let mut n = 0usize;
    loop {
        let Some(&b) = input.get(*i) else {
            return Err("lz4: truncated length".into());
        };
        *i += 1;
        n = n
            .checked_add(usize::from(b))
            .ok_or("lz4: length overflow")?;
        if b != 255 {
            return Ok(n);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(data: &[u8]) -> usize {
        let c = compress(data);
        assert_eq!(
            decompress(&c, data.len()).unwrap(),
            data,
            "len {}",
            data.len()
        );
        c.len()
    }

    // M9-API-1101
    #[test]
    fn lz4_blocks_round_trip_and_shrink_repetitive_data() {
        for n in [0, 1, 4, 12, 13, 17, 64, 1000] {
            let data: Vec<u8> = (0..n).map(|i| (i * 7 % 13) as u8).collect();
            round_trip(&data);
        }
        // Runs (overlapping copies), text, and noise.
        assert!(round_trip(&[7u8; 100_000]) < 600);
        let text = "the quick brown fox jumps over the lazy dog. ".repeat(2000);
        assert!(round_trip(text.as_bytes()) < text.len() / 20);
        let mut x = 0x1234_5678u32;
        let noise: Vec<u8> = (0..70_000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect();
        assert!(round_trip(&noise) <= noise.len() + noise.len() / 255 + 16);
        // Mixed: noise then a long-distance repeat beyond the 64 KiB window and inside it.
        let mut mixed = noise.clone();
        mixed.extend_from_slice(&noise[60_000..]);
        round_trip(&mixed);
    }

    // M9-API-1101
    #[test]
    fn lz4_reads_reference_blocks_and_rejects_damage() {
        // "abcabcabcabcabcabcabcabc" + "XYZXY" as liblz4 encodes it: 3 literals, a match at
        // offset 3 of 21 bytes, then 5 literals.
        let block = [
            0x3f, b'a', b'b', b'c', 3, 0, 2, 0x50, b'X', b'Y', b'Z', b'X', b'Y',
        ];
        assert_eq!(
            decompress(&block, 29).unwrap(),
            b"abcabcabcabcabcabcabcabcXYZXY"
        );
        assert!(decompress(&block, 28).is_err());
        assert!(decompress(&block, 30).is_err());
        assert!(decompress(&block[..5], 29).is_err());
        // An offset reaching before the start.
        assert!(decompress(&[0x10, b'a', 9, 0, 0x00], 10).is_err());
        // Garbage never panics.
        let mut x = 99u32;
        for len in 0..300 {
            let junk: Vec<u8> = (0..len)
                .map(|_| {
                    x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    (x >> 16) as u8
                })
                .collect();
            let _ = decompress(&junk, 1000);
        }
    }
}
