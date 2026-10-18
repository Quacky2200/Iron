//! `Map` and `Set`: insertion-ordered hash tables (JavaScript's SameValueZero equality: `NaN` equals
//! `NaN`, `-0` equals `0`, strings by content, objects by identity).
//!
//! One GC object, `MapObj`, with two raw blocks: the entries in insertion order (deleted ones stay as
//! tombstones until the table is rebuilt) and an open-addressing index of entry numbers. A `Set` is
//! a map whose values are unused.
//!
//! **Iteration is live and survives changes**, as in JavaScript: every entry gets an increasing
//! sequence number when it is added, and an iterator is just "the sequence number of the last entry
//! I gave out" (plus a position hint). To step it finds the first entry with a larger number, so
//! deleting entries, adding entries and rebuilding the table (which keeps the order) never confuse
//! it.
//!
//! Keys travel as 64-bit words: a number's bits, a pointer, a handle or a boolean. The key kind
//! tells equality and hashing (and the collector which words are pointers).

use super::string::Str;
use super::{HDR_MAP, Heap, maybe_collect, new_array, new_obj, raw_alloc, raw_free, with_heap};

/// Key kinds.
const K_NUM: u32 = 0;
const K_STR: u32 = 1;
/// A script object (a pointer, compared by identity).
const K_OBJ: u32 = 2;
// Kind 3 is a plain word (a handle, a boolean or a union tag): compared and hashed as the word.

#[repr(C)]
pub struct MapObj {
    pub header: u64,
    entries: *mut Entry,
    /// Entries used (live and deleted).
    len: i64,
    cap: i64,
    live: i64,
    index: *mut u32,
    index_cap: i64,
    next_seq: u64,
    key_kind: u32,
    val_ptr: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Entry {
    key: u64,
    value: u64,
    seq: u64,
    hash: u32,
    deleted: u32,
}

const ENTRY: usize = std::mem::size_of::<Entry>();

impl MapObj {
    fn entries(&self) -> &[Entry] {
        if self.entries.is_null() {
            return &[];
        }
        // SAFETY: `len` entries were written.
        unsafe { std::slice::from_raw_parts(self.entries, self.len as usize) }
    }

    fn entries_mut(&mut self) -> &mut [Entry] {
        if self.entries.is_null() {
            return &mut [];
        }
        // SAFETY: `len` entries were written.
        unsafe { std::slice::from_raw_parts_mut(self.entries, self.len as usize) }
    }

    fn index_slice(&self) -> &[u32] {
        if self.index.is_null() {
            return &[];
        }
        // SAFETY: `index_cap` slots.
        unsafe { std::slice::from_raw_parts(self.index, self.index_cap as usize) }
    }

    fn key_is_ptr(&self) -> bool {
        self.key_kind == K_STR || self.key_kind == K_OBJ
    }

    /// `(entries bytes, index bytes)` the collector accounts for.
    pub(crate) fn blocks(&self) -> (usize, usize) {
        (self.cap as usize * ENTRY, self.index_cap as usize * 4)
    }

    /// Frees both raw blocks (the object is dead).
    pub(crate) fn free_blocks(&mut self, h: &mut Heap) {
        let (e, i) = self.blocks();
        if !self.entries.is_null() {
            raw_free(h, self.entries as *mut u8, e);
        }
        if !self.index.is_null() {
            raw_free(h, self.index as *mut u8, i);
        }
    }

    /// The pointers the collector must keep alive.
    pub(crate) fn for_each_pointer(&self, mut f: impl FnMut(*mut u8)) {
        for e in self.entries() {
            if e.deleted == 0 {
                if self.key_is_ptr() {
                    f(e.key as *mut u8);
                }
                if self.val_ptr != 0 {
                    f(e.value as *mut u8);
                }
            }
        }
    }
}

/// `-0` is `0` and every `NaN` is one `NaN`.
fn normalize(kind: u32, key: u64) -> u64 {
    if kind != K_NUM {
        return key;
    }
    let f = f64::from_bits(key);
    if f == 0.0 {
        0
    } else if f.is_nan() {
        f64::NAN.to_bits()
    } else {
        key
    }
}

fn mix(mut x: u64) -> u32 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x as u32
}

fn hash_of(kind: u32, key: u64) -> u32 {
    if kind == K_STR {
        // SAFETY: a string key is a live string.
        return unsafe { &*(key as *const Str) }.content_hash();
    }
    mix(key)
}

fn same(kind: u32, a: u64, b: u64) -> bool {
    if a == b {
        return true;
    }
    if kind == K_STR {
        // SAFETY: string keys are live strings.
        let (x, y) = unsafe { (&*(a as *const Str), &*(b as *const Str)) };
        return x.as_bytes() == y.as_bytes();
    }
    false
}

fn find(m: &MapObj, key: u64) -> Option<usize> {
    let idx = m.index_slice();
    if idx.is_empty() {
        return None;
    }
    let key = normalize(m.key_kind, key);
    let h = hash_of(m.key_kind, key);
    let mask = idx.len() - 1;
    let entries = m.entries();
    let mut i = h as usize & mask;
    loop {
        let slot = idx[i];
        if slot == 0 {
            return None;
        }
        let e = &entries[slot as usize - 1];
        if e.deleted == 0 && e.hash == h && same(m.key_kind, e.key, key) {
            return Some(slot as usize - 1);
        }
        i = (i + 1) & mask;
    }
}

/// Rebuilds the table with room for `want` entries: deleted entries are dropped, order kept.
fn rebuild(h: &mut Heap, m: &mut MapObj, want: usize) {
    let new_cap = want.max(8).next_power_of_two();
    let new_index_cap = (new_cap * 2).max(16);
    let ne = raw_alloc(h, new_cap * ENTRY, false) as *mut Entry;
    let ni = raw_alloc(h, new_index_cap * 4, true) as *mut u32;
    let mut n = 0;
    for e in m.entries() {
        if e.deleted == 0 {
            // SAFETY: `n < new_cap` because the live count fits.
            unsafe { *ne.add(n) = *e };
            n += 1;
        }
    }
    let mask = new_index_cap - 1;
    for k in 0..n {
        // SAFETY: written above.
        let e = unsafe { &*ne.add(k) };
        let mut i = e.hash as usize & mask;
        // SAFETY: the index has free slots (half empty at most).
        unsafe {
            while *ni.add(i) != 0 {
                i = (i + 1) & mask;
            }
            *ni.add(i) = k as u32 + 1;
        }
    }
    m.free_blocks(h);
    m.entries = ne;
    m.index = ni;
    m.cap = new_cap as i64;
    m.index_cap = new_index_cap as i64;
    m.len = n as i64;
}

fn insert(h: &mut Heap, m: &mut MapObj, key: u64, value: u64) {
    let key = normalize(m.key_kind, key);
    if let Some(i) = find(m, key) {
        m.entries_mut()[i].value = value;
        return;
    }
    if m.len == m.cap {
        // Mostly tombstones: rebuild at the same size; otherwise double.
        let want = if m.live * 2 <= m.cap {
            m.cap as usize
        } else {
            m.cap as usize * 2
        };
        rebuild(h, m, want);
    }
    let hash = hash_of(m.key_kind, key);
    let seq = m.next_seq;
    m.next_seq += 1;
    let at = m.len as usize;
    // SAFETY: `len < cap` after the check above.
    unsafe {
        *m.entries.add(at) = Entry {
            key,
            value,
            seq,
            hash,
            deleted: 0,
        };
    }
    m.len += 1;
    m.live += 1;
    let mask = m.index_cap as usize - 1;
    let mut i = hash as usize & mask;
    // SAFETY: the index always has empty slots (twice the entry capacity).
    unsafe {
        while *m.index.add(i) != 0 {
            i = (i + 1) & mask;
        }
        *m.index.add(i) = at as u32 + 1;
    }
}

fn m<'a>(p: *mut MapObj) -> &'a mut MapObj {
    // SAFETY: callers pass live maps (the extern functions' contract).
    unsafe { &mut *p }
}

/// A new map. `key_kind` 0 numbers, 1 strings, 2 objects (identity), 3 plain words; `val_ptr` is 1
/// when the values are script pointers. May collect first (generated code).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_map_new_gc(key_kind: f64, val_ptr: f64) -> *mut MapObj {
    with_heap(|h| {
        maybe_collect(h);
        let p = new_obj(h, HDR_MAP, std::mem::size_of::<MapObj>()) as *mut MapObj;
        // SAFETY: just allocated (zeroed).
        unsafe {
            // Sequence numbers start at 1: an iterator's cursor 0 means "before the first".
            (*p).next_seq = 1;
            (*p).key_kind = key_kind as u32;
            (*p).val_ptr = val_ptr as u32;
        }
        p
    })
}

/// The entry number of `key`, or -1.
///
/// # Safety
/// `map` must be a live map and `key` a word of its key kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_find(map: *mut MapObj, key: u64) -> f64 {
    find(m(map), key).map_or(-1.0, |i| i as f64)
}

/// `map.set(key, value)` / `set.add(key)`. Never collects.
///
/// # Safety
/// `map` must be a live map; `key` and `value` words of its kinds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_set(map: *mut MapObj, key: u64, value: u64) {
    with_heap(|h| insert(h, m(map), key, value));
}

/// `map.delete(key)`: was it there?
///
/// # Safety
/// `map` must be a live map and `key` a word of its key kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_delete(map: *mut MapObj, key: u64) -> bool {
    let mo = m(map);
    match find(mo, key) {
        Some(i) => {
            let e = &mut mo.entries_mut()[i];
            e.deleted = 1;
            e.key = 0;
            e.value = 0;
            mo.live -= 1;
            true
        }
        None => false,
    }
}

/// `map.clear()`.
///
/// # Safety
/// `map` must be a live map.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_clear(map: *mut MapObj) {
    let mo = m(map);
    for e in mo.entries_mut() {
        e.deleted = 1;
        e.key = 0;
        e.value = 0;
    }
    mo.live = 0;
}

/// `map.size`.
///
/// # Safety
/// `map` must be a live map.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_size(map: *mut MapObj) -> f64 {
    m(map).live as f64
}

/// The key of entry `i` (from `find` or `next`).
///
/// # Safety
/// `map` must be a live map and `i` an entry number it gave out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_key_at(map: *mut MapObj, i: f64) -> u64 {
    m(map).entries()[i as usize].key
}

/// The value of entry `i`.
///
/// # Safety
/// `map` must be a live map and `i` an entry number it gave out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_value_at(map: *mut MapObj, i: f64) -> u64 {
    m(map).entries()[i as usize].value
}

/// The sequence number of entry `i`: the iterator's cursor after visiting it.
///
/// # Safety
/// `map` must be a live map and `i` an entry number it gave out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_seq_at(map: *mut MapObj, i: f64) -> f64 {
    m(map).entries()[i as usize].seq as f64
}

/// The next live entry after the one with sequence number `seq` (0 to start; `hint` is that
/// entry's last known position, -1 if unknown): its entry number, or -1 at the end.
///
/// # Safety
/// `map` must be a live map.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_next(map: *mut MapObj, seq: f64, hint: f64) -> f64 {
    let entries = m(map).entries();
    let seq = seq as u64;
    let hint = hint as i64;
    let mut i = if hint >= 0 && (hint as usize) < entries.len() && entries[hint as usize].seq == seq
    {
        hint as usize + 1
    } else {
        entries.partition_point(|e| e.seq <= seq)
    };
    while i < entries.len() {
        if entries[i].deleted == 0 {
            return i as f64;
        }
        i += 1;
    }
    -1.0
}

fn snapshot(map: *mut MapObj, values: bool) -> *mut super::Array {
    let mo = m(map);
    let ptrs = if values {
        mo.val_ptr != 0
    } else {
        mo.key_is_ptr()
    };
    let arr = with_heap(|h| {
        maybe_collect(h);
        new_array(h, mo.live, ptrs)
    });
    for e in mo.entries() {
        if e.deleted == 0 {
            // SAFETY: `arr` was just made; pushing never collects.
            unsafe { super::ts_rt_arr_push(arr, if values { e.value } else { e.key }) };
        }
    }
    arr
}

/// `[...map.keys()]` / `[...set]`: the keys now, as a new array.
///
/// # Safety
/// `map` must be a live map.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_keys_gc(map: *mut MapObj) -> *mut super::Array {
    snapshot(map, false)
}

/// `[...map.values()]`: the values now, as a new array.
///
/// # Safety
/// `map` must be a live map.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_map_values_gc(map: *mut MapObj) -> *mut super::Array {
    snapshot(map, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(x: f64) -> u64 {
        x.to_bits()
    }

    // M9-TS-206
    #[test]
    fn numbers_follow_same_value_zero_and_keep_insertion_order() {
        let mp = ts_rt_map_new_gc(0.0, 0.0);
        unsafe {
            ts_rt_map_set(mp, num(3.0), 30);
            ts_rt_map_set(mp, num(f64::NAN), 99);
            ts_rt_map_set(mp, num(-0.0), 7);
            ts_rt_map_set(mp, num(3.0), 31);
            assert_eq!(ts_rt_map_size(mp), 3.0);
            // NaN is one key; -0 is 0.
            let nan = ts_rt_map_find(mp, num(f64::NAN));
            assert!(nan >= 0.0);
            assert_eq!(ts_rt_map_value_at(mp, nan), 99);
            let zero = ts_rt_map_find(mp, num(0.0));
            assert_eq!(ts_rt_map_value_at(mp, zero), 7);
            assert_eq!(ts_rt_map_value_at(mp, ts_rt_map_find(mp, num(3.0))), 31);
            // Order: 3, NaN, 0.
            let mut seq = 0.0;
            let mut hint = -1.0;
            let mut got = Vec::new();
            loop {
                let i = ts_rt_map_next(mp, seq, hint);
                if i < 0.0 {
                    break;
                }
                got.push(ts_rt_map_value_at(mp, i));
                seq = ts_rt_map_seq_at(mp, i);
                hint = i;
            }
            assert_eq!(got, [31, 99, 7]);
        }
    }

    // M9-TS-207
    #[test]
    fn an_iterator_survives_deletes_adds_and_rebuilds() {
        let mp = ts_rt_map_new_gc(0.0, 0.0);
        unsafe {
            for k in 0..4 {
                ts_rt_map_set(mp, num(k as f64), k);
            }
            let mut seq = 0.0;
            let mut hint = -1.0;
            let mut seen = Vec::new();
            loop {
                let i = ts_rt_map_next(mp, seq, hint);
                if i < 0.0 {
                    break;
                }
                let v = ts_rt_map_value_at(mp, i);
                seen.push(v);
                seq = ts_rt_map_seq_at(mp, i);
                hint = i;
                if v == 0 {
                    // Delete the next one before it is reached and add many (forcing a rebuild).
                    ts_rt_map_delete(mp, num(1.0));
                    for k in 10..60 {
                        ts_rt_map_set(mp, num(k as f64), k);
                    }
                    for k in 10..50 {
                        ts_rt_map_delete(mp, num(k as f64));
                    }
                }
            }
            // 0, then 2 and 3 (1 was deleted), then the surviving additions 50..59.
            let mut want = vec![0, 2, 3];
            want.extend(50..60);
            assert_eq!(seen, want);
        }
    }

    // M9-TS-208
    #[test]
    fn a_map_is_one_collectable_object_that_keeps_its_pointers_alive() {
        let before = crate::gc_stats().objects;
        let mp = ts_rt_map_new_gc(1.0, 1.0); // string keys, pointer values
        let h = crate::ts_rt_root_add(mp as *mut u8);
        let k = unsafe { crate::ts_rt_str_new(b"key".as_ptr(), 3) };
        let v = unsafe { crate::ts_rt_str_new(b"value".as_ptr(), 5) };
        unsafe { ts_rt_map_set(mp, k as u64, v as u64) };
        // A different string object with the same content finds the entry.
        let k2 = unsafe { crate::ts_rt_str_new(b"key".as_ptr(), 3) };
        assert_eq!(crate::gc_stats().objects, before + 4);
        crate::ts_rt_gc_collect();
        // The map, its key and its value survive; the duplicate key does not.
        assert_eq!(crate::gc_stats().objects, before + 3);
        let _ = k2;
        crate::ts_rt_root_remove(h);
        crate::ts_rt_gc_collect();
        assert_eq!(crate::gc_stats().objects, before);
    }
}
