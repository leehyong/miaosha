use iced::executor::Executor as TraitExecutor;
use iced_native::executor::Tokio;
use tokio::runtime::Builder;

#[derive(Debug)]
pub struct MiaoshaExecutor(Tokio);

impl TraitExecutor for MiaoshaExecutor {
    fn new() -> Result<Self, futures::io::Error> {
        let cpus = num_cpus::get();
        let workers = std::cmp::max(cpus, 2);
        // 启动cpu 4 倍 的线程
        let mrt = Builder::new_multi_thread().enable_all()
            .worker_threads(workers * 4)
            .build()?;
        Ok(MiaoshaExecutor(mrt))
    }

    fn spawn(
        &self,
        future: impl futures::Future<Output = ()> + Send + 'static,
    ) {
        let _ = self.0.spawn(future);
    }

    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        TraitExecutor::enter(&self.0, f)
    }
}
