//! Warm instance pool. Spent slots are never reused (C4.1).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
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
    peak_in_use: AtomicUsize,
    total: AtomicUsize,
    min: usize,
    max: usize,
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
            .field("peak_in_use", &self.peak_in_use())
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
            peak_in_use: AtomicUsize::new(0),
            total: AtomicUsize::new(0),
            min,
            max,
            response_done: Mutex::new(None),
            teardown_done: Mutex::new(None),
        });
        let pool = Self {
            inner: Arc::clone(&inner),
        };
        pool.fill_to_min()?;
        if min > 0 {
            let bg = Arc::downgrade(&inner);
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
                if !reserve_slot(&self.inner) {
                    return Err(ExecError::Saturated);
                }
                match self.mint() {
                    Ok(slot) => slot,
                    Err(err) => {
                        self.inner.total.fetch_sub(1, Ordering::SeqCst);
                        return Err(err);
                    }
                }
            }
        };
        let in_use = self.inner.in_use.fetch_add(1, Ordering::SeqCst) + 1;
        self.inner.peak_in_use.fetch_max(in_use, Ordering::SeqCst);
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
    pub fn peak_in_use(&self) -> usize {
        self.inner.peak_in_use.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn min(&self) -> usize {
        self.inner.min
    }

    #[must_use]
    pub fn max(&self) -> usize {
        self.inner.max
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
        while self.idle_count() < self.inner.min && reserve_slot(&self.inner) {
            let slot = match self.mint() {
                Ok(slot) => slot,
                Err(err) => {
                    self.inner.total.fetch_sub(1, Ordering::SeqCst);
                    return Err(err);
                }
            };
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
            self.inner.total.fetch_sub(1, Ordering::SeqCst);
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

fn reserve_slot(inner: &PoolInner) -> bool {
    inner
        .total
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |total| {
            (total < inner.max).then_some(total + 1)
        })
        .is_ok()
}

fn refill_loop(owner: Weak<PoolInner>) {
    loop {
        let Some(inner) = owner.upgrade() else {
            return;
        };
        let idle = inner.idle.lock().unwrap_or_else(|e| e.into_inner()).len();
        if idle < inner.min && reserve_slot(&inner) {
            match mint_slot(&inner) {
                Ok(slot) => inner
                    .idle
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(slot),
                Err(_) => {
                    inner.total.fetch_sub(1, Ordering::SeqCst);
                }
            }
        }
        drop(inner);
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EngineBuilder, RestoreStrategy, ToyExecutor, snap_guest_wizer};

    #[test]
    fn refill_worker_releases_the_last_pool_owner() {
        let facade = EngineBuilder::new().build().expect("engine");
        let module = facade
            .load_wasm_bytes(snap_guest_wizer())
            .expect("snapshot module");
        let exec = ToyExecutor::with_strategy(
            facade,
            module,
            Duration::from_secs(2),
            RestoreStrategy::Snapshot,
        )
        .expect("executor")
        .with_pool(1, 2)
        .expect("pool");
        let owner = Arc::downgrade(&exec.pool().inner);

        drop(exec);

        for _ in 0..20 {
            if owner.upgrade().is_none() {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("refill worker retained the pool after its last owner was dropped");
    }
}
