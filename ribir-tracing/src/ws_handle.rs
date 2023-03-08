use crossbeam_epoch::{pin, Atomic, Owned, Shared};
use monitor_msg::MonitorMsg;
use serde::Serialize;
use std::net::TcpStream;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};
use url::Url;

use crate::log_writer::{LogConsumeHandle, RLogConsumers};

pub const SERVER_ADDR: &str = "ws://localhost:31813/socket";

struct Socket(WebSocket<MaybeTlsStream<TcpStream>>);

impl Drop for Socket {
  fn drop(&mut self) { let _ = self.0.close(None); }
}

impl Deref for Socket {
  type Target = WebSocket<MaybeTlsStream<TcpStream>>;
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Socket {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

#[derive(Clone)]
pub struct WSHandle {
  addr: String,
  socket: Arc<Atomic<Option<Socket>>>,
}

fn update_when<T>(
  ptr: &Atomic<T>,
  mut check: impl FnMut(&Shared<T>) -> bool,
  mut val_fn: impl FnMut() -> T,
) {
  let mut new_val: Option<Owned<T>> = None;
  loop {
    let guard = &pin();
    let old_val = ptr.load(Relaxed, guard);
    if check(&old_val) {
      if new_val.is_none() {
        new_val = Some(Owned::new(val_fn()));
      }
      let res = ptr.compare_exchange(old_val, new_val.unwrap(), Relaxed, Relaxed, &guard);
      match res {
        Ok(_) => break,
        Err(err) => new_val = Some(err.new),
      }
    } else {
      break;
    }
  }
}

impl WSHandle {
  pub fn new(addr: String) -> Self {
    Self {
      addr,
      socket: Arc::new(Atomic::new(None)),
    }
  }
  pub fn block_connect(&mut self) -> tungstenite::Result<()> {
    let mut res = Ok(());
    let res_ref = &mut res;
    update_when(
      &self.socket,
      |socket| unsafe { socket.deref().is_none() },
      || {
        connect(Url::parse(&self.addr).unwrap()).map_or_else(
          |e| {
            *res_ref = Err(e);
            None
          },
          |(socket, _)| Some(Socket(socket)),
        )
      },
    );
    res
  }

  pub fn disconnect(&mut self) {
    update_when(
      &self.socket,
      |socket| unsafe { socket.deref().is_some() },
      || None,
    )
  }

  pub fn is_connected(&self) -> bool {
    let guard = &pin();
    unsafe { self.socket.load(Relaxed, guard).deref().is_some() }
  }

  #[inline]
  fn send_to_remote<T: Serialize>(&mut self, vals: &[T]) -> Result<(), tungstenite::Error> {
    fn encode<T: Serialize>(val: &T) -> Vec<u8> {
      bincode::serialize(val)
        .or_else(|e| bincode::serialize(&MonitorMsg::MonitorError(e.to_string())))
        .unwrap()
    }

    fn handle_err(
      socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
      e: tungstenite::Error,
    ) -> Result<(), tungstenite::Error> {
      match e {
        tungstenite::Error::SendQueueFull(msg) => socket
          .write_pending()
          .and_then(|_| socket.write_message(msg)),
        tungstenite::Error::Capacity(_) => socket.write_message(Message::Binary(encode(
          &MonitorMsg::MonitorError(e.to_string()),
        ))),
        _ => Err(e),
      }
    }
    let guard = &pin();

    let socket = unsafe { self.socket.load(Relaxed, guard).deref_mut() };
    if let Some(socket) = socket {
      return vals
        .iter()
        .map(|val| Message::Binary(encode(val)))
        .try_for_each(|msg| {
          socket
            .write_message(msg)
            .or_else(|e| handle_err(socket, e))
            .or_else(|e| {
              // double call handle_err to deal with SendQueueFull which may failed again.
              let res = handle_err(socket, e);
              if let Err(err) = &res {
                println!("write_message faield {}", err.to_string());
              }
              res
            })
        });
    }
    Ok(())
  }
}

pub fn add_remote_listener(
  consumers: &mut RLogConsumers,
  addr: impl Into<String>,
) -> (WSHandle, LogConsumeHandle) {
  let ws = WSHandle::new(addr.into());
  let mut ws2 = ws.clone();
  let handle = consumers.add(Box::new(move |vals| {
    let _ = ws2.send_to_remote(vals);
  }));
  (ws, handle)
}

#[cfg(test)]
mod test {
  use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread::{self, spawn},
    time::Duration,
  };

  use crossbeam_epoch::Atomic;
  use monitor_msg::MonitorMsg;
  use tungstenite::{accept, Message};

  use crate::{
    log_writer::new_log_writer,
    ws_handle::{WSHandle, SERVER_ADDR},
  };
  fn init_server(recvs: Arc<Mutex<Vec<Message>>>) {
    spawn(move || {
      let server = TcpListener::bind("127.0.0.1:31813").unwrap();
      for stream in server.incoming() {
        let msgs = recvs.clone();
        spawn(move || {
          let mut websocket = accept(stream.unwrap()).unwrap();
          loop {
            let msg = websocket.read_message().unwrap();
            if msg.is_binary() || msg.is_text() {
              msgs.lock().unwrap().push(msg);
            }
          }
        });
      }
    });
  }
  #[test]
  fn websocket_log() {
    let recvs: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(vec![]));
    init_server(recvs.clone());
    let mut consumer = WSHandle {
      addr: SERVER_ADDR.to_string(),
      socket: Arc::new(Atomic::new(None)),
    };
    let _ = consumer.block_connect();
    let (sender, mut consumers) = new_log_writer();
    consumers.add(Box::new(move |vals| consumer.send_to_remote(vals).unwrap()));

    sender
      .send(MonitorMsg::EnterSpan {
        id: 1,
        time_stamp: Duration::from_secs(2),
      })
      .unwrap();
    thread::sleep(Duration::from_millis(20));
    assert!(recvs.lock().unwrap().len() == 1);
  }
}
