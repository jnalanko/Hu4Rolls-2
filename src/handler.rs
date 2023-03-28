use crate::{ws, Client, Clients, Result, GameState, Games};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::Game;
use warp::{http::StatusCode, reply::json, ws::Message, Reply};

#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    user_id: usize,
}

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    url: String,
}

#[derive(Deserialize, Debug)]
pub struct Event {
    topic: String,
    user_id: Option<usize>,
    message: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateGameRequest {
    id: u64,
    sb_size: u64,
    stacks: (u64, u64), // Seat 0, seat 1
}

#[derive(Serialize, Debug)]
pub struct CreateGameResponse {
    message: String
}

pub async fn publish_handler(body: Event, clients: Clients) -> Result<impl Reply> {
    clients
        .read()
        .await
        .iter()
        .filter(|(_, client)| match body.user_id {
            Some(v) => client.user_id == v,
            None => true,
        })
        .filter(|(_, client)| client.topics.contains(&body.topic))
        .for_each(|(_, client)| {
            if let Some(sender) = &client.sender {
                let _ = sender.send(Ok(Message::text(body.message.clone())));
            }
        });

    Ok(StatusCode::OK)
}

pub async fn register_handler(body: RegisterRequest, clients: Clients) -> Result<impl Reply> {
    let user_id = body.user_id; // Used as the seat in the table
    if user_id >= 2{
        panic!("Only 2 players allowed");   
    }

    let uuid = Uuid::new_v4().as_simple().to_string();

    register_client(uuid.clone(), user_id, clients).await;
    Ok(json(&RegisterResponse {
        url: format!("ws://127.0.0.1:8000/ws/{}", uuid),
    }))
}

async fn register_client(id: String, user_id: usize, clients: Clients) {
    clients.write().await.insert(
        id,
        Client {
            user_id,
            topics: vec![String::from("cats")],
            sender: None,
        },
    );
}

pub async fn unregister_handler(id: String, clients: Clients) -> Result<impl Reply> {
    clients.write().await.remove(&id);
    Ok(StatusCode::OK)
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients, gamestate: GameState) -> Result<impl Reply> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, id, clients, c, gamestate))),
        None => Err(warp::reject::not_found()),
    }
}

pub async fn create_game_handler(body: CreateGameRequest, games: Games) -> Result<impl Reply> {
    let id = body.id;
    let sb_size = body.sb_size;
    let stacks = body.stacks;

    if games.read().await.contains_key(&id){
        Ok(json(&CreateGameResponse {
            message: format!("Game with id {} already exists", id),
        }))
    } else{
        let newgame = Game::new_with_stacks_and_sb(stacks.0, stacks.1, sb_size);
        games.write().await.insert(id, newgame);
        Ok(json(&CreateGameResponse {
            message: format!("Game created with id {id}, sb_size {sb_size}, stacks ({}, {})", stacks.0, stacks.1),
        }))
    }
}
pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}