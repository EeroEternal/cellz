//! HTTP handlers and WebSocket/SSE streaming endpoints.

pub mod handlers;
pub mod ws;

pub use handlers::{AppState, EventsQuery};
