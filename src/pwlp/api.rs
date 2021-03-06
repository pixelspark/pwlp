use super::program::Program;
use super::protocol::{Message, MessageType};
use super::server::{DeviceStatus, ServerState};
use eui48::MacAddress;
use phf::phf_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

static BUILTIN_PROGRAMS: phf::Map<&'static str, &'static [u8]> = phf_map! {
	"off" => include_bytes!("../programs/off.bin"),
	"default" => include_bytes!("../programs/default_serve.bin")
};

#[derive(Deserialize, Debug, Clone)]
pub struct APIConfig {
	pub enabled: bool,
	pub bind_address: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum APIError {
	NotFound(String),     // An entity was not found
	NetworkError(String), // Communicating with a device failed
}

#[derive(Serialize)]
struct ErrorReply {
	code: String,
	message: Option<String>,
}

#[derive(Serialize)]
struct SetReply {}

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
			APIError::NotFound(e) => ErrorReply {
				code: "not_found".into(),
				message: Some(e.clone()),
			},
			APIError::NetworkError(e) => ErrorReply {
				code: "network_error".into(),
				message: Some(e.clone()),
			},
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
pub struct IndexReply {}

#[derive(Serialize)]
pub struct DevicesReply<'a> {
	devices: &'a HashMap<String, DeviceStatus>,
}

async fn get_devices(state: Arc<Mutex<ServerState>>) -> Result<Box<dyn Reply>, Rejection> {
	let s = state.lock().unwrap();
	let sa = &(*s);
	Ok(Box::new(warp::reply::json(&DevicesReply {
		devices: &sa.devices,
	})))
}

async fn get_index(_state: Arc<Mutex<ServerState>>) -> Result<Box<dyn Reply>, Rejection> {
	Ok(Box::new(warp::reply::json(&IndexReply {})))
}

async fn get_device(
	state: Arc<Mutex<ServerState>>,
	device: String,
) -> Result<Box<dyn Reply>, Rejection> {
	let s = state.lock().unwrap();
	if s.devices.contains_key(&device) {
		Ok(Box::new(warp::reply::json(&s.devices[&device])))
	} else {
		return Err(warp::reject::custom(APIError::NotFound(
			"dveice not found".to_string(),
		)));
	}
}

async fn set_builtin_program(
	state: Arc<Mutex<ServerState>>,
	device_address: String,
	program_name: String,
) -> Result<Box<dyn Reply>, Rejection> {
	let mut s = state.lock().unwrap();
	if s.devices.contains_key(&device_address) {
		if !BUILTIN_PROGRAMS.contains_key(program_name.as_str()) {
			return Err(warp::reject::custom(APIError::NotFound(
				"built-in program not found".to_string(),
			)));
		}

		let program_code = BUILTIN_PROGRAMS[program_name.as_str()];
		let program = Program::from_binary(program_code.to_vec());
		let mut device_state = s.devices[&device_address].clone();
		device_state.program = Some(program.clone());

		// Send off the program
		let msg = Message::new(MessageType::Run, MacAddress::nil(), Some(&program.code)).unwrap();
		s.socket
			.send_to(
				&msg.signed(device_state.secret.as_bytes()),
				device_state.address,
			)
			.map_err(|e| warp::reject::custom(APIError::NetworkError(format!("{}", e))))?;
		s.devices.insert(device_address, device_state);

		Ok(Box::new(warp::reply::json(&SetReply {})))
	} else {
		return Err(warp::reject::custom(APIError::NotFound(
			"device not found".to_string(),
		)));
	}
}

pub async fn handle_rejection(err: Rejection) -> Result<Box<dyn Reply>, Infallible> {
	log::warn!("Rejection: {:?}", err);

	let (status, reply) = if err.is_not_found() {
		(
			StatusCode::NOT_FOUND,
			ErrorReply {
				message: Some("not found".into()),
				code: "not_found".into(),
			},
		)
	} else if let Some(e) = err.find::<APIError>() {
		(e.status(), e.reply())
	} else {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			ErrorReply {
				code: "internal_error".into(),
				message: Some(format!("unhandled rejection: {:?}", err)),
			},
		)
	};

	let json = warp::reply::json(&reply);
	Ok(Box::new(warp::reply::with_status(json, status)))
}

pub async fn serve_http(config: &APIConfig, state: Arc<Mutex<ServerState>>) {
	if !config.enabled {
		return;
	}

	let a = state.clone();
	let device = warp::get()
		.map(move || a.clone())
		.and(warp::path!("devices" / String).and(warp::path::end()))
		.and_then(get_device);

	let b = state.clone();
	let device_off = warp::get()
		.map(move || b.clone())
		.and(warp::path!("devices" / String / String).and(warp::path::end()))
		.and_then(set_builtin_program);

	let c = state.clone();
	let devices = warp::path!("devices")
		.and(warp::path::end())
		.map(move || c.clone())
		.and_then(get_devices);

	let d = state.clone();
	let index = warp::path::end().map(move || d.clone()).and_then(get_index);

	let routes = warp::any().and(device).or(device_off).or(devices).or(index);
	let mut bind_address = String::from("127.0.0.1:33334");

	if let Some(b) = &config.bind_address {
		bind_address = b.clone();
	}

	log::info!("HTTP API server listening at {}", bind_address);
	let address: SocketAddr = bind_address.parse().expect("valid IP address");
	warp::serve(routes.recover(handle_rejection))
		.run(address)
		.await;
}
