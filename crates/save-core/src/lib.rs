//! Lossless, game-scoped Odin binary documents. No filesystem, Unity or Tauri dependency.
pub mod drops;
mod general;
pub mod inventory;
mod service;
mod wire;
mod workspace;

pub use workspace::{Command, DocumentSummary, Operation, Workspace};

pub use service::{EditRequest, NodeView, Page, Request, Response, Service, Summary};
pub use wire::{Document, Error, Limits};
