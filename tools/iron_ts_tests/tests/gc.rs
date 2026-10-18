//! The collector under the zombie sim: it must not change what the game computes, and it must
//! keep memory bounded. IDs are in TESTS.md (M9-TS-23..25).
use iron_ts_rt::{gc_stats, link_anchor, ts_rt_gc_collect, ts_rt_gc_stress};
use iron_ts_tests::sim::*;
use iron_ts_tests::*;

fn run_busy(steps: usize) -> f64 {
    assert_ne!(link_anchor(), 0);
    let b = busy_state();
    for _ in 0..steps {
        unsafe { ts_step(b.state, b.input, 1.0 / 60.0, b.walls) };
    }
    busy_checksum(b.state)
}

// M9-TS-23
#[test]
fn collecting_at_every_allocation_changes_nothing_the_game_computes() {
    let normal = run_busy(600);
    ts_rt_gc_stress(true);
    let before = gc_stats().collections;
    let stressed = run_busy(600);
    ts_rt_gc_stress(false);
    assert!(
        gc_stats().collections - before > 1_000,
        "stress mode collects on every allocation: {} collections",
        gc_stats().collections - before
    );
    assert_eq!(normal, stressed);
}

// M9-TS-24
#[test]
fn a_long_run_collects_on_its_own_and_memory_stays_bounded() {
    let b = {
        assert_ne!(link_anchor(), 0);
        busy_state()
    };
    let start = gc_stats();
    for _ in 0..8_000 {
        unsafe { ts_step(b.state, b.input, 1.0 / 60.0, b.walls) };
    }
    let end = gc_stats();
    assert!(end.collections - start.collections >= 10, "{end:?}");
    // What the run handed out in all is far more than what is alive.
    assert!(
        end.allocated_bytes - start.allocated_bytes > 8 << 20,
        "{end:?}"
    );
    ts_rt_gc_collect();
    let live = gc_stats();
    assert!(
        live.live_bytes < 512 << 10 && live.objects < 5_000,
        "{live:?}"
    );
}

// M9-TS-25
#[test]
fn what_the_host_roots_survives_and_the_rest_is_freed() {
    assert_ne!(link_anchor(), 0);
    let b = busy_state();
    for _ in 0..300 {
        unsafe { ts_step(b.state, b.input, 1.0 / 60.0, b.walls) };
    }
    ts_rt_gc_stress(true);
    ts_rt_gc_collect();
    // The state is rooted, so its players, zombies and bullets are still readable (not 0xDD poison).
    unsafe {
        let st = &*b.state;
        let p = &*(*st.players).get::<*mut Player>(0);
        assert!(p.score >= 0.0 && p.health > 1e8);
        assert!(arr_len(st.bullets) < 1_000);
    }
    ts_rt_gc_stress(false);
}
