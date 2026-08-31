//! This crate provides [CavaReader] to parse the output from cava in raw mode.
//! Also provides [CavaHandle] and [CavaConfig] to easily setup a configurable cava child process
//! for all your audio visualizing needs!

use std::process::{ExitStatus, Stdio};

use tempfile::NamedTempFile;
use tokio::{
    io::AsyncReadExt,
    process::{Child, ChildStdout},
};

use crate::reader::CavaReader;

pub use crate::config::CavaConfig;
pub use crate::reader::{BarFrame, CavaOutputFormat};
pub use ini;

pub mod config;
pub mod reader;
#[cfg(feature = "watcher")]
pub mod watcher;

/// A handle to a running cava process
pub struct CavaHandle {
    reader: CavaReader<ChildStdout>,
    process: Child,
    _config_file: NamedTempFile,
}

impl CavaHandle {
    /// Creates a new [CavaHandle] and runs the cava process
    pub fn new(config: CavaConfig) -> tokio::io::Result<Self> {
        let mut config_file = NamedTempFile::new()?;

        config.write_to(&mut config_file)?;

        let mut cava = tokio::process::Command::new("cava")
            .args([
                "-p",
                config_file
                    .path()
                    .to_str()
                    .expect("tempfile has a valid path"),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let cava_stdout = cava.stdout.take().unwrap();

        let reader = CavaReader::new(config.output_format, config.num_bars, cava_stdout);

        Ok(Self {
            reader,
            process: cava,
            _config_file: config_file,
        })
    }

    pub async fn next_frame(&mut self) -> Result<Option<&BarFrame>, Error> {
        let result = self.reader.next_frame().await;
        if let Ok(None) = result {
            let exit_status = self.process.wait().await?;
            if exit_status.code().is_some_and(|code| code == 0) {
                return Ok(None);
            }

            let mut stderr_str = String::new();
            if let Some(ref mut stderr) = self.process.stderr {
                let result = stderr.read_to_string(&mut stderr_str).await;
                if let Err(err) = result {
                    stderr_str = format!("failed to read stderr: {}", err)
                }
            }

            Err(Error::CavaError {
                exit_status,
                stderr: stderr_str,
            })
        } else {
            result.map_err(Error::ReaderError)
        }
    }

    /// Consumes `self` and returns a stream over Owned [BarFrame], cloning each frame.
    /// This makes it slightly more expensive then [CavaHandle::next_frame]
    pub fn into_stream(mut self) -> impl futures::Stream<Item = Result<BarFrame, Error>> {
        async_stream::try_stream! {
            while let Some(bars) = self.next_frame().await? {
                yield bars.clone();
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    IoError(#[from] tokio::io::Error),
    #[error("failed to read cava input: {0}")]
    ReaderError(tokio::io::Error),
    #[error("cava exited with exit status {exit_status}. cava stderr:\n{stderr}")]
    CavaError {
        exit_status: ExitStatus,
        stderr: String,
    },
}
