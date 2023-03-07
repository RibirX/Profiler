use crossbeam_channel::Sender;
use monitor_msg::{FieldValue, Fields, Meta, MonitorMsg};
use std::{
  borrow::Cow,
  time::{Duration, Instant},
};
use tracing::{field::*, span::*, Event, Metadata, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

pub struct MonitorLayer {
  /// The monitor starts at.
  start_time: Instant,
  msg_sender: Sender<MonitorMsg>,
}

macro_rules! base_visit {
  ($this: ident, $fields: ident) => {{
    let time_stamp = $this.time_stamp();
    let meta_data = $fields.metadata();
    let meta = record_meta_data(meta_data);
    let mut visitor = FieldsVisitor {
      fields: vec![],
      error_writer: Box::new(|_| {}),
    };
    $fields.record(&mut visitor);

    (time_stamp, meta, visitor.data())
  }};
}

impl<S> Layer<S> for MonitorLayer
where
  S: Subscriber,
{
  fn on_event(&self, event: &Event, _: Context<S>) {
    let time_stamp = self.time_stamp();
    let meta = record_meta_data(event.metadata());
    let mut fields = Fields::default();
    event.record(&mut fields);

    let event = MonitorMsg::Event { meta, fields, time_stamp };
    self.send_msg(event);
  }

  fn on_new_span(&self, attrs: &Attributes, id: &Id, _: Context<S>) {
    let time_stamp = self.time_stamp();
    let meta = record_meta_data(attrs.metadata());
    let mut fields = Fields::default();
    attrs.record(&mut fields);

    let span = MonitorMsg::NewSpan {
      id: id.into_u64(),
      meta,
      fields,
      time_stamp,
    };
    self.send_msg(span);
  }

  fn on_record(&self, span: &Id, values: &Record<'_>, _: Context<'_, S>) {
    let time_stamp = self.time_stamp();
    let mut fields = Fields::default();
    values.record(&mut fields);

    let id = span.into_u64();
    let record = MonitorMsg::SpanUpdate { id, changes: fields, time_stamp };
    self.send_msg(record);
  }

  fn on_enter(&self, id: &Id, _: Context<S>) {
    let enter = MonitorMsg::EnterSpan {
      id: id.into_u64(),
      time_stamp: self.time_stamp(),
    };
    self.send_msg(enter);
  }

  fn on_exit(&self, id: &Id, _: Context<S>) {
    let exit = MonitorMsg::ExitSpan {
      id: id.into_u64(),
      time_stamp: self.time_stamp(),
    };

    self.send_msg(exit);
  }

  fn on_close(&self, id: Id, _: Context<S>) {
    let close = MonitorMsg::CloseSpan {
      id: id.into_u64(),
      time_stamp: self.time_stamp(),
    };
    self.send_msg(close);
  }
}

fn record_meta_data(data: &Metadata) -> Meta {
  Meta {
    name: Cow::Borrowed(data.name()),
    target: data.target().to_string(),
    level: Cow::Borrowed(data.level().as_str()),
    module_path: data.module_path().map(ToString::to_string),
    file: data.file().map(ToString::to_string),
    line: data.line(),
  }
}

impl MonitorLayer {
  pub fn new(sender: Sender<MonitorMsg>) -> Self {
    Self {
      start_time: Instant::now(),
      msg_sender: sender,
    }
  }

  #[inline]
  fn time_stamp(&self) -> Duration { Instant::now().duration_since(self.start_time) }

  fn send_msg(&self, msg: MonitorMsg) {
    if self.msg_sender.send(msg).is_err() {
      eprintln!("Send monitor message failed!");
    }
  }
}

#[cfg(test)]
mod tests {

  use std::borrow::Cow;

  use super::*;
  use crate::*;
  use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

  #[test]
  fn layer_smoke() {
    let (tx, rx) = crossbeam_channel::unbounded();
    tracing_subscriber::registry()
      .with(MonitorLayer::new(tx))
      // .with(tracing_subscriber::fmt::layer().compact())
      .init();

    {
      let out = error_span!("outside span");
      {
        let _enter = out.enter();
        let _inner = error_span!("inner span");
        {
          error!("error event");
        }
      }
    }

    let mut msgs = vec![];
    while let Ok(msg) = rx.try_recv() {
      msgs.push(msg)
    }
    assert_eq!(msgs.len(), 7);
    let MonitorMsg::NewSpan {
      id: outside_id,
      meta: Meta { name: Cow::Borrowed("outside span"), .. },
      ..
    } = &msgs[0] else {
      panic!();
    };

    assert!(matches!(&msgs[1], MonitorMsg::EnterSpan { .. }));

    assert!(matches!(
      &msgs[2],
      MonitorMsg::NewSpan {
        meta: Meta {
          name: Cow::Borrowed("inner span"),
          ..
        },
        ..
      }
    ));

    assert!(matches!(&msgs[4], MonitorMsg::CloseSpan { .. }));
    assert!(matches!(&msgs[5], MonitorMsg::ExitSpan { .. }));
    assert!(matches!(&msgs[6], MonitorMsg::CloseSpan { .. }));
  }
}
