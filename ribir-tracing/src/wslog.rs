use std::net::TcpStream;
use url::Url;

use crate::log_sender::LogConsumer;
use serde::Serialize;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

const SERVER_ADDR: &'static str = "ws://localhost:3012/socket";
struct LogWS {
  socket: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl LogWS {
  pub fn new() -> Option<Self> {
    connect(Url::parse(SERVER_ADDR).unwrap())
      .ok()
      .map(|(socket, _)| Self { socket })
  }
}

impl<T> LogConsumer<T> for LogWS
where
  T: Serialize,
{
  fn consumes(&mut self, vals: &[T]) {
    for val in vals {
      self
        .socket
        .write_message(Message::Binary(bincode::serialize(val).unwrap()))
        .unwrap();
    }
  }
}
