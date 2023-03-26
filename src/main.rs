use poker::{cards, Card, EvalClass, Evaluator, Rank};
use std::io::BufRead;
use std::cmp::max as max; 

#[derive(Debug)]
enum Position{
    Button,
    BigBlind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DealerAction{
    Start,
    Flop,
    Turn,
    River,
    End
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Action{
    Fold,
    Check,
    Call,
    Bet(u64),
    Raise(u64), // Raise *to*, not *by*
    Deal(DealerAction),
}

struct Hand{

    btn_hole_cards: (Card, Card),
    bb_hole_cards: (Card, Card),
    board_cards: Vec<Card>,
    deck: Vec<Card>,

    sb_size: u64,

    btn_stack: u64,
    bb_stack: u64,
    pot: u64,

    hand_history: Vec<Action>

}

impl Hand{

    // Assumes that both players have enough chips to post blinds
    fn new(mut deck: Vec<Card>, btn_stack: u64, bb_stack: u64, sb_size: u64) -> Hand{
        let btn_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let bb_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let board_cards = Vec::new();
        let pot = sb_size * 3;
        let mut hand_history = Vec::<Action>::new();
        hand_history.push(Action::Deal(DealerAction::Start));
        hand_history.push(Action::Bet(sb_size)); // Small blind
        hand_history.push(Action::Bet(2*sb_size)); // Big blind

        Hand{btn_hole_cards, bb_hole_cards, board_cards, deck, sb_size, btn_stack, bb_stack, pot, hand_history}
    }

    // Returns the valid actions for the player in turn.
    // For bets, raises, and allins, return the minimum and maximum amounts.
    fn get_available_actions(&self) -> Vec<Action>{
        let (street_action_index, street) = self.hand_history.iter().enumerate().rev().find(
            |&(_, &x)| match x{
                Action::Deal(_) => true,
                _ => false,
            }
        ).unwrap();

        let street_actions = &self.hand_history[street_action_index.. ];

        let mut active_player = match street{
            Action::Deal(DealerAction::Start) => Position::Button,
            Action::Deal(_) => Position::BigBlind,
            _ => panic!("Invalid street"),
        };

        // Simulate the street so far
        let mut btn_added_chips: u64 = 0;
        let mut bb_added_chips: u64 = 0;

        let mut minimum_raise_size: Option<u64> = Some(self.sb_size*2); // Minimum raise *to*, not by

        for action in street_actions{
            let to_call = (btn_added_chips as i64 - bb_added_chips as i64).abs() as u64; // |btn_added_chips - bb_added_chips|
            let bigger_added_chips_before_action = max(btn_added_chips, bb_added_chips);

            // Get a reference to the added chips of the active player
            let active_player_added_chips = match active_player{
                Position::Button => &mut btn_added_chips,
                Position::BigBlind => &mut bb_added_chips,
            };

            match action{
                Action::Fold => (),
                Action::Check => (),
                Action::Call => *active_player_added_chips += to_call,
                Action::Bet(amount) => {
                    minimum_raise_size = Some(2 * amount);
                    *active_player_added_chips = *amount;
                },
                Action::Raise(amount) => {
                    // Minimum raise size can not be None because there can only be a raise if there is has been a bet
                    let raise_by_amount = amount - bigger_added_chips_before_action;
                    minimum_raise_size = Some(bigger_added_chips_before_action + 2 * raise_by_amount);
                    *active_player_added_chips = *amount;
                },
                Action::Deal(_) => (),
            }

            // Switch active player
            active_player = match active_player{
                Position::Button => Position::BigBlind,
                Position::BigBlind => Position::Button,
            };
        }

        // Figure out valid actions
        let mut valid_actions =Vec::<Action>::new();

        // We can always fold if it's our turn to act
        valid_actions.push(Action::Fold);

        // Can we bet?
        if btn_added_chips == 0 && bb_added_chips == 0{
            // Bet must be possible if no chips have been added yet and the hand has not ended yet
            valid_actions.push(Action::Bet(minimum_raise_size.unwrap())); // Minimum bet
            valid_actions.push(Action::Bet(max(self.btn_stack, self.bb_stack))); // Maximum bet
        }

        // Can we call?
        if btn_added_chips != bb_added_chips{
            valid_actions.push(Action::Call);
        }

        // Can we raise?
        if btn_added_chips != bb_added_chips{
            match minimum_raise_size{ // Raises
                Some(minimum_raise_size) => {
                    let active_player_stack = match active_player{
                        Position::Button => self.btn_stack,
                        Position::BigBlind => self.bb_stack,
                    };
                    valid_actions.push(Action::Raise(minimum_raise_size)); // Minimum raise
                    valid_actions.push(Action::Raise(active_player_stack)); // Maximum raise
                },
                None => () // No raise possible
            }
        } 
        
        // Can we check?
        if btn_added_chips + bb_added_chips == 0 || btn_added_chips != bb_added_chips{
            // Equal amount of added bets and raises -> check is possible
            valid_actions.push(Action::Check);
        }
        
        valid_actions
    }

    fn is_valid_action(&self, action: Action) -> bool{
        match action{
            Action::Fold => true,
            Action::Check => true,
            Action::Call => true,
            Action::Bet(amount) => amount > 0,
            Action::Raise(amount) => amount > 0,
            Action::Deal(_) => true,
        }
    }

    // Todo: repeated code with get_available_actions
    fn get_active_player(&self) -> Position{
        let (street_action_index, street) = self.hand_history.iter().enumerate().rev().find(
            |&(_, &x)| match x{
                Action::Deal(_) => true,
                _ => false,
            }
        ).unwrap();

        let street_actions = &self.hand_history[street_action_index.. ];

        let mut active_player = match street{
            Action::Deal(DealerAction::Start) => Position::Button,
            Action::Deal(_) => Position::BigBlind,
            _ => panic!("Invalid street"),
        };

        active_player
    }

    fn deal_next_step(&mut self){
        // Find the last element in the hand history that is a Deal action
        let street = self.hand_history.iter().rev().find(
            |&x| match x{
                Action::Deal(_) => true,
                _ => false,
            }
        ).unwrap();

        match street {
            Action::Deal(DealerAction::Start) => {
                self.hand_history.push(Action::Deal(DealerAction::Flop));
                self.board_cards.push(self.deck.pop().unwrap());
                self.board_cards.push(self.deck.pop().unwrap());
                self.board_cards.push(self.deck.pop().unwrap());
            },
            Action::Deal(DealerAction::Flop) => {
                self.hand_history.push(Action::Deal(DealerAction::Turn));
                self.board_cards.push(self.deck.pop().unwrap());
            },
            Action::Deal(DealerAction::Turn) => {
                self.hand_history.push(Action::Deal(DealerAction::River));
                self.board_cards.push(self.deck.pop().unwrap());
            },
            Action::Deal(DealerAction::River) => {
                self.hand_history.push(Action::Deal(DealerAction::End));
            },
            Action::Deal(DealerAction::End) => (),
            _ => panic!("Invalid street"),
        };
    }

    fn submit_action(&mut self, action: Action){
        if self.is_valid_action(action) {
            self.hand_history.push(action);
            match action{
                Action::Fold => self.deal_next_step(),
                Action::Check => self.deal_next_step(),
                Action::Call => self.deal_next_step(),
                _ => ()
            }
        } else {
            println!("Invalid action");
        }
    }

    fn finished(&self) -> bool{
        false // Todo
    }
}

fn play() {
    let mut stdin = std::io::stdin();
    let deck: Vec<Card> = Card::generate_shuffled_deck().to_vec();
    let mut hand = Hand::new(deck, 1000, 1000, 5);
    while !hand.finished(){
        println!("Button has: {} {}", hand.btn_hole_cards.0.to_string(), hand.btn_hole_cards.1.to_string());
        println!("BB has: {} {}", hand.bb_hole_cards.0.to_string(), hand.bb_hole_cards.1.to_string());
        print!("Board: ");
        println!("Action is on: {:?}", hand.get_active_player());
        for card in &hand.board_cards{
            print!("{} ", card.to_string());
        }
        println!();

        let actions = hand.get_available_actions();
        dbg!(&actions);
        let input = stdin.lock().lines().next().unwrap().unwrap();
        let tokens = input.split_whitespace().collect::<Vec<&str>>();
        let user_action =
        if tokens.len() == 0{
            None
        } else if tokens.len() == 1 {
            match tokens.first().unwrap(){
                &"fold" => Some(Action::Fold),
                &"check" => Some(Action::Check),
                &"call" => Some(Action::Call),
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

        if let Some(action) = user_action{
            // Check if the action is valid
            if actions.contains(&action){
                hand.submit_action(action);
            } else{
                println!("Action invalid according to the rules");
            }
        } else{
            println!("Invalid action");
        }
        dbg!(&hand.hand_history);
    }
}

fn main() {

    play();

    return;

    // Create a reusable evaluator
    let eval = Evaluator::new();

    // Parse a `Vec` of cards from a str
    let royal_flush_cards: Vec<Card> = cards!("Ks Js Ts Qs As").try_collect().unwrap();
    dbg!(&royal_flush_cards);

    // Evaluate the hand
    let royal_flush_hand = eval.evaluate(royal_flush_cards).unwrap();

    assert!(matches!(
        royal_flush_hand.class(),
        EvalClass::StraightFlush {
            high_rank: Rank::Ace
        }
    ));
    assert!(royal_flush_hand.is_royal_flush());

    // Compare hands
    let pair_cards: Vec<Card> = cards!("3c 4h Td 3h Kd").try_collect().unwrap();
    let pair_hand = eval.evaluate(pair_cards).unwrap();
    assert!(royal_flush_hand.is_better_than(pair_hand));
}
