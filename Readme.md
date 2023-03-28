# Heads up poker app

Work in Progress

## Running the server

Install Rust and run:

```
cargo run --bin server
```

## Interacting with the server

Create a game:

```
http POST localhost:8000/create_game id:=123 sb_size:=5 stacks:=[200,300]
```

Join the game you just created at seat 0:

```
http POST localhost:8000/join game_id:=123 seat:=0
```

This returns a websocket URL to that seat in that table, like this:

```
HTTP/1.1 200 OK
content-length: 65
content-type: application/json
date: Tue, 28 Mar 2023 17:46:37 GMT

{
    "url": "ws://127.0.0.1:8000/ws/d0906cd24a454ae68482e7980892718f"
}
```

There can be multiple websockets to the same seat. Also, nothing is currently preventing you from joining to your opponent's seat and seeing their hole cards.

To connect to the game, you can use `wscat`:

```
wscat -c ws://127.0.0.1:8000/ws/d0906cd24a454ae68482e7980892718f
```

To get the current game state, send the string `state`. It looks like this: 

```
{
  "pot_size": 15,
  "btn_stack": 200,
  "bb_stack": 300,
  "btn_added_chips_this_street": 5,
  "bb_added_chips_this_street": 10,
  "button_seat": 0,
  "sb_size": 5,
  "bb_size": 10,
  "btn_hole_cards": [
    "2d",
    "7s"
  ],
  "bb_hole_cards": null,
  "board_cards": [],
  "available_actions": [
    "Fold",
    {
      "Call": 10
    },
    {
      "Raise": [
        10,
        195
      ]
    }
  ],
  "active_player": "Button"
}
```

To play the game, send back any of the available actions as JSON. The server will respond with `{"action_response": "ok"}` if the action was accepted. If not, then there is an error message in place of "ok".