
use crate::{MyClient, MyClients};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use crate::Games;

// Create a new task to handle message from/to the client
pub async fn client_connection(ws: WebSocket, id: String, clients: MyClients, mut client: MyClient, games: Games) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    let seat = client.seat as u8;
    let game_id = client.game_id;

    client.sender = Some(client_sender);
    clients.write().await.insert(id.clone(), client);

    println!("{} connected", id);

    // The main loop that processes each message to the client
    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };
        client_msg(&id, game_id, seat, msg, &clients, &games).await;
    }

    clients.write().await.remove(&id);
    println!("{} disconnected", id);
}

async fn client_msg(websocket_id: &str, game_id: u64, seat: u8, msg: Message, clients: &MyClients, games: &Games) {

    println!("received message from {}: {:?}", websocket_id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut locked = clients.write().await;
    if let Some(v) = locked.get_mut(websocket_id) {
        if let Some(sender) = &v.sender {
            match games.write().await.get_mut(&game_id){ // Find the game
                Some(game) => { // Game found
                    let answer = game.process_user_command(&message.to_owned(), seat);
                    let _ = sender.send(Ok(Message::text(answer)));
                },
                None => { // Game not found
                    let _ = sender.send(Ok(Message::text("Game not found")));
                }
            }
        }
    }
}