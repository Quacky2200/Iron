//! Test harness for the native code `iron_ts` generates from `sim.ts`: the generated bindings
//! (layouts, tags, extern functions) plus helpers to build state from Rust.
#![allow(non_snake_case, clippy::not_unsafe_ptr_arg_deref)] // a test harness: every pointer comes from the generated code

#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod sim {
    include!(concat!(env!("OUT_DIR"), "/sim_bindings.rs"));
}

/// The sine-circle curve maths (`bench/sine.ts`), prefix `sine_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod sine {
    include!(concat!(env!("OUT_DIR"), "/sine_bindings.rs"));
}

use iron_ts_rt::{Array, Closure, ts_rt_alloc, ts_rt_arr_new, ts_rt_arr_push, ts_rt_closure_new};
use sim::*;

/// Allocates a zeroed object of type `T` from the runtime arena (headers are not needed by the
/// generated code, only by a future collector).
pub fn alloc<T>() -> *mut T {
    ts_rt_alloc(std::mem::size_of::<T>()) as *mut T
}

pub fn push_ptr<T>(arr: *mut Array, p: *mut T) {
    // SAFETY: `arr` is a runtime array.
    unsafe { ts_rt_arr_push(arr, p as u64) };
}

/// `buildWalls(w, h, rows)` from run-length rows.
pub fn build_walls(w: usize, h: usize, rows: &[Vec<f64>]) -> *mut Walls {
    let outer = ts_rt_arr_new(rows.len() as i64);
    for r in rows {
        let inner = ts_rt_arr_new(r.len() as i64);
        for v in r {
            unsafe { ts_rt_arr_push(inner, v.to_bits()) };
        }
        push_ptr(outer, inner);
    }
    unsafe { ts_buildWalls(w as f64, h as f64, outer) }
}

/// A `(map: number) => Walls` closure that always returns `walls`.
pub fn walls_closure(walls: *mut Walls) -> *mut Closure {
    extern "C" fn same(env: *mut u8, _map: f64) -> *mut Walls {
        env as *mut Walls
    }
    ts_rt_closure_new(same as *const u8, walls as *mut u8)
}

pub fn new_input(move_x: f64, move_y: f64, aim_x: f64, aim_y: f64, fire: bool) -> *mut PlayerInput {
    let p = alloc::<PlayerInput>();
    unsafe {
        (*p).moveX = move_x;
        (*p).moveY = move_y;
        (*p).aimX = aim_x;
        (*p).aimY = aim_y;
        (*p).fire = fire as u64;
    }
    p
}

pub fn inputs(list: &[*mut PlayerInput]) -> *mut Array {
    let a = ts_rt_arr_new(list.len() as i64);
    for p in list {
        push_ptr(a, *p);
    }
    a
}

pub fn new_zombie(
    id: f64,
    x: f64,
    y: f64,
    hp: f64,
    attack_timer: Option<f64>,
    target_id: f64,
) -> *mut Zombie {
    let z = alloc::<Zombie>();
    unsafe {
        (*z).id = id;
        (*z).x = x;
        (*z).y = y;
        (*z).hp = hp;
        (*z).maxHp = hp;
        (*z).attackTimer = attack_timer.unwrap_or_else(null_num);
        (*z).targetId = target_id;
    }
    z
}

pub fn arr_len(a: *mut Array) -> usize {
    unsafe { (*a).len() }
}

/// Steps `state` for whole 60 Hz frames, like the test file's `run`.
pub fn run(state: *mut SimState, seconds: f64, input: *mut Array, walls: *mut Closure) {
    let frames = (seconds * 60.0).round() as usize;
    for _ in 0..frames {
        unsafe { ts_step(state, input, 1.0 / 60.0, walls) };
    }
}
