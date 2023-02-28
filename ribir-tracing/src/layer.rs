use std::error::Error;

use crate::monitor_item::MonitorItem;
use serde::{Deserialize, Serialize};
use tracing::{field::*, Event, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

pub struct MonitorLayer;

impl<S> Layer<S> for MonitorLayer
where
  S: Subscriber,
{
  fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
    let meta_data = event.metadata();
    let mut fields = Fields {
      fields: vec![],
      error_writer: Box::new(|_| {}),
    };
    event.record(&mut fields);
  }
}

struct Fields {
  fields: Vec<FieldValue>,
  error_writer: Box<dyn FnMut(MonitorItem)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FieldValue {
  field: &'static str,
  data: Box<[u8]>,
}

impl Visit for Fields {
  fn record_value(&mut self, field: &Field, value: valuable::Value<'_>) {
    let value = valuable_serde::Serializable::new(value);
    let data = bincode::serialize(&value);

    match data {
      Ok(data) => {
        let fv = FieldValue {
          field: field.name(),
          data: data.into_boxed_slice(),
        };
        self.fields.push(fv);
      }
      Err(e) => {
        (self.error_writer)(MonitorItem::MonitorError(e.to_string()));
      }
    }
  }

  fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
    self.record_value(field, valuable::Value::String(&format!("{value:?}")));
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
      .with(MonitorLayer {})
      .with(tracing_subscriber::fmt::layer().compact())
      .init();

    use valuable::Valuable;
    error!(snapshot = vec!["xxx", "yyy"].as_value(), "xxx");
    panic!("xxx");
  }
}
