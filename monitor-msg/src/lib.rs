use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap, time::Duration};
use tracing_core::{field::Visit, Field};

#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
  /// The name of the span described by this metadata.
  pub name: Cow<'static, str>,
  /// The part of the system that the span that this metadata describes
  /// occurred in.
  pub target: String,

  /// The level of verbosity of the described span.
  pub level: Cow<'static, str>,

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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Fields(BTreeMap<Cow<'static, str>, FieldValue>);

#[derive(Debug, Serialize, Deserialize)]
pub enum FieldValue {
  Bool(bool),
  F64(f64),
  I64(i64),
  U64(u64),
  I128(i128),
  U128(u128),
  String(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "Meta: Deserialize<'de>, FieldValue: Deserialize<'de>"))]
pub enum MonitorMsg {
  Event {
    meta: Meta,
    fields: Fields,
    time_stamp: Duration,
  },
  NewSpan {
    id: u64,
    meta: Meta,
    fields: Fields,
    time_stamp: Duration,
  },
  SpanUpdate {
    id: u64,
    changes: Fields,
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

impl Visit for Fields {
  fn record_f64(&mut self, field: &Field, value: f64) {
    self.0.insert(field.name().into(), FieldValue::F64(value));
  }

  fn record_i64(&mut self, field: &Field, value: i64) {
    self.0.insert(field.name().into(), FieldValue::I64(value));
  }

  fn record_u64(&mut self, field: &Field, value: u64) {
    self.0.insert(field.name().into(), FieldValue::U64(value));
  }

  fn record_i128(&mut self, field: &Field, value: i128) {
    self.0.insert(field.name().into(), FieldValue::I128(value));
  }

  fn record_u128(&mut self, field: &Field, value: u128) {
    self.0.insert(field.name().into(), FieldValue::U128(value));
  }

  fn record_bool(&mut self, field: &Field, value: bool) {
    self.0.insert(field.name().into(), FieldValue::Bool(value));
  }

  fn record_str(&mut self, field: &Field, value: &str) {
    self
      .0
      .insert(field.name().into(), FieldValue::String(value.to_string()));
  }

  fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
    self.0.insert(
      field.name().into(),
      FieldValue::String(format!("{value:?}")),
    );
  }
}

impl Fields {
  pub fn merge(&mut self, other: &mut Fields) { self.0.append(&mut other.0); }
}
