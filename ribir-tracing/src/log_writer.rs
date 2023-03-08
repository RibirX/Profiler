use crossbeam_channel::{unbounded, Receiver, Sender};
use crossbeam_skiplist::SkipSet;
use crossbeam_utils::atomic::AtomicCell;
use monitor_msg::MonitorMsg;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const LOG_COLLECT_INTERVAL_MS: u64 = 10;
type LogConsumeFn = dyn for<'a> FnMut(&'a [MonitorMsg]) + Send;

#[derive(Clone)]
pub struct LogConsumeHandle {
  closed: Arc<AtomicBool>,
}

impl LogConsumeHandle {
  pub fn is_closed(&self) -> bool { self.closed.load(Ordering::Relaxed) }

  pub fn close(&self) { self.closed.swap(true, Ordering::Relaxed); }
}

struct LogConsumer(LogConsumeHandle, AtomicCell<Box<LogConsumeFn>>);

impl PartialOrd for LogConsumer {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

impl PartialEq for LogConsumer {
  fn eq(&self, other: &Self) -> bool { self.cmp(other).is_eq() }
}

impl Eq for LogConsumer {}

impl Ord for LogConsumer {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    let ptr: *const LogConsumeFn = self.1.as_ptr();
    let ptr2: *const LogConsumeFn = other.1.as_ptr();
    ptr.cmp(&ptr2)
  }
}

impl LogConsumer {
  #[inline]
  fn is_closed(&self) -> bool { self.0.is_closed() }

  #[inline]
  fn consume(&self, vals: &[MonitorMsg]) {
    unsafe {
      (*self.1.as_ptr())(vals);
    }
  }
}


#[derive(Default)]
pub struct RLogConsumers {
  consumers: Arc<SkipSet<Box<LogConsumer>>>,
}

impl Clone for RLogConsumers {
  fn clone(&self) -> Self { Self { consumers: self.consumers.clone() } }
}

impl RLogConsumers {
  pub fn add(&mut self, call: Box<LogConsumeFn>) -> LogConsumeHandle {
    let handle = LogConsumeHandle {
      closed: Arc::new(AtomicBool::new(false)),
    };
    self
      .consumers
      .insert(Box::new(LogConsumer(handle.clone(), AtomicCell::new(call))));
    handle
  }

  fn consume(&mut self, vals: &[MonitorMsg]) {
    self.consumers.iter().for_each(|consumer| {
      if !consumer.is_closed() {
        consumer.consume(vals)
      }
    });
    self
      .consumers
      .iter()
      .filter(|consumer| consumer.is_closed())
      .for_each(drop);
  }
}

pub(crate) fn new_log_writer() -> (Sender<MonitorMsg>, RLogConsumers) {
  fn recv(mut consumers: RLogConsumers, rx: Receiver<MonitorMsg>) {
    loop {
      let vals: Vec<_> = rx.try_iter().collect();
      if !vals.is_empty() {
        consumers.consume(&vals);
      }
      thread::sleep(Duration::from_millis(LOG_COLLECT_INTERVAL_MS));
    }
  }

  let (sx, rx) = unbounded();
  let consumers = RLogConsumers {
    consumers: Arc::new(SkipSet::default()),
  };
  let consumers2 = consumers.clone();
  thread::spawn(move || recv(consumers2, rx));
  (sx, consumers)
}

#[cfg(test)]
mod test {
  use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
  };

  use monitor_msg::MonitorMsg;

  use crate::log_writer::new_log_writer;
  #[test]
  fn log_sender() {
    let logs = Arc::new(Mutex::new(vec![]));
    let consume = |logs: Arc<Mutex<Vec<usize>>>| {
      move |vals: &[MonitorMsg]| {
        if let Ok(mut logs) = logs.lock() {
          logs.push(vals.len());
        }
      }
    };
    let (sender, mut consumers) = new_log_writer();
    consumers.add(Box::new(consume(logs.clone())));
    sender
      .send(MonitorMsg::MonitorError("test".to_string()))
      .unwrap();
    thread::sleep(Duration::from_millis(20));
    assert!(logs.lock().unwrap().len() == 1);

    consumers.add(Box::new(consume(logs.clone())));
    sender
      .send(MonitorMsg::MonitorError("test".to_string()))
      .unwrap();
    thread::sleep(Duration::from_millis(20));
    assert!(logs.lock().unwrap().len() == 3);
  }
}
