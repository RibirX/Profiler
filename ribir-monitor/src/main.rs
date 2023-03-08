pub(crate) mod client_infos_store;
mod monitor;
mod net;
use std::thread::spawn;

use net::WsListener;

use crate::net::DEFAULT_LISTEN_ADDR;

fn main() {
  let handle = spawn(|| {
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(
        async {
          loop {
            let listener = WsListener::new(DEFAULT_LISTEN_ADDR).await;
            let mut handle = listener.accept().await.unwrap();
            let msg = handle.next().await;
            println!("{:?}", msg);
          }
        }
      )
  });
  handle
    .join()
    .expect("Couldn't join on the associated thread");
}
