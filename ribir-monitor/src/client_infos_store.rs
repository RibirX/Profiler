use std::{
  collections::{HashMap, VecDeque},
  time::{Duration, Instant},
};

use ahash::HashSet;
use monitor_msg::{Fields, Meta, MonitorMsg};
use ribir_tracing::error;

/// It stored the summary info of a client application and all log from it.  All
/// logs had been preprocessed.
pub struct ClientInfoStore {
  /// The name of the client.
  client_name: String,
  /// The instance of client starts at, all other `Instant` are relative to
  /// this.
  client_start_at: Instant,
  /// All spans received from the client connect to the monitor
  spans: HashMap<Id, Span, ahash::RandomState>,
  /// All events received from the client connect to the monitor
  events: HashMap<Id, Event, ahash::RandomState>,
  /// All calc_scope received from the client connect to the monitor
  calc_scopes: HashMap<Id, CalcScope, ahash::RandomState>,
  /// All `Span`, `Event` and `CalcScope` received from the client connect to
  /// the monitor, it ordered by timestamp.
  timelines: VecDeque<Id>,
  call_stack: Vec<Id>,
  event_id_acc: u64,
  calc_span_id_acc: u64,
  /// The limit for closed spans to keep. When the count of spans in the store
  /// is over this number, the closed span will be dropped to reduce the number.
  span_limit: usize,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Id {
  Span(u64),
  CalcScope(u64),
  Event(u64),
}

struct Span {
  meta: Meta,
  fields: Fields,
  start_at: Duration,
  close_at: Option<Duration>,
  calc_scope: Option<Id>,
}

struct CalcScope {
  /// The parent `CalcScope` of this scope if have.
  parent_scope: Option<Id>,
  /// The `Span` this scope entered.
  host: Id,
  enter_at: Duration,
  exit_at: Option<Duration>,
}

struct Event {
  meta: Meta,
  calc_scope: Option<Id>,
  fields: Fields,
  time_stamp: Duration,
}

impl ClientInfoStore {
  pub fn record(&mut self, msg: MonitorMsg) {
    match msg {
      MonitorMsg::Event { meta, fields, time_stamp } => {
        let calc_scope = self.call_stack.last().copied();
        let id = self.new_event_id();
        self.timelines.push_back(id);
        self
          .events
          .insert(id, Event { meta, calc_scope, fields, time_stamp });
      }

      MonitorMsg::NewSpan { id, meta, fields, time_stamp } => {
        let id = Id::Span(id);
        self.timelines.push_back(id);
        let span = Span {
          meta,
          fields,
          start_at: time_stamp,
          close_at: None,
          calc_scope: self.call_stack.last().copied(),
        };
        self.spans.insert(id, span);
      }
      MonitorMsg::SpanUpdate { id, mut changes, .. } => {
        // monitor messages is a hot data stream, and may not have a full information.
        if let Some(span) = self.spans.get_mut(&Id::Span(id)) {
          span.fields.merge(&mut changes)
        }
      }
      MonitorMsg::EnterSpan { id, time_stamp } => {
        let host = Id::Span(id);
        if self.spans.get(&host).is_some() {
          let id = self.new_scope_id();
          self.timelines.push_back(id);
          let parent_scope = self.call_stack.last().copied();
          let calc_scope = CalcScope {
            parent_scope,
            host,
            enter_at: time_stamp,
            exit_at: None,
          };
          self.calc_scopes.insert(id, calc_scope);
          self.call_stack.push(id);
        }
      }
      MonitorMsg::ExitSpan { id, time_stamp } => {
        let current_scope = self
          .call_stack
          .pop()
          .and_then(|scope_id| self.calc_scopes.get_mut(&scope_id));

        if let Some(scope) = current_scope {
          if scope.host != Id::Span(id) {
            error!("Call stack record  error!");
          } else {
            scope.exit_at = Some(time_stamp);
          }
        }
      }
      MonitorMsg::CloseSpan { id, time_stamp } => {
        // monitor messages is a hot data stream, and may not have a full information.
        if let Some(span) = self.spans.get_mut(&Id::Span(id)) {
          if span.close_at.is_some() {
            error!("Send invalid message, twice close a span!");
          } else {
            span.close_at = Some(time_stamp)
          }
          // trigger reduce only when this is the most top span.
          if span.calc_scope.is_none() && self.spans.len() >= self.span_limit {
            self.try_reduce_closed_spans();
          }
        }
      }
      MonitorMsg::MonitorError(err) => {
        ribir_tracing::error!(err)
      }
    }
  }

  fn new_event_id(&mut self) -> Id {
    self.event_id_acc += 1;
    Id::Event(self.event_id_acc)
  }

  fn new_scope_id(&mut self) -> Id {
    self.calc_span_id_acc += 1;
    Id::CalcScope(self.calc_span_id_acc)
  }

  fn try_reduce_closed_spans(&mut self) {
    let mut removed = ahash::HashSet::default();
    self.timelines.retain(|id| {
      let should_drop = match id {
        id @ Id::Span(..) => {
          let Some(span) = self.spans.get(id) else { return false };
          removed.is_empty()
            || span
              .calc_scope
              .map_or(false, |calc| removed.contains(&calc))
        }
        id @ Id::CalcScope(_) => {
          let Some(scope) = self.calc_scopes.get(id) else { return false};
          let CalcScope { parent_scope, host, .. } = scope;
          removed.contains(host) || parent_scope.map_or(false, |s| removed.contains(&s))
        }
        id @ Id::Event(_) => {
          let Some(e) = self.events.get(id) else { return false};
          e.calc_scope.map_or(false, |id| removed.contains(&id))
        }
      };
      if should_drop {
        removed.insert(*id);
      }
      !should_drop
    });
    self.spans.retain(|id, _| !removed.contains(&id));
    self.calc_scopes.retain(|id, _| !removed.contains(&id));
    self.events.retain(|id, _| !removed.contains(&id));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn store() -> ClientInfoStore {
    ClientInfoStore {
      client_name: "test client".into(),
      client_start_at: Instant::now(),
      spans: <_>::default(),
      events: <_>::default(),
      calc_scopes: <_>::default(),
      timelines: <_>::default(),
      call_stack: <_>::default(),
      event_id_acc: 0,
      calc_span_id_acc: 0,
      span_limit: 1024,
    }
  }

  fn mock_meta() -> Meta {
    Meta {
      name: "test meta".into(),
      target: "monitor test".into(),
      level: "error".into(),
      module_path: None,
      file: None,
      line: None,
    }
  }

  #[test]
  fn smoke() {
    let mut store = store();
    store.span_limit = 2;

    store.record(MonitorMsg::NewSpan {
      id: 0,
      meta: mock_meta(),
      fields: <_>::default(),
      time_stamp: Duration::ZERO,
    });
    store.record(MonitorMsg::EnterSpan { id: 0, time_stamp: Duration::ZERO });

    {
      // new span
      store.record(MonitorMsg::NewSpan {
        id: 1,
        meta: mock_meta(),
        fields: <_>::default(),
        time_stamp: Duration::ZERO,
      });
      store.record(MonitorMsg::EnterSpan { id: 1, time_stamp: Duration::ZERO });

      store.record(MonitorMsg::Event {
        meta: mock_meta(),
        fields: <_>::default(),
        time_stamp: Duration::ZERO,
      });

      store.record(MonitorMsg::ExitSpan { id: 1, time_stamp: Duration::ZERO });
      store.record(MonitorMsg::CloseSpan { id: 1, time_stamp: Duration::ZERO });
    }

    store.record(MonitorMsg::ExitSpan { id: 0, time_stamp: Duration::ZERO });
    assert_eq!(store.spans.len(), 2);
    assert_eq!(store.calc_scopes.len(), 2);
    assert_eq!(store.events.len(), 1);

    assert!(store.spans.get(&Id::Span(1)).unwrap().close_at.is_some());
    assert!(store.spans.get(&Id::Span(0)).unwrap().close_at.is_none());
    assert!(
      store
        .calc_scopes
        .get(&Id::CalcScope(1))
        .unwrap()
        .exit_at
        .is_some()
    );

    store.record(MonitorMsg::CloseSpan { id: 0, time_stamp: Duration::ZERO });

    assert_eq!(store.spans.len(), 0);
    assert_eq!(store.calc_scopes.len(), 0);
    assert_eq!(store.events.len(), 0);
    assert_eq!(store.timelines.len(), 0);
  }
}
