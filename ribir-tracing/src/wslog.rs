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
  pub fn listen(&mut self) -> bool {
    if self.socket.is_none() {
      self.socket = connect(Url::parse(&self.addr).unwrap())
        .ok()
        .map(|(socket, _)| socket);
    }
    self.socket.is_some()
  }

  #[inline]
  pub fn send_to_remote(&mut self, vals: &[MonitorMsg]) -> Result<(), tungstenite::Error> {
    if let Some(socket) = &mut self.socket {
      for val in vals {
        socket.write_message(Message::Binary(bincode::serialize(&val).unwrap()))?
      }
    }
    Ok(())
  }

  // todo read_message
}

#[cfg(test)]
mod test {
  use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread::{self, spawn},
    time::Duration,
  };

  use monitor_msg::MonitorMsg;
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
    consumer.listen();
    let (mut logger, mut consumers) = new_log_writer();
    consumers.add(Box::new(move |vals| consumer.send_to_remote(vals).unwrap()));

    logger.write(MonitorMsg::MonitorError("test".to_string()));
    thread::sleep(Duration::from_millis(20));
    assert!(recvs.lock().unwrap().len() == 1);
  }
}
