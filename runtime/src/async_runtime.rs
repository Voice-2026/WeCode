use std::{future::Future, sync::OnceLock};

pub use tokio::{
    sync::mpsc::{Receiver, Sender, channel},
    task::JoinHandle,
};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("codux-runtime")
            .build()
            .expect("failed to create Codux async runtime")
    })
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(future)
}

pub fn spawn_blocking<F, R>(function: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    runtime().spawn_blocking(function)
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    runtime().block_on(future)
}
