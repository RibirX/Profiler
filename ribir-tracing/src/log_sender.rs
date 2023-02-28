use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const LOG_COLLECT_INTERVAL_MS: u64 = 100;

pub struct RLogSender<T> {
  pub sender: Sender<T>,
  pub consumers: Arc<Mutex<Vec<Box<dyn LogConsumer<T>>>>>,
}

impl<T: Send + 'static> RLogSender<T> {
  pub fn new(consumers: Vec<Box<dyn LogConsumer<T>>>) -> Self {
    let (sx, rx) = mpsc::channel();
    let consumers = Arc::new(Mutex::new(consumers));
    let logger = Self {
      sender: sx,
      consumers: consumers.clone(),
    };
    thread::spawn(move || Self::recv(consumers, rx));
    logger
  }

  fn recv(consumers: Arc<Mutex<Vec<Box<dyn LogConsumer<T>>>>>, rx: Receiver<T>) {
    loop {
      let vals: Vec<_> = rx.try_iter().collect();
      if let Ok(mut consumers) = consumers.lock() {
        consumers.iter_mut().for_each(|c| c.consumes(&vals));
      }
      thread::sleep(Duration::from_millis(LOG_COLLECT_INTERVAL_MS));
    }
  }
}

pub trait LogConsumer<T>: Send {
  fn consumes(&mut self, vals: &[T]);
}

#[cfg(test)]
mod test {
  use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
  };

  use super::{LogConsumer, RLogSender};
  struct MockConsumer {
    pub logs: Arc<Mutex<Vec<i32>>>,
  }
  impl LogConsumer<i32> for MockConsumer {
    fn consumes(&mut self, vals: &[i32]) {
      if let Ok(mut logs) = self.logs.lock() {
        for val in vals {
          logs.push(*val);
        }
      }
    }
  }

  #[test]
  fn log_sender() {
    let logs = Arc::new(Mutex::new(vec![]));
    let logger: RLogSender<i32> = RLogSender::new(vec![
      Box::new(MockConsumer { logs: logs.clone() }),
      Box::new(MockConsumer { logs: logs.clone() }),
    ]);
    logger.sender.send(0).unwrap();

    thread::sleep(Duration::from_millis(200));
    assert!(logs.lock().unwrap().len() == 2);
  }
}
