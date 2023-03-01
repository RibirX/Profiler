use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
  /// The name of the span described by this metadata.
  pub name: &'static str,
  /// The part of the system that the span that this metadata describes
  /// occurred in.
  pub target: String,

  /// The level of verbosity of the described span.
  pub level: &'static str,

  /// The name of the Rust module where the span occurred, or `None` if this
  /// could not be determined.
  pub module_path: Option<String>,

  /// The name of the source code file where the span occurred, or `None` if
  /// this could not be determined.
  pub file: Option<String>,

  /// The line number in the source code file where the span occurred, or
  /// `None` if this could not be determined.
  pub line: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldValue {
  field: &'static str,
  data: Box<[u8]>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub enum MonitorMsg {
  Event {
    meta: Meta,
    parent_span: Option<u64>,
    fields: Box<[FieldValue]>,
    time_stamp: Duration,
  },
  NewSpan {
    id: u64,
    meta: Meta,
    parent_span: Option<u64>,
    fields: Box<[FieldValue]>,
    time_stamp: Duration,
  },
  SpanUpdate {
    id: u64,
    meta: Meta,
    updated: Box<[FieldValue]>,
    time_stamp: Duration,
  },
  EnterSpan {
    id: u64,
    /// A time stamp relative to the MonitorLayer start.
    time_stamp: Duration,
  },
  ExitSpan {
    id: u64,
    /// A time stamp relative to the MonitorLayer start.
    time_stamp: Duration,
  },
  CloseSpan {
    id: u64,
    /// A time stamp relative to the MonitorLayer start.
    time_stamp: Duration,
  },
  MonitorError(String),
}

impl FieldValue {
  pub fn new(field: &'static str, data: Box<[u8]>) -> Self {
    Self { field, data }
  }
}
