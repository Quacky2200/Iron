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

/// Closures (`bench/closures.ts`), prefix `clo_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod clo {
    include!(concat!(env!("OUT_DIR"), "/closures_bindings.rs"));
}

/// Module-level `let` variables (`bench/globals.ts`), prefix `glob_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod glob {
    include!(concat!(env!("OUT_DIR"), "/globals_bindings.rs"));
}

/// `for(;;)`, `do...while` and labels (`bench/loops.ts`), prefix `loop_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod lp {
    include!(concat!(env!("OUT_DIR"), "/loops_bindings.rs"));
}

/// `switch` and enums (`bench/switches.ts`), prefix `sw_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod sw {
    include!(concat!(env!("OUT_DIR"), "/switches_bindings.rs"));
}

/// Destructuring (`bench/destructure.ts`), prefix `des_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod des {
    include!(concat!(env!("OUT_DIR"), "/destructure_bindings.rs"));
}

/// Default, optional and rest parameters, spread (`bench/params.ts`), prefix `par_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod par {
    include!(concat!(env!("OUT_DIR"), "/params_bindings.rs"));
}

/// Programs that probe the collector's root analysis (`bench/gap.ts`), prefix `gap_`.
#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    clippy::all,
    unused_imports
)]
pub mod gap {
    include!(concat!(env!("OUT_DIR"), "/gap_bindings.rs"));
}

use iron_ts_rt::{
    Array, Closure, ts_rt_arr_new, ts_rt_arr_push, ts_rt_closure_new, ts_rt_gc_stress,
    ts_rt_obj_new, ts_rt_root_add,
};
use sim::*;

/// Allocates a zeroed script object of type `T` whose bindings header is `hdr` (`HDR_<Name>`).
/// Host allocations never collect.
pub fn alloc<T>(hdr: u64) -> *mut T {
    ts_rt_obj_new(hdr, std::mem::size_of::<T>()) as *mut T
}

/// Keeps a script object (and what it reaches) alive across calls into generated code.
pub fn root<T>(p: *mut T) -> *mut T {
    ts_rt_root_add(p as *mut u8);
    p
}

/// `IRON_TS_GC=stress` collects at every allocation of generated code and poisons freed memory
/// (a missing root then crashes loudly); `off` never collects on its own.
pub fn gc_mode_from_env() {
    match std::env::var("IRON_TS_GC").as_deref() {
        Ok("stress") => ts_rt_gc_stress(true),
        Ok("off") => iron_ts_rt::ts_rt_gc_enable(false),
        _ => {}
    }
}

/// Pushes numbers onto a `number[]`.
pub fn push_f64s(arr: *mut Array, vals: &[f64]) {
    for v in vals {
        // SAFETY: `arr` is a runtime array.
        unsafe { ts_rt_arr_push(arr, v.to_bits()) };
    }
}

pub fn push_ptr<T>(arr: *mut Array, p: *mut T) {
    // SAFETY: `arr` is a runtime array.
    unsafe { ts_rt_arr_push(arr, p as u64) };
}

/// `buildWalls(w, h, rows)` from run-length rows.
pub fn build_walls(w: usize, h: usize, rows: &[Vec<f64>]) -> *mut Walls {
    let outer = ts_rt_arr_new(rows.len() as i64, 1);
    for r in rows {
        let inner = ts_rt_arr_new(r.len() as i64, 0);
        for v in r {
            unsafe { ts_rt_arr_push(inner, v.to_bits()) };
        }
        push_ptr(outer, inner);
    }
    root(unsafe { ts_buildWalls(w as f64, h as f64, root(outer)) })
}

/// A `(map: number) => Walls` closure that always returns `walls`.
pub fn walls_closure(walls: *mut Walls) -> *mut Closure {
    extern "C" fn same(env: *mut u8, _map: f64) -> *mut Walls {
        env as *mut Walls
    }
    root(ts_rt_closure_new(same as *const u8, walls as *mut u8))
}

pub fn new_input(move_x: f64, move_y: f64, aim_x: f64, aim_y: f64, fire: bool) -> *mut PlayerInput {
    let p = root(alloc::<PlayerInput>(HDR_PlayerInput));
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
    let a = root(ts_rt_arr_new(list.len() as i64, 1));
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
    let z = alloc::<Zombie>(HDR_Zombie);
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

const W: usize = 1022;
const H: usize = 750;

/// The benchmark scenario (the same as `bench/bench.ts`): one player auto-firing an MP5 that
/// never runs dry or dies, zombies respawning, and the root handles Rust needs.
pub struct Busy {
    pub state: *mut SimState,
    pub walls: *mut Closure,
    pub input: *mut Array,
}

pub fn busy_state() -> Busy {
    unsafe {
        ts_init();
        let open = build_walls(W, H, &vec![vec![W as f64]; H]);
        let walls = walls_closure(open);
        let state = root(ts_createState(7.0, 6.0));
        let p = ts_addPlayer(state);
        ts_startLevel(state, walls);
        let gun = alloc::<Gun>(HDR_Gun);
        (*gun).name = TAG_MP5 as u64;
        (*gun).rounds = 1e9;
        (*gun).clip = 1e9;
        (*gun).damage = 40.0;
        (*gun).auto = 1;
        (*p).gun = gun;
        (*p).health = 1e9;
        (*state).zombiesLeft = 1e9;
        // 50 bullets that stay where they are (the MP5's own last a fraction of a second).
        for i in 0..50 {
            let b = alloc::<Bullet>(HDR_Bullet);
            (*b).id = 5000.0 + i as f64;
            (*b).x = 100.0 + i as f64 * 10.0;
            (*b).y = 100.0;
            (*b).targetId = null_num();
            (*b).ownerId = 1.0;
            (*b).life = 1e9;
            push_ptr((*state).bullets, b);
        }
        let input = inputs(&[new_input(0.0, 0.0, 900.0, 375.0, true)]);
        Busy {
            state,
            walls,
            input,
        }
    }
}

/// The same fingerprint of a state as `checksum` in `bench/bench.ts`.
pub fn busy_checksum(s: *mut SimState) -> f64 {
    unsafe {
        let st = &*s;
        let p = &*(*st.players).get::<*mut Player>(0);
        let mut sum = st.level * 1000.0
            + p.score
            + p.kills * 7.0
            + arr_len(st.bullets) as f64 * 13.0
            + arr_len(st.zombies) as f64 * 17.0
            + st.seed;
        for i in 0..arr_len(st.zombies) {
            let z = &*(*st.zombies).get::<*mut Zombie>(i);
            sum += z.x + z.y;
        }
        sum
    }
}
