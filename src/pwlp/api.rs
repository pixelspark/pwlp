use super::server::{DeviceStatus, ServerState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use warp::{Filter, Rejection, Reply};
use std::convert::Infallible;
use warp::http::StatusCode;
use super::program::Program;
use super::protocol::{Message, MessageType};
use eui48::MacAddress;

#[derive(Deserialize, Debug, Clone)]
pub struct APIConfig {
	pub enabled: bool,
	pub bind_address: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum APIError {
	NotFound(String),	// An entity was not found
	NetworkError(String)	// Communicating with a device failed
}

#[derive(Serialize)]
struct ErrorReply {
	code: String,
	message: Option<String>,
}

#[derive(Serialize)]
struct SetReply {
}

impl warp::reject::Reject for APIError {}

impl APIError {
	fn status(&self) -> StatusCode {
		match self {
			APIError::NotFound(_) => StatusCode::NOT_FOUND,
			APIError::NetworkError(_) => StatusCode::BAD_GATEWAY,
		}
	}

	fn reply(&self) -> ErrorReply {
		match self {
			APIError::NotFound(e) => ErrorReply { code: "not_found".into(), message: Some(e.clone()) },
			APIError::NetworkError(e) => ErrorReply { code: "network_error".into(), message: Some(e.clone()) }
		}
	}
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

async fn get_device(state: Arc<Mutex<ServerState>>, device: String) -> Result<Box<dyn Reply>, Rejection> {
	let s = state.lock().unwrap();
	if s.devices.contains_key(&device) {
		Ok(Box::new(warp::reply::json(&s.devices[&device])))
	}
	else {
		return Err(warp::reject::custom(APIError::NotFound("dveice not found".to_string())));
	}
}

async fn set_off(state: Arc<Mutex<ServerState>>, device_address: String) -> Result<Box<dyn Reply>, Rejection> {
	let mut s = state.lock().unwrap();
	if s.devices.contains_key(&device_address) {
		// Send an off program!
		let program = Program::from_source("for(n=get_length) { set_pixel(n - 1, 0, 0, 0) }; blit; yield").unwrap();
		let mut device_state = s.devices[&device_address].clone();
		device_state.program = Some(program.clone());

		// Send off the program
		let msg = Message::new(MessageType::Run, MacAddress::nil(), Some(&program.code)).unwrap();
		s.socket.send_to(&msg.signed(device_state.secret.as_bytes()), device_state.address).map_err(|e| warp::reject::custom(APIError::NetworkError(format!("{}", e))))?;
		s.devices.insert(device_address, device_state);

		Ok(Box::new(warp::reply::json(&SetReply {
		})))
	}
	else {
		return Err(warp::reject::custom(APIError::NotFound("dveice not found".to_string())));
	}
}

pub async fn handle_rejection(err: Rejection) -> Result<Box<dyn Reply>, Infallible> {
	log::warn!("Rejection: {:?}", err);

	let (status, reply) = if err.is_not_found() {
		(StatusCode::NOT_FOUND, ErrorReply {
			message: Some("not found".into()),
			code: "not_found".into()
		})
	} 
	else if let Some(e) = err.find::<APIError>() {
		(e.status(), e.reply())
	}
	else {
		(StatusCode::INTERNAL_SERVER_ERROR, ErrorReply {
			code: "internal_error".into(),
			message: Some(format!("unhandled rejection: {:?}", err))
		})
	};

	let json = warp::reply::json(&reply);
	Ok(Box::new(warp::reply::with_status(json, status)))
}


pub async fn serve_http(config: &APIConfig, state: Arc<Mutex<ServerState>>) {
	if !config.enabled {
		return;
	}

	let a = state.clone();
	let get_info = warp::get()
		.map( move || a.clone())
		.and(warp::path!(String).and(warp::path::end()))
		.and_then(get_device);

	let b = state.clone();
	let post_set_off = warp::get()
		.map( move || b.clone())
		.and(warp::path!(String / "off").and(warp::path::end()))
		.and_then(set_off);

	let index = warp::path::end().map( move || state.clone())
		.and_then(get_index);

	let routes = warp::any()
		.and(get_info)
		.or(post_set_off)
		.or(index);
	let mut bind_address = String::from("127.0.0.1:33334");

	if let Some(b) = &config.bind_address {
		bind_address = b.clone();
	}

	log::info!("HTTP API server listening at {}", bind_address);
	let address: SocketAddr = bind_address.parse().expect("valid IP address");
	warp::serve(routes.recover(handle_rejection)).run(address).await;
}
