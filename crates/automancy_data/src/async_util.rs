use core::{
    cell::UnsafeCell,
    fmt::Debug,
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};
use std::sync::Arc;

use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};

const MAX_SIZE: usize = 2;

#[repr(u8)]
enum BufStatus {
    Uninit,
    Idle,
    Updating,
}

/// A buffer that allows non-blocking access of some async value (except for initial read) that works by checking if the buffer is currently being updated, while taking a `FnOnce` that returns a `Future`:
/// - If `status` == `Idle`:
///     - Set the status to `Updating`
///     - Find the next available slot in the buffer
///     - Run the `FnOnce` and spawn a `tokio` task that moves and `await`s on the `Future`
///     - When the `Future` finishes:
///         - Store the value into the buffer
///         - Advance the current index by one
///         - Set the status to `Idle`
/// - If `status` == `Uninit`:
///     - The same as `status` == `Idle`, however, wait for the task to finish and return the initial value.
/// - If `status` == `Updating`:
///     - Return the newest available value.
///
/// This should only be used if up-to-date value is *not required*!
#[derive(Debug)]
pub struct AsyncSwapBuf<T> {
    buf: [UnsafeCell<MaybeUninit<T>>; MAX_SIZE],
    current_index: Arc<AtomicUsize>,
    status: Arc<AtomicU8>,
}

impl<T> Default for AsyncSwapBuf<T> {
    fn default() -> Self {
        Self {
            buf: [const { UnsafeCell::new(MaybeUninit::uninit()) }; MAX_SIZE],
            current_index: Arc::new(AtomicUsize::new(0)),
            status: Arc::new(AtomicU8::new(BufStatus::Uninit as u8)),
        }
    }
}

impl<T> AsyncSwapBuf<T> {
    pub fn new() -> AsyncSwapBuf<T> {
        Self::default()
    }
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl<T> AsyncSwapBuf<T>
where
    T: Debug + Send + 'static,
{
    /// Read the latest value of the buffer.
    ///
    /// Panics if [`AsyncSwapBuf::update`] hadn't been called yet.
    #[inline]
    #[must_use]
    pub fn read_latest(&self) -> &T {
        #[cfg(debug_assertions)]
        if self.status.load(Ordering::Relaxed) == BufStatus::Uninit as u8 {
            panic!("`AsyncSwapBuf::update` should be called before reading")
        }

        let index = self.current_index.load(Ordering::Relaxed);

        unsafe { (*self.buf[index].get()).assume_init_ref() }
    }

    pub fn update<F: Future<Output = T> + Send + 'static>(&mut self, f: impl FnOnce() -> F) -> Option<JoinHandle<()>> {
        let status = self.status.load(Ordering::Relaxed);
        if status == BufStatus::Updating as u8 {
            return None;
        }

        let status = self.status.swap(BufStatus::Updating as u8, Ordering::Acquire);
        let curr_index = self.current_index.load(Ordering::Acquire);
        let new_index = (curr_index + 1) % MAX_SIZE;

        debug_assert_ne!(curr_index, new_index);
        let new = self.buf.get_mut(new_index).unwrap();
        let handle = {
            // SAFETY: we should have unique access to `new`
            let value = unsafe { &mut *new.get() };
            let status = self.status.clone();
            let index = self.current_index.clone();

            let fut = f();
            tokio::spawn(async move {
                value.write(fut.await);
                #[cfg(debug_assertions)]
                debug_assert_eq!(BufStatus::Updating as u8, status.swap(BufStatus::Idle as u8, Ordering::Release));
                #[cfg(not(debug_assertions))]
                status.store(BufStatus::Idle as u8, Ordering::Release);
                index.store(new_index, Ordering::Release);
            })
        };

        // if the buffer is uninitialized, block until `new` has value.
        if status == BufStatus::Uninit as u8 {
            Handle::current().block_on(async move {
                handle.await.unwrap();
            });

            None
        } else {
            Some(handle)
        }
    }

    pub fn update_with_runtime<F: Future<Output = T> + Send + 'static>(
        &mut self,
        runtime: &Runtime,
        f: impl FnOnce() -> F,
    ) -> Option<JoinHandle<()>> {
        let guard = runtime.enter();
        let handle = self.update(f);
        drop(guard);
        handle
    }
}
