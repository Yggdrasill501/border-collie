//! Minimal client for herdr's newline-delimited JSON socket API.

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{json, Value};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn socket_path() -> PathBuf {
    if let Some(path) = env::var_os("HERDR_SOCKET_PATH") {
        return PathBuf::from(path);
    }
    let config = match env::var_os("XDG_CONFIG_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"),
    };
    config.join("herdr/herdr.sock")
}

/// Sends one request on its own connection and returns its `result`.
pub fn request(method: &str, params: Value) -> Result<Value> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .map_err(|error| format!("cannot reach herdr at {}: {error}", path.display()))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let line = json!({ "id": "border-collie", "method": method, "params": params });
    stream.write_all(format!("{line}\n").as_bytes())?;

    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    let mut reply: Value = serde_json::from_str(&reply)?;
    if let Some(error) = reply.get("error") {
        let message = error["message"]
            .as_str()
            .map_or(error.to_string(), String::from);
        return Err(format!("{method}: {message}").into());
    }
    Ok(reply["result"].take())
}

/// Sends a request and returns the array stored under `key` in its result.
pub fn list(method: &str, key: &str) -> Result<Vec<Value>> {
    match request(method, json!({}))?[key].take() {
        Value::Array(items) => Ok(items),
        _ => Err(format!("{method}: missing `{key}`").into()),
    }
}
