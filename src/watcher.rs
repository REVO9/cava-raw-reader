use std::ops::Deref;

use futures::executor::block_on;
use tokio::{io, sync::watch, task::JoinHandle};

use crate::{BarFrame, CavaHandle};

pub struct CavaWatcher {
    last_frame: watch::Receiver<BarFrame>,
    watcher_task: Option<JoinHandle<Result<(), crate::Error>>>,
}

impl CavaWatcher {
    pub fn spawn(mut handle: CavaHandle) -> Self {
        let (tx, rx) = watch::channel(BarFrame::default());
        let watcher_task = tokio::spawn(async move {
            while let Some(frame) = handle.next_frame().await? {
                match tx.send(frame.clone()) {
                    // The watcher has been dropped
                    Err(_) => break,
                    Ok(()) => {}
                };
            }

            Ok::<(), crate::Error>(())
        });

        Self {
            watcher_task: Some(watcher_task),
            last_frame: rx,
        }
    }

    pub fn latest_frame(&mut self) -> Result<BarFrame, crate::Error> {
        // check if the watcher handle is still running
        if let Some(task) = self.watcher_task.take_if(|t| t.is_finished()) {
            match block_on(task) {
                Ok(Ok(())) => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "cava process ended unexpectedly",
                    )
                    .into());
                }
                Err(_) => {
                    return Err(
                        io::Error::new(io::ErrorKind::BrokenPipe, "cava watcher panicked").into(),
                    );
                }
                Ok(Err(e)) => return Err(e),
            }
        }

        let frame = self.last_frame.borrow();
        Ok(frame.clone())
    }
}
