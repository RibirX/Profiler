use crossbeam::channel::Receiver;
use futures_util::StreamExt;
use futures_util::stream::SplitSink;
use monitor_msg::MonitorMsg;
use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::LocalSet;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

pub const DEFAULT_LISTEN_ADDR: &'static str = "127.0.0.1:31813";

async fn recv(
  ws: WebSocketStream<TcpStream>,
  addr: SocketAddr,
  mut handle: impl FnMut(SocketAddr, Receiver<MonitorMsg>),
) {
  let (outgoing, incoming) = ws.split();
  let (mut tx, rx) = crossbeam::channel::unbounded::<MonitorMsg>();
  handle(addr, rx);
  incoming
    .fold(tx, |tx, message| async move {
      if let Ok(src) = message {
        match src {
          Message::Binary(data) => match bincode::deserialize(&data) {
            Ok(msg) => tx.send(msg).unwrap_or_else(|e| println!("{:?}", e)),
            Err(e) => println!("deserialize failed {:?}", e),
          },
          Message::Close(_) => {}
          _ => (),
        }
      }
      tx
    })
    .await;
}

pub async fn run(
  addr: &str,
  handle: impl FnMut(SocketAddr, Receiver<MonitorMsg>) + Clone + 'static,
) {
  LocalSet::new()
    .run_until(async {
      let try_socket = TcpListener::bind(addr).await;
      let listener = try_socket.expect("Failed to bind");

      while let Ok((stream, addr)) = listener.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(stream)
          .await
          .expect("Error during the websocket handshake occurred");
        println!("WebSocket connection established: {}", addr);
        tokio::task::spawn_local(recv(ws_stream, addr, handle.clone()));
      }
    })
    .await;
}
