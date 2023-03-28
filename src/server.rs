use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use warp::{ws::Message, Filter, Rejection};

mod handler;
mod ws;

type Result<T> = std::result::Result<T, Rejection>;
type Clients = Arc<RwLock<HashMap<String, Client>>>;

use poker::{cards, Card, EvalClass, Evaluator, Rank};
mod street;
mod common;
mod hand;
mod game;

use street::{Action, ActionOption};
use hand::Hand;
use game::Game;

type GameState = Arc<RwLock<Game>>;
type Games = Arc<RwLock<HashMap<u64, Game>>>;

#[derive(Debug, Clone)]
pub struct Client {
    pub user_id: usize,
    pub topics: Vec<String>,
    pub sender: Option<mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

// https://github.com/zupzup/warp-websockets-example

#[tokio::main]
async fn main() {

    let deck: Vec<Card> = Card::generate_shuffled_deck().to_vec();
    let gamestate = Arc::new(RwLock::new(Game::new()));
    let games = Arc::new(RwLock::new(HashMap::<u64, Game>::new()));

    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    let health_route = warp::path!("health").and_then(handler::health_handler);

    let register = warp::path("register");
    let register_routes = register
        .and(warp::post())
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(handler::register_handler)
        .or(register
            .and(warp::delete())
            .and(warp::path::param())
            .and(with_clients(clients.clone()))
            .and_then(handler::unregister_handler));

    let create_game = warp::path("create_game");
    let create_game_routes = create_game
        .and(warp::post())
        .and(warp::body::json())
        .and(with_games(games.clone()))
        .and_then(handler::create_game_handler);
    
    let publish = warp::path!("publish")
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(handler::publish_handler);

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_clients(clients.clone()))
        .and(with_gamestate(gamestate.clone()))
        .and_then(handler::ws_handler);

    let routes = health_route
        .or(register_routes)
        .or(create_game_routes)
        .or(ws_route)
        .or(publish)
        .with(warp::cors().allow_any_origin());

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

fn with_gamestate(gamestate: GameState) -> impl Filter<Extract = (GameState,), Error = Infallible> + Clone {
    warp::any().map(move || gamestate.clone())
}

fn with_games(games: Games) -> impl Filter<Extract = (Games,), Error = Infallible> + Clone {
    warp::any().map(move || games.clone())
}