mod backend;
mod state;
mod types;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc as std_mpsc,
    },
    thread::{self, JoinHandle},
};

use tokio::sync::mpsc;

use backend::{SdlWorker, SharedState};

pub use types::{
    AxisPosition, BatteryInfo, BatteryState, ButtonState, ConnectionKind, ControllerAxis,
    ControllerButton, ControllerConfig, ControllerDevice, ControllerEvent, ControllerEventKind,
    ControllerSnapshot, DUALSENSE_PRODUCT_ID, SONY_VENDOR_ID, TriggerThresholds,
};

static RUNTIME_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    #[error("invalid controller configuration: {0}")]
    InvalidConfiguration(String),
    #[error("the controller runtime is already active")]
    AlreadyRunning,
    #[error("failed to start the controller worker: {0}")]
    WorkerStart(String),
    #[error("failed to initialize the SDL controller backend: {0}")]
    Initialization(String),
    #[error("the controller worker stopped during initialization")]
    InitializationStopped,
    #[error("the controller worker did not initialize within 10 seconds")]
    InitializationTimedOut,
}

pub type ControllerEventReceiver = mpsc::Receiver<ControllerEvent>;

#[derive(Clone)]
pub struct ControllerHandle {
    shared: Arc<SharedState>,
}

impl ControllerHandle {
    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Release);
    }

    pub fn paused(&self) -> bool {
        self.shared.paused.load(Ordering::Acquire)
    }

    pub fn snapshot(&self) -> ControllerSnapshot {
        self.shared.snapshot.read().clone()
    }

    pub fn request_shutdown(&self) {
        self.shared.shutdown.store(true, Ordering::Release);
    }
}

pub struct ControllerWorker {
    shared: Arc<SharedState>,
    thread: Option<JoinHandle<()>>,
}

impl ControllerWorker {
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        if let Some(worker) = self.thread.take() {
            if worker.thread().id() != thread::current().id() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for ControllerWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct ControllerRuntime {
    handle: ControllerHandle,
    events: Option<ControllerEventReceiver>,
    worker: Option<ControllerWorker>,
}

impl ControllerRuntime {
    pub fn start(config: ControllerConfig) -> Result<Self, ControllerError> {
        config
            .validate()
            .map_err(ControllerError::InvalidConfiguration)?;
        RUNTIME_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ControllerError::AlreadyRunning)?;

        let shared = Arc::new(SharedState::new(config.start_paused));
        let (event_sender, events) = mpsc::channel(config.event_capacity);
        let (initialization_sender, initialization_receiver) = std_mpsc::sync_channel(1);
        let worker_shared = Arc::clone(&shared);
        let thread = match thread::Builder::new()
            .name("dual-deck-controller".into())
            .spawn(move || {
                let _active_guard = RuntimeActiveGuard;
                match SdlWorker::initialize(config, event_sender, worker_shared) {
                    Ok(worker) => {
                        let _ = initialization_sender.send(Ok(()));
                        worker.run();
                    }
                    Err(error) => {
                        let _ = initialization_sender.send(Err(error));
                    }
                }
            }) {
            Ok(thread) => thread,
            Err(error) => {
                RUNTIME_ACTIVE.store(false, Ordering::Release);
                return Err(ControllerError::WorkerStart(error.to_string()));
            }
        };

        match initialization_receiver.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(())) => {
                let handle = ControllerHandle {
                    shared: Arc::clone(&shared),
                };
                let worker = ControllerWorker {
                    shared,
                    thread: Some(thread),
                };
                Ok(Self {
                    handle,
                    events: Some(events),
                    worker: Some(worker),
                })
            }
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(ControllerError::Initialization(error))
            }
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                let _ = thread.join();
                Err(ControllerError::InitializationStopped)
            }
            Err(std_mpsc::RecvTimeoutError::Timeout) => {
                shared.shutdown.store(true, Ordering::Release);
                drop(thread);
                Err(ControllerError::InitializationTimedOut)
            }
        }
    }

    pub fn handle(&self) -> ControllerHandle {
        self.handle.clone()
    }

    pub fn events(&mut self) -> &mut ControllerEventReceiver {
        self.events.as_mut().unwrap()
    }

    pub fn into_parts(mut self) -> (ControllerHandle, ControllerEventReceiver, ControllerWorker) {
        (
            self.handle.clone(),
            self.events.take().unwrap(),
            self.worker.take().unwrap(),
        )
    }
}

struct RuntimeActiveGuard;

impl Drop for RuntimeActiveGuard {
    fn drop(&mut self) {
        RUNTIME_ACTIVE.store(false, Ordering::Release);
    }
}
