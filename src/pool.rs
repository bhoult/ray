//! `Shared`: a slice that parallel jobs write through, each into its own
//! disjoint part (fraktal256's pool.rs; the thread pool itself is rayon here).

pub struct Shared<'a, T> {
    ptr: *mut T,
    len: usize,
    _life: core::marker::PhantomData<&'a mut [T]>,
}

unsafe impl<T: Send> Sync for Shared<'_, T> {}
unsafe impl<T: Send> Send for Shared<'_, T> {}

impl<'a, T> Shared<'a, T> {
    pub fn new(s: &'a mut [T]) -> Self {
        Shared { ptr: s.as_mut_ptr(), len: s.len(), _life: core::marker::PhantomData }
    }

    /// `len` elements from `start`.
    ///
    /// # Safety
    /// No other job may access these elements while the slice lives.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn slice(&self, start: usize, len: usize) -> &mut [T] {
        assert!(start <= self.len && len <= self.len - start);
        unsafe { core::slice::from_raw_parts_mut(self.ptr.add(start), len) }
    }
}
