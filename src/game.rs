use crate::common::Position;
use crate::street::{Action, ActionOption};
use crate::hand::{Hand, ShowdownResult};
use serde::{Serialize, Deserialize};
use poker::{Card};

pub struct Game{
    current_hand: Hand,
    button_seat: u8, // 0 or 1
}

// Game state struct passed to players
#[derive(Serialize, Deserialize)]
pub struct GameState{
    pot_size: u64,
    btn_stack: u64,
    bb_stack: u64,
    btn_added_chips_this_street: u64,
    bb_added_chips_this_street: u64,
    button_seat: u8,
    sb_size: u64,
    bb_size: u64,
    btn_hole_cards: Option<(String, String)>,
    bb_hole_cards: Option<(String, String)>,
    board_cards: Vec<String>,
    available_actions: Vec<ActionOption>,
    active_player: Position,
}

impl Game{
    pub fn new() -> Game{
        let deck: Vec<Card> = Card::generate_shuffled_deck().to_vec();
        let mut hand = Hand::new(deck, 995, 990, 5);
        Game{current_hand: hand, button_seat: 0}
    }

    pub fn new_with_stacks_and_sb(btn_stack: u64, bb_stack: u64, sb_size: u64) -> Game{
        let deck: Vec<Card> = Card::generate_shuffled_deck().to_vec();
        let hand = Hand::new(deck, btn_stack, bb_stack, sb_size);
        Game{current_hand: hand, button_seat: 0}
    }

    pub fn get_state_json(&self, for_seat: u8) -> String{
        let (btn_added_chips, bb_added_chips, minimum_raise_size, active_player) = self.current_hand.streets.last().unwrap().get_street_status();
        let button_seat = self.button_seat;

        let button_card1 = self.current_hand.btn_hole_cards.0.rank_suit_string();
        let button_card2 = self.current_hand.btn_hole_cards.1.rank_suit_string();

        let bb_card1 = self.current_hand.bb_hole_cards.0.rank_suit_string();
        let bb_card2 = self.current_hand.bb_hole_cards.1.rank_suit_string();

        let board: Vec<String> = self.current_hand.board_cards.iter().map(|card| card.rank_suit_string()).collect();
        
        let gamestate = GameState{
            pot_size: self.current_hand.pot,
            btn_stack: self.current_hand.btn_stack,
            bb_stack: self.current_hand.bb_stack,
            btn_added_chips_this_street: btn_added_chips,
            bb_added_chips_this_street: bb_added_chips,
            button_seat: button_seat,
            sb_size: self.current_hand.sb_size,
            bb_size: self.current_hand.sb_size*2,
            btn_hole_cards: match for_seat{
                _ if for_seat == button_seat => Some((button_card1, button_card2)),
                _ => None,
            },
            bb_hole_cards: match for_seat{
                _ if for_seat == button_seat => None,
                _ => Some((bb_card1, bb_card2)),
            },
            board_cards: board,
            available_actions: self.current_hand.streets.last().unwrap().get_available_actions(),
            active_player,
        };

        serde_json::to_string(&gamestate).unwrap()
        
    }

    // Returns the message to the user
    pub fn process_user_command(&mut self, input: &String, from_seat: u8) -> String{

        if input == "state"{
            return self.get_state_json(from_seat);
        }

        // See if it is the user's turn to act
        let (_,_,_,active_player) = self.current_hand.streets.last().unwrap().get_street_status();
        let player_position = match from_seat == self.button_seat{
            true => Position::Button,
            false => Position::BigBlind,
        };
        if player_position != active_player{
            return format!("{{\"action_response\": \"It is not your turn to act\"}}");
        }

        // Deserialize input as Action
        let action: Action = match serde_json::from_str(input){
            Ok(action) => action,
            Err(e) => {
                return format!("{{\"action_response\": \"{}\"}}", e);
            }
        };

        // Submit the action and return the response
        match self.current_hand.submit_action(action){
            Ok(showdown) => {
                match showdown{
                    Some(res) => format!("Showdown: {:?}", res), // Showdown
                    None => { // No showdown, but valid action
                        "{\"action_response\": \"ok\"}".to_string()
                    }
                }
            },
            Err(e) => format!("{{\"action_response\": \"{}\"}}", e), // Action was not allowed
        }
    
    }
        
}

#[cfg(test)]
mod tests{

    use super::*;

    #[test]
    fn initial_state(){
        let game = Game::new_with_stacks_and_sb(500, 600, 5);
        let state: GameState = serde_json::from_str(&game.get_state_json(0)).unwrap();

        assert_eq!(state.pot_size, 5 + 10); // SB + BB
        assert_eq!(state.btn_stack, 500 - 5); // Subtract the small blind
        assert_eq!(state.bb_stack, 600 - 10); // Subtract the big blind
        assert_eq!(state.btn_added_chips_this_street, 5);
        assert_eq!(state.bb_added_chips_this_street, 10);
        assert_eq!(state.button_seat, 0);
        assert_eq!(state.sb_size, 5);
        assert_eq!(state.bb_size, 10);
        assert_eq!(state.bb_hole_cards, None); // Opponent's cards are not revealed
        assert_eq!(state.board_cards.len(), 0);
        assert_eq!(state.available_actions, vec![ActionOption::Fold, ActionOption::Call(10), ActionOption::Raise(20,500)]);
        assert_eq!(state.active_player, Position::Button);


    }

}