use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::{ws::Message, Filter, Rejection};

mod handler;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;
type MyClients = Arc<RwLock<HashMap<String, MyClient>>>;

mod street;
mod common;
mod hand;
mod game;

use game::Game;

type Games = Arc<RwLock<HashMap<u64, Game>>>;

#[derive(Debug, Clone)]
pub struct MyClient {
    pub game_id: u64,
    pub seat: u64,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

// https://github.com/zupzup/warp-websockets-example

#[tokio::main]
async fn main() {

    let games = Arc::new(RwLock::new(HashMap::<u64, Game>::new()));

    let myclients: MyClients = Arc::new(RwLock::new(HashMap::new()));

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let cors = warp::cors()
    .allow_any_origin()
    .allow_headers(vec![
        "Content-Type",
        "Content-Length",
        "ETag",
        "Date",
        "Connection",
    ])
    .allow_methods(vec!["POST", "GET"])
    .expose_headers(vec![
        "Content-Type",
        "Content-Length",
        "ETag",
        "Date",
        "Connection",
    ]);

    let join = warp::path("join");
    let join_routes = join
        .and(warp::post())
        .and(warp::body::json())
        .and(with_clients(myclients.clone()))
        .and_then(handler::join_handler);

    let create_game = warp::path("create_game");
    let create_game_routes = create_game
        .and(warp::post())
        .and(warp::body::json())
        .and(with_games(games.clone()))
        .and_then(handler::create_game_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_clients(myclients.clone()))
        .and(with_games(games.clone()))
        .and_then(handler::ws_handler);

    let routes = health_route
        .or(create_game_routes)
        .or(join_routes)
        .or(ws_route)
        .with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_clients(clients: MyClients) -> impl Filter<Extract = (MyClients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

fn with_games(games: Games) -> impl Filter<Extract = (Games,), Error = Infallible> + Clone {
    warp::any().map(move || games.clone())
}