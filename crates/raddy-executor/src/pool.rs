//! Warm instance pool. Spent slots are never reused (C4.1).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use wasmtime::{Instance, InstancePre, Store};

use crate::ExecError;
use crate::engine::EngineFacade;
use crate::host::HostState;
use crate::slot::deadline_ticks;

pub(crate) struct ReadySlot {
    pub store: Store<HostState>,
    pub instance: Instance,
}

struct PoolInner {
    facade: EngineFacade,
    pre: InstancePre<HostState>,
    idle: Mutex<Vec<ReadySlot>>,
    in_use: AtomicUsize,
    min: usize,
    max: usize,
    stop: AtomicBool,
    response_done: Mutex<Option<std::time::Instant>>,
    teardown_done: Mutex<Option<std::time::Instant>>,
}

/// Pre-instantiated guests. Steal, execute once, drop. Background refill to `min`.
#[derive(Clone)]
pub struct InstancePool {
    inner: Arc<PoolInner>,
}

impl std::fmt::Debug for InstancePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstancePool")
            .field("idle", &self.idle_count())
            .field("in_use", &self.in_use())
            .field("min", &self.inner.min)
            .field("max", &self.inner.max)
            .finish()
    }
}

impl InstancePool {
    pub(crate) fn new(
        facade: EngineFacade,
        pre: InstancePre<HostState>,
        min: usize,
        max: usize,
    ) -> Result<Self, ExecError> {
        let max = max.max(1);
        let min = min.min(max);
        let inner = Arc::new(PoolInner {
            facade,
            pre,
            idle: Mutex::new(Vec::with_capacity(min)),
            in_use: AtomicUsize::new(0),
            min,
            max,
            stop: AtomicBool::new(false),
            response_done: Mutex::new(None),
            teardown_done: Mutex::new(None),
        });
        let pool = Self {
            inner: Arc::clone(&inner),
        };
        pool.fill_to_min()?;
        if min > 0 {
            let bg = Arc::clone(&inner);
            thread::Builder::new()
                .name("raddy-pool-refill".into())
                .spawn(move || refill_loop(bg))
                .expect("pool refill thread starts");
        }
        Ok(pool)
    }

    pub fn steal(&self) -> Result<Stolen, ExecError> {
        let popped = self
            .inner
            .idle
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop();
        let slot = match popped {
            Some(slot) => slot,
            None => {
                if self.inner.in_use.load(Ordering::SeqCst) >= self.inner.max {
                    return Err(ExecError::Saturated);
                }
                self.mint()?
            }
        };
        self.inner.in_use.fetch_add(1, Ordering::SeqCst);
        Ok(Stolen {
            slot: Some(slot),
            inner: Arc::clone(&self.inner),
            spent: false,
        })
    }

    #[must_use]
    pub fn idle_count(&self) -> usize {
        self.inner
            .idle
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    #[must_use]
    pub fn in_use(&self) -> usize {
        self.inner.in_use.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn min(&self) -> usize {
        self.inner.min
    }

    pub fn note_response_done(&self) {
        *self
            .inner
            .response_done
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
    }

    pub fn note_teardown(&self) {
        *self
            .inner
            .teardown_done
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
    }

    #[must_use]
    pub fn teardown_followed_response(&self) -> bool {
        let inner = &self.inner;
        let resp = *inner
            .response_done
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tear = *inner
            .teardown_done
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        matches!((resp, tear), (Some(r), Some(t)) if t >= r)
    }

    fn fill_to_min(&self) -> Result<(), ExecError> {
        while self.idle_count() < self.inner.min
            && self.idle_count() + self.in_use() < self.inner.max
        {
            let slot = self.mint()?;
            self.inner
                .idle
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(slot);
        }
        Ok(())
    }

    fn mint(&self) -> Result<ReadySlot, ExecError> {
        mint_slot(&self.inner)
    }
}

impl Drop for InstancePool {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            self.inner.stop.store(true, Ordering::Relaxed);
        }
    }
}

/// A stolen ready instance. Drop returns it unless [`Stolen::mark_spent`].
pub struct Stolen {
    slot: Option<ReadySlot>,
    inner: Arc<PoolInner>,
    spent: bool,
}

impl std::fmt::Debug for Stolen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stolen")
            .field("spent", &self.spent)
            .finish_non_exhaustive()
    }
}

impl Stolen {
    pub(crate) fn slot_mut(&mut self) -> &mut ReadySlot {
        self.slot
            .as_mut()
            .expect("stolen slot is present until drop")
    }

    pub fn mark_spent(&mut self) {
        self.spent = true;
    }
}

impl Drop for Stolen {
    fn drop(&mut self) {
        self.inner.in_use.fetch_sub(1, Ordering::SeqCst);
        let Some(slot) = self.slot.take() else {
            return;
        };
        if self.spent {
            drop(slot);
            return;
        }
        self.inner
            .idle
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(slot);
    }
}

fn mint_slot(inner: &PoolInner) -> Result<ReadySlot, ExecError> {
    let host = HostState::vacant();
    let mut store = Store::new(inner.facade.engine(), host);
    store.set_epoch_deadline(deadline_ticks(
        Duration::from_secs(30),
        inner.facade.epoch_tick(),
    ));
    let instance = inner
        .pre
        .instantiate(&mut store)
        .map_err(|err| ExecError::Artifact(err.to_string()))?;
    Ok(ReadySlot { store, instance })
}

fn refill_loop(inner: Arc<PoolInner>) {
    while !inner.stop.load(Ordering::Relaxed) {
        let idle = inner.idle.lock().unwrap_or_else(|e| e.into_inner()).len();
        let used = inner.in_use.load(Ordering::SeqCst);
        if idle < inner.min
            && idle + used < inner.max
            && let Ok(slot) = mint_slot(&inner)
        {
            inner
                .idle
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(slot);
        }
        thread::sleep(Duration::from_millis(5));
    }
}
