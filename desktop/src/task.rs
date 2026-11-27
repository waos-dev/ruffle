//! WaOS port: where the program's tasks run, and how it waits for one.
//!
//! On a desktop tokio's runtime runs them, on its threads, and a wait parks the thread. A WaOS
//! app is a WASI component with one thread: its tasks run on winit's event loop, which is the
//! app's loop, and a wait is `wasdk`'s, the only one that can wait there, since an executor that
//! parks the thread is never woken. So a task on WaOS need not be `Send`, and there is no
//! runtime to keep.
//!
//! The invariant on WaOS: a task is started here and never with `wasdk::spawn`, which hands it
//! to whichever wait runs next and holds that wait until the task ends.

use std::future::Future;

/// Starts `task` beside the program's loop.
#[cfg(not(target_os = "wasi"))]
pub fn spawn<F>(task: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(task);
}

/// Starts `task` beside the program's loop.
#[cfg(target_os = "wasi")]
pub fn spawn<F>(task: F)
where
    F: Future<Output = ()> + 'static,
{
    winit::platform::waos::spawn_local(task);
}

/// Waits on the program's thread for `future`'s answer.
pub fn block_on<F: Future>(future: F) -> F::Output {
    #[cfg(not(target_os = "wasi"))]
    return futures::executor::block_on(future);
    #[cfg(target_os = "wasi")]
    return wasdk::block_on(future);
}

/// What runs the program's tasks, kept for as long as the program runs.
pub struct Runtime {
    #[cfg(not(target_os = "wasi"))]
    tokio: tokio::runtime::Runtime,
}

impl Runtime {
    pub fn new() -> std::io::Result<Runtime> {
        Ok(Runtime {
            #[cfg(not(target_os = "wasi"))]
            tokio: tokio::runtime::Runtime::new()?,
        })
    }

    /// Makes this the runtime tasks are started on, until what is answered is dropped.
    #[cfg(not(target_os = "wasi"))]
    pub fn enter(&self) -> tokio::runtime::EnterGuard<'_> {
        self.tokio.enter()
    }

    /// Nothing on WaOS, whose executor is the app's own from its start.
    #[cfg(target_os = "wasi")]
    pub fn enter(&self) {}

    /// Stops the runtime, giving its tasks `timeout` to end.
    pub fn shutdown_timeout(self, timeout: std::time::Duration) {
        #[cfg(not(target_os = "wasi"))]
        self.tokio.shutdown_timeout(timeout);
        #[cfg(target_os = "wasi")]
        let _ = timeout;
    }
}
