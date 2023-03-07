use crossbeam_channel::{unbounded, Receiver, Sender};
use monitor_msg::MonitorMsg;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const LOG_COLLECT_INTERVAL_MS: u64 = 10;
type LogConsumeFn = dyn for<'a> FnMut(&'a [MonitorMsg]) + Send;

#[derive(Clone)]
pub struct LogConsumeHandle {
  closed: Arc<Mutex<bool>>,
}

impl LogConsumeHandle {
  pub fn is_closed(&self) -> bool { *self.closed.lock().unwrap() }

  pub fn close(&self) { *self.closed.lock().unwrap() = true; }
}
#[derive(Default)]
pub struct RLogConsumers {
  consumers: Arc<Mutex<Vec<(LogConsumeHandle, Box<LogConsumeFn>)>>>,
}

impl Clone for RLogConsumers {
  fn clone(&self) -> Self { Self { consumers: self.consumers.clone() } }
}

impl RLogConsumers {
  pub fn add(&mut self, call: Box<LogConsumeFn>) -> LogConsumeHandle {
    let handle = LogConsumeHandle { closed: Arc::new(Mutex::new(false)) };
    {
      let mut consumers = self.consumers.lock().unwrap();
      consumers.push((handle.clone(), call));
    }
    handle
  }

  fn consume(&mut self, vals: &[MonitorMsg]) {
    if let Ok(mut consumers) = self.consumers.lock() {
      consumers.iter_mut().for_each(|(h, call_fn)| {
        if !h.is_closed() {
          (call_fn)(vals)
        }
      });
      consumers
        .iter()
        .filter(|(h, _)| h.is_closed())
        .for_each(drop);
    }
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
    consumers: Arc::new(Mutex::new(vec![])),
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
