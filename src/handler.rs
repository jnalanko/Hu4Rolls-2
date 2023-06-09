use crate::{ws, MyClient, MyClients, Result, Games};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::Game;
use warp::{http::StatusCode, reply::json, ws::Message, Reply};

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

#[derive(Deserialize, Debug)]
pub struct JoinRequest {
    game_id: u64,
    seat: u64,
}

#[derive(Serialize, Debug)]
pub struct JoinResponse {
    url: String,
}

pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: MyClients, games: Games) -> Result<impl Reply> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, id, clients, c, games))),
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


pub async fn join_handler(body: JoinRequest, clients: MyClients) -> Result<impl Reply> {

    let uuid = Uuid::new_v4().as_simple().to_string(); // Websocket id

    clients.write().await.insert(
        uuid.clone(),
        MyClient {
            game_id: body.game_id,
            seat: body.seat,
            sender: None,
        },
    );

    Ok(json(&JoinResponse {
        url: format!("ws://127.0.0.1:8000/ws/{}", uuid),
    }))
}


pub async fn health_handler() -> Result<impl Reply> {
    Ok(StatusCode::OK)
}
