// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

pub type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Runtime abstraction — tw_spv does not force a specific async runtime.
pub trait SpvRuntime: Send + Sync {
    fn spawn(&self, task: BoxFuture);
    fn spawn_blocking(&self, task: BoxFuture);
    fn now(&self) -> SystemTime;
}
