pub struct Defer<F: FnOnce()> {
    pub f: Option<F>,
}

impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            f();
        }
    }
}

#[macro_export]
macro_rules! defer {
    ($id:ident, $($body:tt)*) => {
        let $id = $crate::defer::Defer { f: Some(|| { $($body)* }) };
        let _ = &$id;
    };
}
