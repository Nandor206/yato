// =============== Imports ================
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;


#[derive(Serialize)]
#[serde(untagged)]
enum MPVCommand<'a> {
    Get { command: [&'a str; 2] },
    Seek { command: [&'a str; 3] },
}

#[derive(Deserialize, Debug)]
struct MPVResponse {
    data: f64,
    error: String,
}

/// Send a `get_property` command to MPV and return the result as `f64`
pub fn get_property(property: &str) -> Result<f64> {
    let mut stream = UnixStream::connect("/tmp/yato-mpvsocket")
        .with_context(|| "Failed to connect to MPV socket")?;
    let cmd = MPVCommand::Get {
        command: ["get_property", property],
    };

    let json = serde_json::to_string(&cmd)
        .with_context(|| "Failed to serialize get_property command to JSON")?;
    writeln!(stream, "{}", json)
        .with_context(|| "Failed to write get_property command to MPV socket")?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .with_context(|| "Failed to read response from MPV socket")?;


    let parsed: MPVResponse = serde_json::from_str(&response)
        .with_context(|| "Failed to parse response from MPV socket as JSON")?;


    if parsed.error == "success" {
        Ok(parsed.data)
    } else if parsed.error == "property unavailable" {
        log::warn!("MPV property '{}' is unavailable", property);
        Err(anyhow::anyhow!("MPV property '{}' is unavailable", property))
    } else {
        Err(anyhow::anyhow!("MPV IPC error: {}", parsed.error))
    }
}

// Send a `seek` command to MPV to seek to a specific time
pub fn seek_to(time: f64) -> Result<()> {
    let mut stream = UnixStream::connect("/tmp/yato-mpvsocket")
        .with_context(|| "Failed to connect to MPV socket")?;

    let time_str = time.to_string();
    let cmd = MPVCommand::Seek {
        command: ["seek", &time_str, "absolute"],
    };

    let json = serde_json::to_string(&cmd)
        .with_context(|| "Failed to serialize seek command to JSON")?;
    writeln!(stream, "{}", json)
        .with_context(|| "Failed to write seek command to MPV socket")?;
    Ok(())
}
