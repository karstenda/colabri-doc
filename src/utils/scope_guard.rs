pub struct ScopeGuard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> ScopeGuard<F> {
    pub fn new(f: F) -> Self {
        Self(Some(f))
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}
