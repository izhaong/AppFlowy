use crate::util::tokio_default_runtime;
use std::{future::Future, io};
use tokio::{runtime, task::LocalSet};

#[derive(Debug)]
pub struct Runtime {
    local: LocalSet,
    rt: runtime::Runtime,
}

impl Runtime {
    pub fn new() -> io::Result<Runtime> {
        let rt = tokio_default_runtime()?;
        Ok(Runtime {
            rt,
            local: LocalSet::new(),
        })
    }

    pub fn spawn<F>(&self, future: F) -> &Self
    where
        F: Future<Output = ()> + 'static,
    {
        self.local.spawn_local(future);
        self
    }

    pub fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + 'static,
    {
        self.local.block_on(&self.rt, f)
    }
}
