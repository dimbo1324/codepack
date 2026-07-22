//! Progress and log channel abstraction (BLUEPRINT §C.4).
//!
//! The direct analogue of the legacy `Queue[str]`: pipeline steps in later stages send
//! `ProgressEvent`s from a worker thread, the UI (or CLI) drains them without blocking
//! the worker. This module only defines the shape of that channel; the producer arrives
//! with the S9 engine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressEvent {
    StepStarted {
        step: String,
    },
    StepProgress {
        step: String,
        current: u64,
        total: Option<u64>,
    },
    StepFinished {
        step: String,
    },
    Log(LogEvent),
}

pub type ProgressSender = crossbeam_channel::Sender<ProgressEvent>;
pub type ProgressReceiver = crossbeam_channel::Receiver<ProgressEvent>;

/// An unbounded channel: a worker must never block on progress reporting.
pub fn progress_channel() -> (ProgressSender, ProgressReceiver) {
    crossbeam_channel::unbounded()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_arrive_in_order() {
        let (tx, rx) = progress_channel();
        tx.send(ProgressEvent::StepStarted {
            step: "scan".to_string(),
        })
        .unwrap();
        tx.send(ProgressEvent::StepProgress {
            step: "scan".to_string(),
            current: 1,
            total: Some(10),
        })
        .unwrap();
        tx.send(ProgressEvent::StepFinished {
            step: "scan".to_string(),
        })
        .unwrap();
        drop(tx);

        let events: Vec<_> = rx.iter().collect();
        assert_eq!(events.len(), 3);
        assert_eq!(
            events[0],
            ProgressEvent::StepStarted {
                step: "scan".to_string()
            }
        );
        assert_eq!(
            events[2],
            ProgressEvent::StepFinished {
                step: "scan".to_string()
            }
        );
    }

    #[test]
    fn log_event_wraps_level_and_message() {
        let (tx, rx) = progress_channel();
        tx.send(ProgressEvent::Log(LogEvent {
            level: LogLevel::Warn,
            message: "low disk space".to_string(),
        }))
        .unwrap();
        drop(tx);

        match rx.recv().unwrap() {
            ProgressEvent::Log(event) => {
                assert_eq!(event.level, LogLevel::Warn);
                assert_eq!(event.message, "low disk space");
            }
            other => panic!("expected a Log event, got {other:?}"),
        }
    }
}
