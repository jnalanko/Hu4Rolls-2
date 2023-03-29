use crate::common::Position;
use crate::street::{Action, ActionOption};
use crate::hand::{Hand, HandResult};
use serde::{Serialize, Deserialize};
use poker::{Card, cards};

pub struct Game{
    current_hand: Hand,
    button_seat: u8, // 0 or 1
}

// Game state struct passed to players
#[derive(Serialize, Deserialize, Debug)]
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

    // If the action ends the hand, returns HandResult. Otherwise returns None, unless there
    // was an error, returns an error message as a string.
    pub fn submit_action(&mut self, action: Action, from_seat: u8) -> Result<Option<HandResult>, String>{

        // See if it is the user's turn to act
        let (_,_,_,active_player) = self.current_hand.streets.last().unwrap().get_street_status();
        let player_position = match from_seat == self.button_seat{
            true => Position::Button,
            false => Position::BigBlind,
        };
        
        if player_position != active_player{
            return Err("It is not your turn to act".to_string());
        }

        // Submit the action and return the response
        match self.current_hand.submit_action(action){
            Ok(hand_result) => {
                match hand_result{
                    Some(res) => {
                        // Deal a new hand
                        let deck: Vec<Card> = Card::generate_shuffled_deck().to_vec();

                        // New hand: swap stacks between button and sb
                        self.current_hand = Hand::new(deck, res.bb_next_hand_stack, res.btn_next_hand_stack, self.current_hand.sb_size);
                        self.button_seat = 1 - self.button_seat; // Switch who is on the button
                        Ok(Some(res))
                    }
                    None => { // No showdown, but valid action
                        Ok(None)
                    }
                }
            },
            Err(e) => Err(e), // Action was not allowed
        }
    }

    // Takes a user command and returns a JSON response to the user
    pub fn process_user_command(&mut self, input: &String, from_seat: u8) -> String{

        if input == "state"{
            return self.get_state_json(from_seat);
        }

        // Deserialize input as Action
        let action: Action = match serde_json::from_str(input){
            Ok(action) => action,
            Err(e) => {
                return format!("{{\"action_response\": \"{}\"}}", e);
            }
        };

        match self.submit_action(action , from_seat){
            Ok(hand_result) => {
                return "{\"action_response\": \"ok\"}".to_string();
            },
            Err(e) => {
                return format!("{{\"action_response\": \"{}\"}}", e);
            }
        }
    }
        
}

#[cfg(test)]
mod tests{

    use super::*;

    #[test]
    fn test_initial_state(){
        let mut game = Game::new_with_stacks_and_sb(500, 600, 5);
        assert!(game.current_hand.submit_action(Action::PostBlind(5)).is_ok());
        assert!(game.current_hand.submit_action(Action::PostBlind(10)).is_ok());
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

    #[test]
    fn test_fold_immediately(){
        let mut game = Game::new_with_stacks_and_sb(500, 600, 5);
        assert!(game.current_hand.submit_action(Action::PostBlind(5)).is_ok());
        assert!(game.current_hand.submit_action(Action::PostBlind(10)).is_ok());

        let res = game.current_hand.submit_action(Action::Fold).unwrap().unwrap();
        assert_eq!(res.winner, Some(Position::BigBlind));
        assert!(res.showdown.is_none());
        assert_eq!(res.bb_next_hand_stack, 600 + 5);
        assert_eq!(res.btn_next_hand_stack, 500 - 5);
    }

    #[test]
    fn test_finishing_a_hand_and_dealing_the_next_one(){
        let mut game = Game::new_with_stacks_and_sb(500, 600, 5);
        assert!(game.current_hand.submit_action(Action::PostBlind(5)).is_ok());
        assert!(game.current_hand.submit_action(Action::PostBlind(10)).is_ok());

        let state: GameState = serde_json::from_str(&game.get_state_json(0)).unwrap();
        dbg!(&state);
        // Button raises
        //assert!(game.current_hand.submit_action(Action::Raise(40)).is_ok());
        assert!(game.process_user_command(&serde_json::to_string(&Action::Raise(40)).unwrap(), 0) == "{\"action_response\": \"ok\"}");
        // BB folds
        assert!(game.process_user_command(&serde_json::to_string(&Action::Fold).unwrap(), 1) == "{\"action_response\": \"ok\"}");

        // The first hand should be over now

        assert!(game.current_hand.submit_action(Action::PostBlind(5)).is_ok());
        assert!(game.current_hand.submit_action(Action::PostBlind(10)).is_ok());

        let state: GameState = serde_json::from_str(&game.get_state_json(0)).unwrap();
        dbg!(&state);
        assert_eq!(state.pot_size, 5 + 10);

        // The button is now the big blind of the previous hand
        // That player lost 10 in the first hand and is now posting the sb of 5
        assert_eq!(state.btn_stack, 600 - 10 - 5); 

        // The big blind of the previous hand is now the button
        // That player won 10 in the first hand and is now posting the bb of 10
        assert_eq!(state.bb_stack, 500 + 10 - 10);

        assert_eq!(state.btn_added_chips_this_street, 5);
        assert_eq!(state.bb_added_chips_this_street, 10);
        assert_eq!(state.button_seat, 1); // The button of the previous hand is now the big blind
        assert_eq!(state.sb_size, 5);
        assert_eq!(state.bb_size, 10);
        assert_eq!(state.btn_hole_cards, None); // Opponent's cards are not revealed
        assert_eq!(state.board_cards.len(), 0);
        assert_eq!(state.active_player, Position::Button);
    }

    #[test]
    fn test_split_pot(){

        // Rig a deck to give both players AA and a straight flush on board
        let deck: Vec<Card> = cards!("2s 3s 4s 5s 6s Ah Ad Ac As").try_collect().unwrap();
        let hand = Hand::new(deck, 500, 600, 5);
        let mut game = Game{
            current_hand: hand,
            button_seat: 0,
        };

        game.submit_action(Action::PostBlind(5), 0).unwrap();
        game.submit_action(Action::PostBlind(10), 1).unwrap();

        game.submit_action(Action::Call(10), 0).unwrap();
        game.submit_action(Action::Check, 1).unwrap();

        game.submit_action(Action::Check, 1).unwrap();
        game.submit_action(Action::Check, 0).unwrap();

        game.submit_action(Action::Check, 1).unwrap();
        game.submit_action(Action::Check, 0).unwrap();

        game.submit_action(Action::Check, 1).unwrap();
        game.submit_action(Action::Check, 0).unwrap();

        // Now we should have a new hand with button and the bb reversed, with no changes to the stacks
        assert_eq!(game.current_hand.btn_stack, 600);
        assert_eq!(game.current_hand.bb_stack, 500);

    }

    #[test]
    fn test_showdown_winner(){

        // Rig a deck to deal out AA and KK and 2 4 6 8 T on the board
        let deck: Vec<Card> = cards!("2s 4h 6d 8d Ts Ah Ad Kc Ks").try_collect().unwrap();
        let hand = Hand::new(deck, 500, 600, 5);
        let mut game = Game{
            current_hand: hand,
            button_seat: 0,
        };

        game.submit_action(Action::PostBlind(5), 0).unwrap();
        game.submit_action(Action::PostBlind(10), 1).unwrap();

        game.submit_action(Action::Call(10), 0).unwrap();
        game.submit_action(Action::Check, 1).unwrap();

        game.submit_action(Action::Check, 1).unwrap();
        game.submit_action(Action::Check, 0).unwrap();

        game.submit_action(Action::Check, 1).unwrap();
        game.submit_action(Action::Check, 0).unwrap();

        game.submit_action(Action::Check, 1).unwrap();
        game.submit_action(Action::Check, 0).unwrap();

        // Now we should have a new hand with button and the bb reversed, with a win of 10 for the bb
        assert_eq!(game.current_hand.btn_stack, 610);
        assert_eq!(game.current_hand.bb_stack, 490);

    }

}
