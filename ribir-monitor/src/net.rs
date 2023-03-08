use futures_util::{stream::SplitSink, SinkExt, Stream, StreamExt};
use monitor_msg::MonitorMsg;
use ribir_tracing::{info, warn};
use std::io::Result;
use std::net::SocketAddr;
use std::rc::Rc;
use std::{cell::RefCell, pin::Pin};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

pub const DEFAULT_LISTEN_ADDR: &'static str = "127.0.0.1:31813";

type Sender = SplitSink<WebSocketStream<TcpStream>, Message>;

#[derive(Clone)]
pub struct StreamHandle<S> {
  addr: SocketAddr,
  sender: Rc<RefCell<Option<Sender>>>,
  stream: Pin<Box<S>>,
}

impl<S: StreamExt> StreamHandle<S> {
  pub fn addr(&self) -> SocketAddr { self.addr }
  pub async fn close(&self) {
    if let Some(sender) = &mut *self.sender.borrow_mut() {
      let _ = sender.close().await;
    }
    self.sender.borrow_mut().take();
  }

  pub fn is_close(&self) -> bool { self.sender.borrow().is_none() }

  pub async fn next(&mut self) -> Option<<S as Stream>::Item> { self.stream.as_mut().next().await }
}

async fn recv(
  ws: WebSocketStream<TcpStream>,
  addr: SocketAddr,
) -> StreamHandle<impl StreamExt<Item = Option<MonitorMsg>>> {
  let (sender, recver) = ws.split();
  let sender = Rc::new(RefCell::new(Some(sender)));
  let sender2 = sender.clone();
  let stream = recver.then(move |message| {
    let sender = sender2.clone();
    async move {
      if let Ok(src) = message {
        match src {
          Message::Binary(data) => match bincode::deserialize::<MonitorMsg>(&data) {
            Ok(msg) => return Some(msg),
            Err(e) => warn!("deserialize failed {:?}", e),
          },
          Message::Close(_) => {
            sender.borrow_mut().take();
          }
          _ => (),
        };
      } else {
        warn!("network error:{:?}", message);
        sender.borrow_mut().take();
      }
      None
    }
  });

  StreamHandle {
    addr,
    sender,
    stream: Box::pin(stream),
  }
}

pub struct WsListener {
  listener: TcpListener,
}

impl WsListener {
  pub async fn new(addr: &str) -> WsListener {
    let try_socket = TcpListener::bind(addr).await;
    let listener = try_socket.expect("Failed to bind");
    WsListener { listener }
  }
}

impl WsListener {
  pub async fn accept(&self) -> Result<StreamHandle<impl StreamExt<Item = Option<MonitorMsg>>>> {
    let (stream, addr) = self.listener.accept().await?;
    let ws_stream = tokio_tungstenite::accept_async(stream)
      .await
      .expect("Error during the websocket handshake occurred");
    info!("WebSocket connection established: {}", addr);
    Ok(recv(ws_stream, addr).await)
  }
}
