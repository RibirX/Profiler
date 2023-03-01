use std::time::{Duration, Instant};

use monitor_msg::{FieldValue, Meta, MonitorMsg};
use tracing::{field::*, span::*, Event, Metadata, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

pub struct MonitorLayer {
  start_time: Instant,
}

impl<S> Layer<S> for MonitorLayer
where
  S: Subscriber,
{
  fn on_event(&self, event: &Event, _: Context<S>) {
    let time_stamp = self.time_stamp();
    let meta_data = event.metadata();
    let meta = record_meta_data(meta_data);
    let mut fields = FieldsVisitor {
      fields: vec![],
      error_writer: Box::new(|_| {}),
    };
    event.record(&mut fields);

    let event = MonitorMsg::Event {
      meta,
      parent_span: event.parent().map(Id::into_u64),
      fields: fields.data(),
      time_stamp,
    };
  }

  fn on_new_span(&self, attrs: &Attributes, id: &Id, _: Context<S>) {
    let time_stamp = self.time_stamp();
    let meta_data = attrs.metadata();
    let meta = record_meta_data(meta_data);
    let mut fields = FieldsVisitor {
      fields: vec![],
      error_writer: Box::new(|_| {}),
    };
    attrs.record(&mut fields);

    let span = MonitorMsg::NewSpan {
      id: id.into_u64(),
      meta,
      parent_span: attrs.parent().map(Id::into_u64),
      fields: fields.data(),
      time_stamp,
    };
  }

  fn on_record(&self, _span: &Id, _values: &Record<'_>, _ctx: Context<'_, S>) {}

  fn on_enter(&self, _id: &Id, _ctx: Context<'_, S>) {
    println!("enter {_id:?}");
  }

  fn on_exit(&self, _id: &Id, _ctx: Context<'_, S>) {
    println!("exit {_id:?}");
  }

  fn on_close(&self, _id: Id, _ctx: Context<'_, S>) {
    println!("close {_id:?}");
  }
}

fn record_meta_data(data: &Metadata) -> Meta {
  Meta {
    name: data.name(),
    target: data.target().to_string(),
    level: data.level().as_str(),
    module_path: data.module_path().map(ToString::to_string),
    file: data.file().map(ToString::to_string),
    line: data.line(),
  }
}

struct FieldsVisitor {
  fields: Vec<FieldValue>,
  error_writer: Box<dyn FnMut(MonitorMsg)>,
}

impl Visit for FieldsVisitor {
  fn record_value(&mut self, field: &Field, value: valuable::Value<'_>) {
    let value = valuable_serde::Serializable::new(value);
    let data = bincode::serialize(&value);

    match data {
      Ok(data) => {
        let fv = FieldValue::new(field.name(), data.into_boxed_slice());
        self.fields.push(fv);
      }
      Err(e) => {
        (self.error_writer)(MonitorMsg::MonitorError(e.to_string()));
      }
    }
  }

  #[inline]
  fn record_f64(&mut self, field: &Field, value: f64) {
    self.record_value(field, value.into())
  }

  #[inline]
  fn record_i64(&mut self, field: &Field, value: i64) {
    self.record_value(field, value.into())
  }

  #[inline]
  fn record_u64(&mut self, field: &Field, value: u64) {
    self.record_value(field, value.into())
  }

  #[inline]
  fn record_i128(&mut self, field: &Field, value: i128) {
    self.record_value(field, value.into())
  }

  #[inline]
  fn record_u128(&mut self, field: &Field, value: u128) {
    self.record_value(field, value.into())
  }

  #[inline]
  fn record_bool(&mut self, field: &Field, value: bool) {
    self.record_value(field, value.into())
  }

  #[inline]
  fn record_str(&mut self, field: &Field, value: &str) {
    self.record_value(field, value.into())
  }

  #[inline]
  fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
    self.record_value(field, valuable::Value::String(&format!("{value:?}")));
  }
}

impl FieldsVisitor {
  fn data(self) -> Box<[FieldValue]> {
    self.fields.into_boxed_slice()
  }
}

impl MonitorLayer {
  #[inline]
  fn time_stamp(&self) -> Duration {
    Instant::now().duration_since(self.start_time)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::*;
  use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

  #[test]
  fn layer_smoke() {
    tracing_subscriber::registry()
      .with(MonitorLayer { start_time: Instant::now() })
      .with(tracing_subscriber::fmt::layer().compact())
      .init();

    use valuable::Valuable;
    error!(snapshot = vec!["xxx", "yyy"].as_value(), name = "xxx");
    panic!("xxx");
  }
}
