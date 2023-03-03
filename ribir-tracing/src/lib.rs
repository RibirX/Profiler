mod layer;
mod log_writer;

mod wslog;
#[macro_export]
macro_rules! error {
  ($($t:tt)*) => {{
    #[cfg(feature= "error")]
    tracing::error!($($t)*)
  }};
}

#[macro_export]
macro_rules! error_span {
  ($($t:tt)*) => {{
    #[cfg(feature= "error")]
    tracing::error_span!($($t)*)
  }};
}

#[macro_export]
macro_rules! warn {
  ($($t:tt)*) => {{
    #[cfg(feature= "warn")]
    tracing::warn!($($t)*)
  }};
}

#[macro_export]
macro_rules! warn_span {
  ($($t:tt)*) => {{
    #[cfg(feature= "warn")]
    tracing::warn_span!($($t)*)
  }};
}

#[macro_export]
macro_rules! info {
  ($($t:tt)*) => {{
    #[cfg(feature= "info")]
    tracing::info!($($t)*)
  }};
}

#[macro_export]
macro_rules! info_span {
  ($($t:tt)*) => {{
    #[cfg(feature= "info")]
    tracing::info_span!($($t)*)
  }};
}

#[macro_export]
macro_rules! debug {
  ($($t:tt)*) => {{
    #[cfg(feature= "debug")]
    tracing::debug!($($t)*)
  }};
}

#[macro_export]
macro_rules! debug_span {
  ($($t:tt)*) => {{
    #[cfg(feature= "debug")]
    tracing::debug_span!($($t)*)
  }};
}

#[macro_export]
macro_rules! trace {
  ($($t:tt)*) => {{
    #[cfg(feature= "trace")]
    tracing::trace!($($t)*)
  }};
}

#[macro_export]
macro_rules! trace_span {
  ($($t:tt)*) => {{
    #[cfg(feature= "trace")]
    tracing::trace_span!($($t)*)
  }};
}

use layer::MonitorLayer;
#[cfg(not(feature = "info"))]
pub use mock_proc_macros::instrument;
#[cfg(feature = "info")]
pub use tracing::instrument;

pub struct WSHandle;

/// This function returns a tuple containing two parts:
///   - a new monitor layer of `tracing_subscriber`, use it to init a tracing subscriber so the monitor can capture the spans and events.
///   - a handle of WebSocket to control the connection from your application to the monitor. You can use it to config the connection. The connection will not be created by default, you should call the `connect` method to create the connection by yourself.
/// # Example
///
/// ```rust
/// let (subscriber, ws_handle) = new_monitor_subscriber();
/// tracing_subscriber::registry().with(subscriber).init();
/// // In practice, maybe you want connect in an async way, use `connect`.
/// ws_handle.block_connect();
/// ```
///
pub fn new_monitor_subscriber() -> (MonitorLayer, WSHandle) {
  todo!();
}
