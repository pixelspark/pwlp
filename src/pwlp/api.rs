use super::server::{DeviceStatus, ServerState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use warp::{Filter, Rejection, Reply};

#[derive(Deserialize, Debug, Clone)]
pub struct APIConfig {
	pub enabled: bool,
	pub bind_address: Option<String>,
}

impl APIConfig {
	pub fn new() -> APIConfig {
		APIConfig {
			enabled: true,
			bind_address: None,
		}
	}
}

#[derive(Serialize)]
pub struct IndexReply<'a> {
	devices: &'a HashMap<String, DeviceStatus>,
}

async fn get_index(state: Arc<Mutex<ServerState>>) -> Result<Box<dyn Reply>, Rejection> {
	let s = state.lock().unwrap();
	let sa = &(*s);
	Ok(Box::new(warp::reply::json(&IndexReply {
		devices: &sa.devices,
	})))
}

pub async fn serve_http(config: &APIConfig, state: Arc<Mutex<ServerState>>) {
	if !config.enabled {
		return;
	}

	let index = warp::get()
		.map(move || (&state).clone())
		.and_then(get_index);
	let mut bind_address = String::from("127.0.0.1:33334");

	if let Some(b) = &config.bind_address {
		bind_address = b.clone();
	}

	println!("HTTP API server listening at {}", bind_address);
	let address: SocketAddr = bind_address.parse().expect("valid IP address");
	warp::serve(index).run(address).await;
}
