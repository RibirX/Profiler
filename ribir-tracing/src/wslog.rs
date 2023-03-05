use monitor_msg::MonitorMsg;
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};
use url::Url;

const SERVER_ADDR: &'static str = "ws://localhost:31813/socket";
pub struct LogWS {
  addr: String,
  socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
}

impl LogWS {
  pub fn connect(&mut self) -> bool {
    if self.socket.is_none() {
      self.socket = connect(Url::parse(&self.addr).unwrap())
        .ok()
        .map(|(socket, _)| socket);
    }
    self.socket.is_some()
  }

  pub fn disconnect(&mut self) { self.socket.as_mut().map(|socket| socket.close(None)).take(); }

  #[inline]
  pub fn send_to_remote(&mut self, vals: &[MonitorMsg]) -> Result<(), tungstenite::Error> {
    fn encode(val: &MonitorMsg) -> Vec<u8> {
      bincode::serialize(val)
        .or_else(|e| bincode::serialize(&MonitorMsg::MonitorError(e.to_string())))
        .unwrap()
    }

    fn handle_err(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>, e: tungstenite::Error) -> Result<(), tungstenite::Error> {
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

    if self.socket.is_none() {
      return Ok(());
    }

    let socket = self.socket.as_mut().unwrap();
    vals
      .iter()
      .map(|val| Message::Binary(encode(val)))
      .try_for_each(|msg| {
        socket
          .write_message(msg)
          .or_else(|e| handle_err(socket, e))
          .or_else(|e| {
            let res = handle_err(socket, e);
            if let Err(err) = &res {
              println!("write_message faield {}", err.to_string());
            }
            res
        })
      })
  }

  pub fn is_connected(&self) { self.socket.is_some(); }
  // todo read_message
}

#[cfg(test)]
mod test {
  use std::{
    borrow::Cow,
    net::TcpListener,
    sync::{Arc, Mutex},
    thread::{self, spawn},
    time::Duration,
  };

  use monitor_msg::{Meta, MonitorMsg};
  use tungstenite::{accept, Message};

  use crate::{
    log_writer::new_log_writer,
    wslog::{LogWS, SERVER_ADDR},
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
    let mut consumer = LogWS {
      addr: SERVER_ADDR.to_string(),
      socket: None,
    };
    consumer.connect();
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
