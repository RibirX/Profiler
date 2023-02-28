use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
  /// The part of the system that the span that this metadata describes
  /// occurred in.
  target: String,

  /// The level of verbosity of the described span.
  level: u8,

  /// The name of the Rust module where the span occurred, or `None` if this
  /// could not be determined.
  module_path: Option<String>,

  /// The name of the source code file where the span occurred, or `None` if
  /// this could not be determined.
  file: Option<String>,

  /// The line number in the source code file where the span occurred, or
  /// `None` if this could not be determined.
  line: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MonitorItem {
  Event { meta: Meta },
  Span { meta: Meta, name: &'static str },
  MonitorError(String),
}
