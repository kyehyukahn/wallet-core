// SPDX-License-Identifier: Apache-2.0

use crate::runtime::{BoxFuture, SpvRuntime};
use std::time::SystemTime;
use tokio::runtime::{Handle, Runtime};

/// Tokio-based implementation of [`SpvRuntime`].
pub struct TokioRuntime {
    rt: Runtime,
}

impl Default for TokioRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl TokioRuntime {
    /// Create a multi-thread runtime with 2 worker threads named "spv-worker".
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("spv-worker")
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        Self { rt }
    }

    /// Run an async future synchronously on this runtime.
    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.rt.block_on(future)
    }

    /// Get a handle to this runtime.
    pub fn handle(&self) -> Handle {
        self.rt.handle().clone()
    }
}

impl SpvRuntime for TokioRuntime {
    fn spawn(&self, task: BoxFuture) {
        self.rt.spawn(task);
    }

    fn spawn_blocking(&self, task: BoxFuture) {
        let handle = self.rt.handle().clone();
        self.rt.spawn_blocking(move || {
            handle.block_on(task);
        });
    }

    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::SpvRuntime;

    #[test]
    fn test_tokio_runtime_creation() {
        let rt = TokioRuntime::new();
        let now = rt.now();
        assert!(now.elapsed().unwrap().as_secs() < 1);
    }

    #[test]
    fn test_tokio_runtime_block_on() {
        let rt = TokioRuntime::new();
        let result = rt.block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_tokio_runtime_spawn() {
        let rt = TokioRuntime::new();
        let (tx, rx) = std::sync::mpsc::channel();
        rt.spawn(Box::pin(async move {
            tx.send(true).unwrap();
        }));
        let received = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert!(received);
    }

    #[test]
    fn test_tokio_runtime_dns_resolve() {
        let rt = TokioRuntime::new();
        let addrs = rt.block_on(async {
            tokio::net::lookup_host("localhost:8333")
                .await
                .expect("DNS resolution failed")
                .collect::<Vec<_>>()
        });
        assert!(!addrs.is_empty());
    }
}
