use serde::Serialize;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const LOG_COLLECT_INTERVAL_MS: u64 = 10;
type LogConsumeFn<T> = dyn for<'a> FnMut(&'a [T]) + Send;

pub struct RLogWriter<T> {
  sender: Sender<T>,
}

impl<T: Send> RLogWriter<T> {
  pub fn write(&mut self, data: T) { self.sender.send(data).unwrap() }
}

impl<T> Clone for RLogWriter<T> {
  fn clone(&self) -> Self { Self { sender: self.sender.clone() } }
}

#[derive(Clone)]
pub struct LogConsumeHandle {
  closed: Arc<Mutex<bool>>,
}

impl LogConsumeHandle {
  pub fn is_closed(&self) -> bool { *self.closed.lock().unwrap() }

  pub fn close(&self) { *self.closed.lock().unwrap() = true; }
}
#[derive(Default)]
pub struct RLogConsumers<T> {
  consumers: Arc<Mutex<Vec<(LogConsumeHandle, Box<LogConsumeFn<T>>)>>>,
}

impl<T> Clone for RLogConsumers<T> {
  fn clone(&self) -> Self { Self { consumers: self.consumers.clone() } }
}

impl<T: Serialize> RLogConsumers<T> {
  pub fn add(&mut self, call: Box<LogConsumeFn<T>>) -> LogConsumeHandle {
    let handle = LogConsumeHandle { closed: Arc::new(Mutex::new(false)) };
    {
      let mut consumers = self.consumers.lock().unwrap();
      consumers.push((handle.clone(), call));
    }
    handle
  }

  fn consume(&mut self, vals: &[T]) {
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

pub fn new_log_writer<T: 'static + Send + Serialize>() -> (RLogWriter<T>, RLogConsumers<T>) {
  fn recv<T: Serialize>(mut consumers: RLogConsumers<T>, rx: Receiver<T>) {
    loop {
      let vals: Vec<_> = rx.try_iter().collect();
      consumers.consume(&vals);
      thread::sleep(Duration::from_millis(LOG_COLLECT_INTERVAL_MS));
    }
  }

  let (sx, rx) = mpsc::channel();
  let writer = RLogWriter { sender: sx };
  let consumers = RLogConsumers {
    consumers: Arc::new(Mutex::new(vec![])),
  };
  let consumers2 = consumers.clone();
  thread::spawn(move || recv(consumers2, rx));
  (writer, consumers)
}

#[cfg(test)]
mod test {
  use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
  };

  use crate::log_writer::new_log_writer;
  #[test]
  fn log_sender() {
    let logs = Arc::new(Mutex::new(vec![]));
    let consume = |logs: Arc<Mutex<Vec<i32>>>| {
      move |vals: &[i32]| {
        if let Ok(mut logs) = logs.lock() {
          for val in vals {
            logs.push(*val);
          }
        }
      }
    };
    let (logger, mut consumers) = new_log_writer();
    consumers.add(Box::new(consume(logs.clone())));
    consumers.add(Box::new(consume(logs.clone())));
    logger.sender.send(0).unwrap();

    thread::sleep(Duration::from_millis(20));
    assert!(logs.lock().unwrap().len() == 2);
  }
}
