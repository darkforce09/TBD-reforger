//! Bounded publication of committed audit facts, retried independently of notifications.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::administration::services::audit_publication::publish_audit_batch;

const PUBLICATION_INTERVAL: Duration = Duration::from_millis(250);
const BATCH_SIZE: i64 = 1000;
const MAX_BATCHES_PER_TICK: usize = 10;

/// Publish immediately, then pause 250 milliseconds between bounded publication passes.
///
/// A pass attempts at most ten batches and stops on an empty batch or an error. Failures
/// leave the durable queue intact and are retried on the next pass, including when the
/// notification listener remains healthy. Closing the pool stops this worker.
pub fn start_audit_publication(pool: PgPool) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if pool.is_closed() {
                return;
            }

            for _ in 0..MAX_BATCHES_PER_TICK {
                match publish_audit_batch(&pool, BATCH_SIZE).await {
                    Ok(count) if count < BATCH_SIZE as u64 => break,
                    Ok(_) => {}
                    Err(error) => {
                        tracing::error!(%error, "audit publication failed; retrying next interval");
                        break;
                    }
                }
            }

            tokio::time::sleep(PUBLICATION_INTERVAL).await;
        }
    })
}
