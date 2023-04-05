
use crate::{MyClient, MyClients};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use crate::Game;
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

async fn broadcast_message(clients: &MyClients, message: &String){
    
    for (_, client) in clients.write().await.iter_mut() {
        if let Some(sender) = &client.sender {
            let _ = sender.send(Ok(Message::text(message)));
        }
    }
}

async fn client_msg(websocket_id: &str, game_id: u64, seat: u8, msg: Message, clients: &MyClients, games: &Games) {

    println!("received message from {}: {:?}", websocket_id, msg);
    let message = match msg.to_str() {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut locked = clients.write().await;

    // Find the game and process the user command for the game
    if let Some(v) = locked.get_mut(websocket_id) {
        if let Some(sender) = &v.sender {
            match games.write().await.get_mut(&game_id){ // Find the game
                Some(game) => { // Game found
                    if message == "state" {
                        let state = game.get_state_json(seat);
                        let _ = sender.send(Ok(Message::text(state)));
                    } else {
                        let (answer, hand_result) = game.process_user_command(&message.to_owned(), seat);
                        let _ = sender.send(Ok(Message::text(answer)));

                        drop(locked); // Drop the write lock to be able to broadcast

                        // Broadcast state to all clients
                        let state = game.get_state_json(seat as u8);
                        broadcast_message(clients, &state).await;

                        // If the game is over, broadcase the showdown result to al lclient
                        if let Some(hand_result) = hand_result {
                            broadcast_message(clients, &serde_json::to_string(&hand_result).unwrap()).await;
                        }
                    }
                },
                None => { // Game not found
                    let _ = sender.send(Ok(Message::text("Game not found")));
                }
            }
        }
    }

}