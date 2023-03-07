use crate::client_infos_store::ClientInfoStore;
use ribir::prelude::*;

#[derive(Default)]
pub struct Monitor {
  clients: Vec<ClientInfoStore>,
}

impl Compose for Monitor {
  fn compose(_: State<Self>) -> Widget { todo!() }
}
