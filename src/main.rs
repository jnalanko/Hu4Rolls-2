use poker::{cards, Card, EvalClass, Evaluator, Rank};
use std::io::BufRead;
use std::cmp::max;

enum Position{
    Button,
    BigBlind,
}

#[derive(Copy, Clone, Debug)]
enum DealerAction{
    Start,
    Flop,
    Turn,
    River,
    Showdown
}

#[derive(Copy, Clone, Debug)]
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

        let mut minimum_raise_size: Option<u64> = None;

        for action in street_actions{
            let to_call = (btn_added_chips as i64 - bb_added_chips as i64).abs() as u64; // |btn_added_chips - bb_added_chips|

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
                    minimum_raise_size = Some(*amount);
                    *active_player_added_chips += amount
                },
                Action::Raise(amount) => {
                    // Minimum raise size can not be None because there can only be a raise if there is has been a bet
                    minimum_raise_size = Some(*amount - minimum_raise_size.unwrap());
                    *active_player_added_chips += amount
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
            valid_actions.push(Action::Bet(minimum_raise_size.unwrap()));
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
                    let raise_to_amount = std::cmp::max(btn_added_chips, bb_added_chips) + minimum_raise_size;
                    valid_actions.push(Action::Raise(raise_to_amount)); // Minimum raise
                    valid_actions.push(Action::Raise(active_player_stack)); // Maximum raise
                },
                None => () // No raise possible
            }
        } 
        
        // Can we check?
        if btn_added_chips != bb_added_chips{
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

    fn submit_action(&mut self, action: Action){
        if self.is_valid_action(action) {
            self.hand_history.push(action);
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
        let actions = hand.get_available_actions();
        dbg!(&actions);
        let input = stdin.lock().lines().next().unwrap().unwrap();
        match input.as_str(){
            "fold" => hand.submit_action(Action::Fold),
            "check" => hand.submit_action(Action::Check),
            "call" => hand.submit_action(Action::Call),
            "bet" => hand.submit_action(Action::Bet(5)),
            "raise" => hand.submit_action(Action::Raise(5)),
            _ => println!("Invalid action"),
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
