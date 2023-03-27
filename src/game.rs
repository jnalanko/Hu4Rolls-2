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
        
        let A = format!("Pot, BB, BTN: {}, {}, {}", hand.pot, hand.bb_stack, hand.btn_stack);
        let B = format!("Button has: {} {}", hand.btn_hole_cards.0.to_string(), hand.btn_hole_cards.1.to_string());
        let C = format!("BB has: {} {}", hand.bb_hole_cards.0.to_string(), hand.bb_hole_cards.1.to_string());
        let D = format!("{:?}", street.get_available_actions());
        let state = format!("{}\n{}\n{}\n{}\n", A, B, C, D);

        // TODO: don't send the opponent's hole cards

        state
    }
        
}