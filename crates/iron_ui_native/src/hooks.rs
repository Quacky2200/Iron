//! Hooks: `useState`, `useRef`, `useMemo`, `useCallback`, `useEffect`, `useContext`. Their state
//! lives in slots owned by the component's instance, in call order (the rules of hooks).

use crate::element::Cleanup;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A dependency value. Compared as `Object.is` does.
#[derive(Clone, Debug)]
pub enum Dep {
    Num(f64),
    Str(String),
    Bool(bool),
    Id(u64),
}

impl PartialEq for Dep {
    fn eq(&self, other: &Dep) -> bool {
        match (self, other) {
            (Dep::Num(a), Dep::Num(b)) => {
                (a.is_nan() && b.is_nan())
                    || (a == b && a.is_sign_negative() == b.is_sign_negative())
            }
            (Dep::Str(a), Dep::Str(b)) => a == b,
            (Dep::Bool(a), Dep::Bool(b)) => a == b,
            (Dep::Id(a), Dep::Id(b)) => a == b,
            _ => false,
        }
    }
}

pub type EffectFn = Box<dyn FnOnce() -> Option<Cleanup>>;

pub(crate) enum Slot {
    /// An `Rc<RefCell<T>>` for a state or a ref.
    Cell(Rc<dyn Any>),
    Memo {
        deps: Vec<Dep>,
        value: Option<Rc<dyn Any>>,
    },
    Effect {
        deps: Option<Vec<Dep>>,
        mounted: bool,
        cleanup: Option<Cleanup>,
        pending: Option<EffectFn>,
    },
}

/// What the scheduler shares with the setters: which components must render again.
#[derive(Default)]
pub(crate) struct Shared {
    pub dirty: RefCell<Vec<usize>>,
}

impl Shared {
    pub fn mark(&self, inst: usize) {
        self.dirty.borrow_mut().push(inst);
    }
}

/// Asks for the component to render again.
#[derive(Clone)]
pub struct Scheduler {
    shared: Rc<Shared>,
    inst: usize,
}

impl Scheduler {
    pub fn mark(&self) {
        self.shared.mark(self.inst);
    }
}

/// A state setter. Setting the value it already has schedules nothing (`Object.is`).
pub struct Setter<T> {
    cell: Rc<RefCell<T>>,
    shared: Rc<Shared>,
    inst: usize,
}

impl<T> Clone for Setter<T> {
    fn clone(&self) -> Self {
        Setter {
            cell: self.cell.clone(),
            shared: self.shared.clone(),
            inst: self.inst,
        }
    }
}

impl<T: PartialEq + 'static> Setter<T> {
    pub fn set(&self, v: T) {
        if *self.cell.borrow() == v {
            return;
        }
        *self.cell.borrow_mut() = v;
        self.shared.mark(self.inst);
    }

    /// `setX(prev => next)`.
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        let next = f(&self.cell.borrow());
        self.set(next);
    }
}

/// What a component's render function gets: the hooks.
pub struct Ctx<'r> {
    pub(crate) slots: &'r mut Vec<Slot>,
    pub(crate) idx: usize,
    pub(crate) shared: Rc<Shared>,
    pub(crate) inst: usize,
    pub(crate) lookup: &'r dyn Fn(u32) -> Option<Rc<dyn Any>>,
    /// Effect slots whose dependencies changed (their cleanup runs, then the effect).
    pub(crate) effects: Vec<usize>,
}

impl Ctx<'_> {
    fn next_slot(&mut self, make: impl FnOnce() -> Slot) -> usize {
        let i = self.idx;
        self.idx += 1;
        if i == self.slots.len() {
            self.slots.push(make());
        }
        i
    }

    /// A hook slot holding a value of your own type (the script bridge keeps its cells here).
    pub fn use_shared<T: 'static>(&mut self, make: impl FnOnce() -> T) -> Rc<T> {
        let i = self.next_slot(|| Slot::Cell(Rc::new(make())));
        let Slot::Cell(c) = &self.slots[i] else {
            panic!("hooks changed order: expected a shared cell in slot {i}")
        };
        c.clone()
            .downcast::<T>()
            .unwrap_or_else(|_| panic!("hooks changed type in slot {i}"))
    }

    /// Marks this component dirty when asked, from anywhere (a setter, a subscription).
    pub fn scheduler(&self) -> Scheduler {
        Scheduler {
            shared: self.shared.clone(),
            inst: self.inst,
        }
    }

    /// `useState`: the current value and its setter.
    pub fn use_state<T: Clone + PartialEq + 'static>(
        &mut self,
        init: impl FnOnce() -> T,
    ) -> (T, Setter<T>) {
        let i = self.next_slot(|| Slot::Cell(Rc::new(RefCell::new(init()))));
        let Slot::Cell(c) = &self.slots[i] else {
            panic!("hooks changed order: expected state in slot {i}")
        };
        let cell = c
            .clone()
            .downcast::<RefCell<T>>()
            .unwrap_or_else(|_| panic!("hooks changed type in slot {i}"));
        let v = cell.borrow().clone();
        (
            v,
            Setter {
                cell,
                shared: self.shared.clone(),
                inst: self.inst,
            },
        )
    }

    /// `useRef`: a mutable box that survives renders and does not cause one.
    pub fn use_ref<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Rc<RefCell<T>> {
        let i = self.next_slot(|| Slot::Cell(Rc::new(RefCell::new(init()))));
        let Slot::Cell(c) = &self.slots[i] else {
            panic!("hooks changed order: expected ref in slot {i}")
        };
        c.clone()
            .downcast::<RefCell<T>>()
            .unwrap_or_else(|_| panic!("hooks changed type in slot {i}"))
    }

    /// `useMemo`: recomputes when `deps` changed.
    pub fn use_memo<T: Clone + 'static>(&mut self, deps: &[Dep], f: impl FnOnce() -> T) -> T {
        let i = self.next_slot(|| Slot::Memo {
            deps: Vec::new(),
            value: None,
        });
        let Slot::Memo { deps: old, value } = &mut self.slots[i] else {
            panic!("hooks changed order: expected memo in slot {i}")
        };
        if value.is_none() || old.as_slice() != deps {
            *value = Some(Rc::new(f()));
            *old = deps.to_vec();
        }
        value
            .as_ref()
            .and_then(|v| v.downcast_ref::<T>())
            .unwrap_or_else(|| panic!("hooks changed type in slot {i}"))
            .clone()
    }

    /// `useCallback`: `f` itself until `deps` change.
    pub fn use_callback<A: 'static, R: 'static>(
        &mut self,
        deps: &[Dep],
        f: impl Fn(A) -> R + 'static,
    ) -> Rc<dyn Fn(A) -> R> {
        self.use_memo(deps, || Rc::new(f) as Rc<dyn Fn(A) -> R>)
    }

    /// `useEffect`: with `deps: None` after every render, with `Some(&[])` once, otherwise when
    /// a dependency changed. The returned cleanup runs before the next run and on unmount.
    pub fn use_effect(
        &mut self,
        deps: Option<&[Dep]>,
        f: impl FnOnce() -> Option<Cleanup> + 'static,
    ) {
        let i = self.next_slot(|| Slot::Effect {
            deps: None,
            mounted: false,
            cleanup: None,
            pending: None,
        });
        let Slot::Effect {
            deps: old,
            mounted,
            pending,
            ..
        } = &mut self.slots[i]
        else {
            panic!("hooks changed order: expected effect in slot {i}")
        };
        let changed = match deps {
            None => true,
            Some(d) => !*mounted || old.as_deref() != Some(d),
        };
        *mounted = true;
        if changed {
            *old = deps.map(<[Dep]>::to_vec);
            *pending = Some(Box::new(f));
            self.effects.push(i);
        }
    }

    /// The value of the nearest provider of context `id`, if there is one.
    pub fn find_context(&self, id: u32) -> Option<Rc<dyn Any>> {
        (self.lookup)(id)
    }

    /// `useContext`: the nearest provider's value, else `default`.
    pub fn use_context<T: Clone + 'static>(&mut self, context: &Context<T>) -> T {
        match (self.lookup)(context.id).and_then(|v| v.downcast_ref::<T>().cloned()) {
            Some(v) => v,
            None => context.default.clone(),
        }
    }
}

/// `createContext(default)`.
pub struct Context<T> {
    pub id: u32,
    pub default: T,
}

impl<T: Clone + 'static> Context<T> {
    pub fn new(id: u32, default: T) -> Self {
        Context { id, default }
    }

    /// `<Context.Provider value={value}>children</Context.Provider>`.
    pub fn provider(&self, value: T, children: Vec<crate::Element>) -> crate::Element {
        crate::Element::Provider {
            key: None,
            context: self.id,
            value: Rc::new(value),
            children,
        }
    }
}
