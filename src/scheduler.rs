use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use log::{info, warn};
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time;

#[async_trait] // TODO: Better use RestState directly and remove crate async-trait
pub trait DeletionTask<E> {
    async fn delete(&mut self, created_before: Duration) -> Result<(), E>;
}

pub type MutexDeletionTask<E> = Arc<Mutex<dyn DeletionTask<E> + Send>>;

// Must be async as required by tokio::select!
async fn repeat<E: Debug>(task: &MutexDeletionTask<E>, period: Duration, mut rx: Receiver<()>) {
    let mut interval = time::interval(period);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut task = task.lock().await;
                if let Err(e) = task.delete(period).await {
                    warn!("Deletion task failed: {:?}, leave scheduler", e);
                    break;
                }
            },
            _ = rx.recv() => {
                info!("Termination signal received, leave deletion scheduler");
                break;
            }
        }
    }
}

pub fn spawn_deletion_scheduler<E: Debug + 'static>(task: &MutexDeletionTask<E>, rx: Receiver<()>, period: Duration) -> JoinHandle<()> {
    info!("Spawn deletion scheduler");
    let task = task.clone();
    tokio::spawn(async move {
        repeat(&task, period, rx).await;
    })
}
