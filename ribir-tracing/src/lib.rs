#[macro_export]
macro_rules! error {
  (*$($t:tt)*) => {
    #[cfg(feature= "error")]
    tracing::error!($($t)*)
  };
}

#[macro_export]
macro_rules! error_span {
  (*$($t:tt)*) => {
    #[cfg(feature= "error")]
    tracing::error_span!($($t)*)
  };
}

#[macro_export]
macro_rules! warn {
  (*$($t:tt)*) => {
    #[cfg(feature= "warn")]
    tracing::warn!($($t)*)
  };
}

#[macro_export]
macro_rules! warn_span {
  (*$($t:tt)*) => {
    #[cfg(feature= "warn")]
    tracing::warn_span!($($t)*)
  };
}

#[macro_export]
macro_rules! info_span {
  (*$($t:tt)*) => {
    #[cfg(feature= "info")]
    tracing::info_span!($($t)*)
  };
}

#[macro_export]
macro_rules! debug {
  (*$($t:tt)*) => {
    #[cfg(feature= "debug")]
    tracing::debug!($($t)*)
  };
}

#[macro_export]
macro_rules! debug_span {
  (*$($t:tt)*) => {
    #[cfg(feature= "debug")]
    tracing::debug_span!($($t)*)
  };
}

#[macro_export]
macro_rules! trace {
  (*$($t:tt)*) => {
    #[cfg(feature= "trace")]
    tracing::trace!($($t)*)
  };
}

#[macro_export]
macro_rules! trace_span {
  (*$($t:tt)*) => {
    #[cfg(feature= "trace")]
    tracing::trace_span!($($t)*)
  };
}

#[cfg(feature = "info")]
pub use tracing::instrument;

#[cfg(not(feature = "info"))]
pub use mock_proc_macros::instrument;
