use std::future::Future;

pub struct AppExecutor {
    runtime: tokio::runtime::Runtime,
}

impl iced::Executor for AppExecutor {
    fn new() -> Result<Self, iced_futures::futures::io::Error> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;

        Ok(Self { runtime })
    }

    #[allow(clippy::let_underscore_future)]
    fn spawn(&self, future: impl Future<Output = ()> + iced_futures::MaybeSend + 'static) {
        let _ = self.runtime.spawn(future);
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.runtime.enter();
        f()
    }
}
