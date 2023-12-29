use std::fmt::Debug;
use std::sync::Arc;
use log::{info, warn};
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time;

#[derive(Clone, Debug)]
pub enum SchedulerCommand {
    Suspend,
    Resume,
    Terminate
}

pub trait DeletionTask<E> {
    fn delete(&mut self, created_before: Duration) -> Result<(), E>;
}

pub type MutexDeletionTask<E> = Arc<Mutex<dyn DeletionTask<E> + Send>>;

// Must be async as required by tokio::select!
async fn repeat<E: Debug>(task: &MutexDeletionTask<E>, period: Duration, mut rx: Receiver<SchedulerCommand>) {
    let mut interval = time::interval(period);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut task = task.lock().await;
                if let Err(e) = task.delete(period) {
                    warn!("Deletion task failed: {:?}, leave scheduler", e);
                    break;
                }
            },
            value = rx.recv() => {
                info!("-----> {:?}", value.unwrap());
                info!("Termination signal received, leave deletion scheduler");
                break;
            }
        }
    }
}

pub fn spawn_deletion_scheduler<E: Debug + 'static>(task: &MutexDeletionTask<E>, rx: Receiver<SchedulerCommand>, period: Duration) -> JoinHandle<()> {
    info!("Spawn deletion scheduler");
    let task = task.clone();
    tokio::spawn(async move {
        repeat(&task, period, rx).await;
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::broadcast;
    use tokio::time::sleep;
    use crate::scheduler::{MutexDeletionTask, spawn_deletion_scheduler, DeletionTask};

    #[derive(Debug)]
    enum TestError {}

    struct TestTask {
        counter: u128
    }

    impl TestTask {
        fn new() -> Self {
            Self { counter: 0 }
        }
    }

    impl DeletionTask<TestError> for TestTask {
        fn delete(&mut self, created_before: Duration) -> Result<(), TestError> {
            self.counter += created_before.as_millis();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_scheduler() {
        let task = Arc::new(Mutex::new(TestTask::new()));
        let cloned : MutexDeletionTask<TestError> = task.clone();
        let (tx, rx) = broadcast::channel(1);
        let handle = spawn_deletion_scheduler(&cloned, rx, Duration::from_millis(1));
        sleep(Duration::from_millis(10)).await;
        assert!(tx.send(()).is_ok()); // Terminate scheduler
        assert!(handle.await.is_ok());
        let task = task.lock().unwrap();
        assert!(task.counter > 0); // TestTask::delete() was called at least once
    }
}