use crate::{
    module::{as_module_map, Module, ModuleMap},
    rt::Runtime,
};
use futures_core::{ready, task::Context};
use std::{cell::RefCell, fmt::Debug, future::Future, io, sync::Arc};
use tokio::{
    macros::support::{Pin, Poll},
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

thread_local!(
    static CURRENT: RefCell<Option<Arc<FlowySystem>>> = RefCell::new(None);
);

#[derive(Debug)]
pub enum SystemCommand {
    Exit(i8),
}

pub struct FlowySystem {
    sys_cmd_tx: UnboundedSender<SystemCommand>,
}

impl FlowySystem {
    pub fn construct<F, S>(module_factory: F, sender_factory: S) -> SystemRunner
    where
        F: FnOnce() -> Vec<Module>,
        S: FnOnce(ModuleMap, &Runtime),
    {
        let runtime = Arc::new(Runtime::new().unwrap());
        let (sys_cmd_tx, sys_cmd_rx) = unbounded_channel::<SystemCommand>();
        let (stop_tx, stop_rx) = oneshot::channel();

        runtime.spawn(SystemController {
            stop_tx: Some(stop_tx),
            sys_cmd_rx,
        });

        let module_map = as_module_map(module_factory());
        sender_factory(module_map.clone(), &runtime);

        let system = Self { sys_cmd_tx };
        FlowySystem::set_current(system);
        let runner = SystemRunner {
            rt: runtime,
            stop_rx,
        };
        runner
    }

    pub fn stop(&self) {
        match self.sys_cmd_tx.send(SystemCommand::Exit(0)) {
            Ok(_) => {},
            Err(e) => {
                log::error!("Stop system error: {}", e);
            },
        }
    }

    #[doc(hidden)]
    pub fn set_current(sys: FlowySystem) {
        CURRENT.with(|cell| {
            *cell.borrow_mut() = Some(Arc::new(sys));
        })
    }

    pub fn current() -> Arc<FlowySystem> {
        CURRENT.with(|cell| match *cell.borrow() {
            Some(ref sys) => sys.clone(),
            None => panic!("System is not running"),
        })
    }
}

struct SystemController {
    stop_tx: Option<oneshot::Sender<i8>>,
    sys_cmd_rx: UnboundedReceiver<SystemCommand>,
}

impl Future for SystemController {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match ready!(Pin::new(&mut self.sys_cmd_rx).poll_recv(cx)) {
                None => return Poll::Ready(()),
                Some(cmd) => match cmd {
                    SystemCommand::Exit(code) => {
                        if let Some(tx) = self.stop_tx.take() {
                            let _ = tx.send(code);
                        }
                    },
                },
            }
        }
    }
}

pub struct SystemRunner {
    rt: Arc<Runtime>,
    stop_rx: oneshot::Receiver<i8>,
}

impl SystemRunner {
    pub fn run(self) -> io::Result<()> {
        let SystemRunner { rt, stop_rx } = self;
        match rt.block_on(stop_rx) {
            Ok(code) => {
                if code != 0 {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Non-zero exit code: {}", code),
                    ))
                } else {
                    Ok(())
                }
            },
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    pub fn spawn<F: Future<Output = ()> + 'static>(self, future: F) -> Self {
        self.rt.spawn(future);
        self
    }
}
