use std::error::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde::{Deserialize, Serialize};
use serde_json::Result as JsonResult;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

#[derive(Debug, Deserialize, Serialize)]
struct Event {
    name: String,
    data: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (mut socket, _) = connect_async("wss://example.com").await?;
    let event = Event {
        name: "subscribe".to_string(),
        data: "some_data".to_string(),
    };
    let message = Message::Text(serde_json::to_string(&event)?);
    socket.send(message).await?;

    while let Some(message) = socket.next().await {
        let message = message?;
        if let Message::Text(text) = message {
            let event: JsonResult<Event> = serde_json::from_str(&text);
            if let Ok(event) = event {
                println!("{:?}", event);
            }
        }
    }

    Ok(())
}