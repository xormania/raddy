/// Capability stub. Every namespace is denied.
#[derive(Clone, Debug, Default)]
pub struct MockCapabilities;

impl MockCapabilities {
    /// Every namespace is denied. Guest-facing errno is negative.
    pub fn call(&self, namespace: &str, request: &[u8]) -> Result<Vec<u8>, i32> {
        let _ = (namespace, request);
        Err(-1)
    }
}
