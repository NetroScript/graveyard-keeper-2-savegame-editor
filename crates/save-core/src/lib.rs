//! Lossless, game-scoped Odin binary documents. No filesystem, Unity or Tauri dependency.
mod service;
mod wire;

pub use service::{EditRequest, NodeView, Page, Request, Response, Service, Summary};
pub use wire::{Document, Error, Limits};
