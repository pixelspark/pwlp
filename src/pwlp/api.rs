
use warp::{Filter,Reply, Rejection};
use super::server::ServerState;
use std::sync::Mutex;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;

#[derive(Deserialize, Debug, Clone)]
pub struct APIConfig {
	pub enabled: bool,
	pub bind_address: Option<String>,
}

impl APIConfig {
	pub fn new() -> APIConfig {
		APIConfig {
			enabled: true,
			bind_address: None
		}
	}
}

#[derive(Serialize)]
pub struct IndexReply<'a> {
	state: &'a ServerState
}

async fn get_index(state: Arc<Mutex<ServerState>>) -> Result<Box<dyn Reply>, Rejection> {
	let s = state.lock().unwrap();
	let sa = &(*s);
	Ok(Box::new(warp::reply::json(&IndexReply {
		state: sa
	})))
}

pub async fn serve_http(config: &APIConfig, state: Arc<Mutex<ServerState>>) {
	if !config.enabled {
		return;
	}

	let index = warp::get().map(move || (&state).clone()).and_then(get_index);
	let mut bind_address = String::from("127.0.0.1:33334");

	if let Some(b) = &config.bind_address {
		bind_address = b.clone();
	}

	println!("HTTP API server listening at {}", bind_address);
	let address: SocketAddr = bind_address.parse().expect("valid IP address");
	warp::serve(index).run(address).await;
}
