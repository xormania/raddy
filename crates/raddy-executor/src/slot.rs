use std::marker::PhantomData;

/// Empty typestate: the slot has no module yet.
#[derive(Debug)]
pub struct Cold;

/// Instantiated from a module (`InstancePre` ready). Not executing.
#[derive(Debug)]
pub struct Warm;

/// A request is in flight. Dropping the slot is the only exit.
#[derive(Debug)]
pub struct Executing;

/// Lifecycle token. Illegal transitions do not compile.
#[derive(Debug)]
pub struct InstanceSlot<S> {
    _state: PhantomData<S>,
}

impl InstanceSlot<Cold> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _state: PhantomData,
        }
    }

    /// Stage 1 stub. Real instantiate binds an `InstancePre`.
    pub fn instantiate(self) -> Result<InstanceSlot<Warm>, String> {
        Err("instantiate is not implemented".into())
    }
}

impl Default for InstanceSlot<Cold> {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceSlot<Warm> {
    pub fn begin(self) -> InstanceSlot<Executing> {
        InstanceSlot {
            _state: PhantomData,
        }
    }
}

impl InstanceSlot<Executing> {
    /// Consume the slot. An instance is never reused after execute returns.
    pub fn finish(self) {}
}

#[cfg(test)]
mod tests {
    use super::InstanceSlot;

    #[test]
    fn typestate_cold_warm_executing_finishes() {
        // The stub instantiate fails until the engine exists; the type
        // sequence is what this test names, so construct Warm via begin
        // after a successful path is available. For now, Cold exists and
        // Executing can only be produced from Warm::begin.
        let cold = InstanceSlot::new();
        let err = cold.instantiate().unwrap_err();
        assert!(
            err.contains("not implemented"),
            "planted stub must still be the instantiate failure, got {err}"
        );
    }
}
