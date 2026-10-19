//! Stackful coroutines for generators (and, next, `async`/`await`). A coroutine runs a `() => void`
//! closure on its own native stack (`corosensei`). The garbage collector's roots live on the shadow
//! stack, a linked list of frames that sit in each function's native frame, so a coroutine only needs
//! its frame chain kept apart: its first frame (`bottom`) is linked to whoever resumed it on every
//! resume, and while it is suspended the collector walks `saved_top` down to `bottom` itself.
//! See `docs/research/iron-ts-generators-and-proxy.md`.

use super::*;
use corosensei::stack::DefaultStack;
use corosensei::{Coroutine, CoroutineResult, Yielder};
use std::cell::RefCell;

const STACK_BYTES: usize = 256 * 1024;

const NEW: i64 = 0;
const SUSPENDED: i64 = 1;
const RUNNING: i64 = 2;
const DONE: i64 = 3;

/// The Rust side of a coroutine: the native stack and where to switch back to.
struct Inner {
    co: Coroutine<(), (), (), DefaultStack>,
    yielder: *const Yielder<(), ()>,
}

#[repr(C)]
pub struct Coro {
    header: u64,
    closure: *mut Closure,
    state: i64,
    yielded: u64,
    yielded_is_ptr: i64,
    sent: u64,
    sent_is_ptr: i64,
    /// An error (or `return()` marker) to raise at the `yield` the body is suspended on.
    inject: *mut u8,
    saved_top: *mut Frame,
    bottom: *mut Frame,
    resumer_top: *mut Frame,
    inner: *mut Inner,
}

thread_local! {
    /// The coroutines being resumed, innermost last.
    static CURRENT: RefCell<Vec<*mut Coro>> = const { RefCell::new(Vec::new()) };
}

fn top() -> *mut Frame {
    with_heap(|h| h.top)
}

fn set_top(f: *mut Frame) {
    with_heap(|h| h.top = f);
}

/// The objects a coroutine keeps alive, for the collector: its closure, the values in transit, and
/// the frames of a suspended body.
///
/// # Safety
/// `p` must be a live coroutine object.
pub(crate) unsafe fn for_each_pointer(p: *const Coro, mut f: impl FnMut(*mut u8)) {
    // SAFETY: by contract.
    unsafe {
        let c = &*p;
        f(c.closure as *mut u8);
        f(c.inject);
        if c.yielded_is_ptr != 0 {
            f(c.yielded as *mut u8);
        }
        if c.sent_is_ptr != 0 {
            f(c.sent as *mut u8);
        }
        if c.state == SUSPENDED {
            let mut fr = c.saved_top;
            while !fr.is_null() {
                let slots = (fr as *mut u8).add(std::mem::size_of::<Frame>()) as *mut *mut u8;
                for i in 0..(*fr).n as usize {
                    f(*slots.add(i));
                }
                if fr == c.bottom {
                    break;
                }
                fr = (*fr).prev;
            }
        }
    }
}

/// Frees the native stack of a coroutine that is being swept.
///
/// # Safety
/// `p` must be a coroutine object that is no longer reachable.
pub(crate) unsafe fn free(p: *mut Coro) {
    // SAFETY: by contract.
    unsafe {
        if !(*p).inner.is_null() {
            let mut inner = Box::from_raw((*p).inner);
            // Frames of compiled code on the stack have no destructors to run.
            inner.co.force_reset();
            (*p).inner = std::ptr::null_mut();
        }
    }
}

/// `new coroutine running closure`: not started until the first resume.
///
/// # Safety
/// `closure` must be a live `() => void` closure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_co_new_gc(closure: *mut Closure) -> *mut Coro {
    with_heap(|h| {
        h.temps.push(closure as *mut u8);
        maybe_collect(h);
        h.temps.pop();
        let p = new_obj(h, HDR_CORO, std::mem::size_of::<Coro>()) as *mut Coro;
        // SAFETY: just allocated and zeroed.
        unsafe { (*p).closure = closure };
        p
    })
}

/// Runs the coroutine until it yields or finishes. Returns true when it has finished (the body
/// returned, or threw: the exception stays pending for the caller to take).
///
/// # Safety
/// `co` must be a live coroutine object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_co_resume(co: *mut Coro, sent: u64, sent_is_ptr: bool) -> bool {
    // SAFETY: by contract.
    unsafe {
        let c = &mut *co;
        match c.state {
            DONE => {
                c.yielded = 0;
                c.yielded_is_ptr = 0;
                return true;
            }
            RUNNING => {
                raise(2.0, "Generator is already running");
                return false;
            }
            _ => {}
        }
        // What the last step produced was read by the caller.
        c.yielded = 0;
        c.yielded_is_ptr = 0;
        c.sent = sent;
        c.sent_is_ptr = i64::from(sent_is_ptr);
        c.resumer_top = top();
        if c.state == NEW {
            let stack = DefaultStack::new(STACK_BYTES).expect("a native stack for the coroutine");
            let me = co;
            let inner = Box::new(Inner {
                co: Coroutine::with_stack(stack, move |yielder, ()| {
                    let c = &mut *me;
                    (*c.inner).yielder = yielder;
                    // The base of this coroutine's frame chain.
                    let mut base = Frame { prev: top(), n: 0 };
                    c.bottom = &mut base;
                    set_top(&mut base);
                    let f: unsafe extern "C" fn(*mut u8) = std::mem::transmute((*c.closure).func);
                    f((*c.closure).env);
                }),
                yielder: std::ptr::null(),
            });
            c.inner = Box::into_raw(inner);
        } else {
            // Link the suspended frames under the resumer's, and continue them.
            (*c.bottom).prev = c.resumer_top;
            set_top(c.saved_top);
        }
        c.state = RUNNING;
        CURRENT.with(|s| s.borrow_mut().push(co));
        let r = (*c.inner).co.resume(());
        CURRENT.with(|s| {
            s.borrow_mut().pop();
        });
        set_top(c.resumer_top);
        match r {
            CoroutineResult::Yield(()) => {
                c.state = SUSPENDED;
                false
            }
            CoroutineResult::Return(()) => {
                c.state = DONE;
                true
            }
        }
    }
}

/// `yield value` inside a coroutine: switches back to the resumer and returns what the next resume
/// sends in.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_co_yield(value: u64, is_ptr: bool) -> u64 {
    let co = CURRENT.with(|s| s.borrow().last().copied());
    let Some(co) = co else {
        raise(1.0, "yield outside a generator");
        return 0;
    };
    // SAFETY: the coroutine is live and running on this stack.
    unsafe {
        let c = &mut *co;
        c.yielded = value;
        c.yielded_is_ptr = i64::from(is_ptr);
        c.saved_top = top();
        c.state = SUSPENDED;
        let y = &*(*c.inner).yielder;
        y.suspend(());
        c.yielded_is_ptr = 0;
        c.sent
    }
}

/// Sets what `yield` raises when the body is resumed next (null clears it).
///
/// # Safety
/// `co` must be a live coroutine object; `err` null or a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_co_inject_set(co: *mut Coro, err: *mut u8) {
    // SAFETY: by contract.
    unsafe { (*co).inject = err };
}

/// The error to raise at the `yield` just resumed (null when none); taken once.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_co_inject_take() -> *mut u8 {
    let co = CURRENT.with(|s| s.borrow().last().copied());
    let Some(co) = co else {
        return std::ptr::null_mut();
    };
    // SAFETY: the running coroutine is live.
    unsafe { std::mem::replace(&mut (*co).inject, std::ptr::null_mut()) }
}

/// `return value` in the running generator body: the value `next()` reports when it is done.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_co_set_return(value: u64, is_ptr: bool) {
    let co = CURRENT.with(|s| s.borrow().last().copied());
    if let Some(co) = co {
        // SAFETY: the running coroutine is live.
        unsafe {
            (*co).yielded = value;
            (*co).yielded_is_ptr = i64::from(is_ptr);
        }
    }
}

/// The coroutine whose body is running (null outside any).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_co_current() -> *mut Coro {
    CURRENT.with(|s| s.borrow().last().copied().unwrap_or(std::ptr::null_mut()))
}

/// Has the body started (or finished)?
///
/// # Safety
/// `co` must be a live coroutine object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_co_started(co: *const Coro) -> bool {
    // SAFETY: by contract.
    unsafe { (*co).state != NEW }
}

/// The value the coroutine last yielded (the slot's bits).
///
/// # Safety
/// `co` must be a live coroutine object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_co_value(co: *const Coro) -> u64 {
    // SAFETY: by contract.
    unsafe { (*co).yielded }
}

/// Has the coroutine finished?
///
/// # Safety
/// `co` must be a live coroutine object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_co_done(co: *const Coro) -> bool {
    // SAFETY: by contract.
    unsafe { (*co).state == DONE }
}

/// `generator.return()`: finishes the coroutine without running the rest of its body.
///
/// # Safety
/// `co` must be a live coroutine object that is not running.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_co_finish(co: *mut Coro) {
    // SAFETY: by contract.
    unsafe {
        if (*co).state != RUNNING {
            free(co);
            (*co).state = DONE;
            (*co).yielded = 0;
            (*co).yielded_is_ptr = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn counter(_env: *mut u8) {
        // A frame with one pointer slot, as compiled code would have, holding a live object.
        let mut frame = [0u64; 3];
        // SAFETY: the frame is 16 + 8 bytes and stays valid until the pop.
        unsafe { ts_rt_frame_push(frame.as_mut_ptr() as *mut Frame, 1) };
        let obj = ts_rt_obj_new(HDR_STR, 32);
        // SAFETY: slot 2 is the frame's one pointer slot; the collector reads it through the chain.
        unsafe { std::ptr::write_volatile(&mut frame[2], obj as u64) };
        for i in 1..=3u64 {
            let got = ts_rt_co_yield(i, false);
            assert_eq!(got, 100 + i);
            ts_rt_gc_collect();
        }
        ts_rt_frame_pop();
    }

    // M9-TS-381
    #[test]
    fn a_coroutine_yields_resumes_and_keeps_its_frames_alive_while_suspended() {
        let closure = ts_rt_closure_new(counter as *const u8, std::ptr::null_mut());
        // SAFETY: a live closure and coroutine.
        unsafe {
            let co = ts_rt_co_new_gc(closure);
            let root = ts_rt_root_add(co as *mut u8);
            let mut seen = Vec::new();
            let mut sent = 100;
            while !ts_rt_co_resume(co, sent, false) {
                seen.push(ts_rt_co_value(co));
                sent += 1;
                ts_rt_gc_collect();
            }
            ts_rt_root_remove(root);
            assert_eq!(seen, [1, 2, 3]);
            assert!(ts_rt_co_done(co));
            assert!(
                ts_rt_co_resume(co, 0, false),
                "a finished coroutine stays finished"
            );
        }
        ts_rt_gc_stress(true);
        let closure = ts_rt_closure_new(counter as *const u8, std::ptr::null_mut());
        // SAFETY: as above; a suspended coroutine is finished early.
        unsafe {
            let co = ts_rt_co_new_gc(closure);
            let root = ts_rt_root_add(co as *mut u8);
            assert!(!ts_rt_co_resume(co, 101, false));
            ts_rt_co_finish(co);
            assert!(ts_rt_co_done(co));
            ts_rt_root_remove(root);
        }
        ts_rt_gc_stress(false);
    }
}
