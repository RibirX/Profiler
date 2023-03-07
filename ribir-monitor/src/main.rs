pub(crate) mod client_infos_store;
mod monitor;

use std::thread::{sleep, spawn};

use io::net::run;

fn main() {
  let handle = spawn(|| {
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(run("127.0.0.1:31813", move |_, rx| {
        // tokio::spawn(future)
      }))
  });
  handle
    .join()
    .expect("Couldn't join on the associated thread");
}
