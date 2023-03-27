use crate::common::Position;
use crate::street::{Action, ActionOption};
use crate::hand::Hand;
use poker::{cards, Card, EvalClass, Evaluator, Rank};

pub struct Game{
    current_hand: Hand,
}

impl Game{
    pub fn new() -> Game{
        let deck: Vec<Card> = Card::generate_shuffled_deck().to_vec();
        let mut hand = Hand::new(deck, 1000, 1000, 5);
        Game{current_hand: hand}
    }

    pub fn get_state_string(&self, for_who: Position) -> String{
        let hand = &self.current_hand;
        let street = hand.streets.last().unwrap();
        street.get_available_actions();
        let (btn_added_chips,bb_added_chips,minimum_raise_size, active_player) = street.get_street_status();
        
        let A = format!("Pot, BB, BTN: {}, {}, {}", hand.pot, hand.bb_stack, hand.btn_stack);
        let B = format!("Button has: {} {}", hand.btn_hole_cards.0.to_string(), hand.btn_hole_cards.1.to_string());
        let C = format!("BB has: {} {}", hand.bb_hole_cards.0.to_string(), hand.bb_hole_cards.1.to_string());
        let D = format!("Street status (btn added, bb added, to act): {} {} {:?}", btn_added_chips, bb_added_chips, active_player);
        let E = format!("{:?}", street.get_available_actions());

        let mut board_string = String::from("Board:");
        for card in &hand.board_cards{
            let card_str = format!(" {}", card.to_string());
            board_string.push_str(&card_str);
        }

        let state = format!("{}\n{}\n{}\n{}\n{}\n{}\n", A, B, C, D, E, board_string);

        // TODO: don't send the opponent's hole cards

        state
    }

    // Returns the message to the user
    pub fn process_user_command(&mut self, command: &String, from_who: Position) -> String{
        let tokens = command.split_whitespace().collect::<Vec<&str>>();

        let street = self.current_hand.streets.last().unwrap();
        let options = street.get_available_actions();
        let call_to_amount = match options.iter().find(|&x| match x{
            ActionOption::Call(_) => true,
            _ => false,
        }) {
            Some(ActionOption::Call(amount)) => *amount,
            _ => 0, // Todo: make this None or something
        };

        if tokens.len() == 0{
            return self.get_state_string(from_who);
        }

        let user_action =
        if tokens.len() == 1 {
            match tokens.first().unwrap(){
                &"fold" => Some(Action::Fold),
                &"check" => Some(Action::Check),
                &"call" => Some(Action::Call(call_to_amount)),
                &_ => None,
            }
        } else if tokens.len() == 2 {
            // Actions that require an amount
            let amount = tokens[1].parse::<u64>().unwrap();
            match tokens.first().unwrap(){
                &"bet" => Some(Action::Bet(amount)),
                &"raise" => Some(Action::Raise(amount)),
                &_ => None,
            }
        } else{ // Three or more tokens -> invalid
            None
        };

        // Submit the action and return the response
        if let Some(action) = user_action{
            match self.current_hand.submit_action(action){
                Ok(_) => {
                    let options = self.current_hand.streets.last().unwrap().get_available_actions();
                    format!("{:?}", options)
                },
                Err(e) => format!("{}", e),
            }
        } else {
            format!("Invalid action")
        }

    }
        
}