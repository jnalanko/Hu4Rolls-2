use std::io::BufRead;

use poker::{cards, Card, EvalClass, Evaluator, Rank};
mod street;
mod common;

use street::{Action, ActionOption, ActionResult, Street, StreetName};

struct Hand{

    btn_hole_cards: (Card, Card),
    bb_hole_cards: (Card, Card),
    board_cards: Vec<Card>,
    deck: Vec<Card>,

    sb_size: u64,

    btn_start_stack: u64, // Stack at the start of the hand
    bb_start_stack: u64, // Stack at the start of the hand

    btn_stack: u64, // Remaining stack after all action in the hand so far
    bb_stack: u64, // Remaining stack after all action in the hand so far
    pot: u64,

    streets: Vec<Street>,

}

impl Hand{

    // Assumes that both players have enough chips to post blinds
    fn new(mut deck: Vec<Card>, btn_stack: u64, bb_stack: u64, sb_size: u64) -> Hand{
        let btn_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let bb_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let board_cards = Vec::new();
        let pot = sb_size * 3;

        let mut streets = Vec::<Street>::new();

        let mut preflop = Street::new(StreetName::Preflop, 2*sb_size, btn_stack, bb_stack);
        preflop.submit_action(Action::PostBlind(sb_size)); // Small blind
        preflop.submit_action(Action::PostBlind(2*sb_size)); // Big blind

        streets.push(preflop);

        Hand{btn_hole_cards, 
            bb_hole_cards, 
            board_cards, 
            deck, 
            sb_size, 
            btn_start_stack: btn_stack, 
            btn_stack, 
            bb_start_stack: bb_stack,
            bb_stack, 
            pot, 
            streets}
    }

    fn run_showdown(&mut self){

        let eval = Evaluator::new();

        let mut btn_hand: Vec<Card> = vec![self.btn_hole_cards.0, self.btn_hole_cards.1];
        let mut bb_hand: Vec<Card> = vec![self.bb_hole_cards.0, self.bb_hole_cards.1];
        btn_hand.extend(self.board_cards.clone());
        bb_hand.extend(self.board_cards.clone());

        let btn_hand_eval = eval.evaluate(&btn_hand).unwrap();
        let bb_hand_eval = eval.evaluate(&bb_hand).unwrap();
        
        dbg!(btn_hand_eval);
        dbg!(bb_hand_eval);

        if btn_hand_eval.is_better_than(bb_hand_eval){
            println!("Button wins");
        } else if btn_hand_eval.is_worse_than(bb_hand_eval){
            println!("BB wins");
        } else {
            println!("Split pot");
        }

    }

    fn goto_next_street(&mut self){

        let street_name = self.streets.last().unwrap().street;
        let mut next_street_name = StreetName::Preflop;

        match street_name {
            StreetName::Preflop => {
                next_street_name = StreetName::Flop;
                self.board_cards.push(self.deck.pop().unwrap());
                self.board_cards.push(self.deck.pop().unwrap());
                self.board_cards.push(self.deck.pop().unwrap());
            },
            StreetName::Flop => {
                next_street_name = StreetName::Turn;
                self.board_cards.push(self.deck.pop().unwrap());
            },
            StreetName::Turn => {
                next_street_name = StreetName::River;
                self.board_cards.push(self.deck.pop().unwrap());
            },
            StreetName::River => {
                next_street_name = StreetName::End;
                self.run_showdown();
            },
            StreetName::End => (),
            _ => panic!("Invalid street"),
        };

        self.streets.push(Street::new(next_street_name, self.sb_size*2, self.btn_stack, self.bb_stack));
    }

    fn update_pot_and_stacks(&mut self){

        // Initialize the pot and stacks
        let mut pot = 0 as u64;
        let mut btn_stack = self.btn_start_stack as u64;
        let mut bb_stack = self.bb_start_stack as u64;

        // Iterate over all streets and update the pot and stacks
        for street in self.streets.iter(){
            let (btn_added_chips, bb_added_chips, _, _) = street.get_street_status();
            pot += btn_added_chips + bb_added_chips;
            btn_stack -= btn_added_chips;
            bb_stack -= bb_added_chips;
        }

        // Apply the updates
        self.pot = pot;
        self.btn_stack = btn_stack;
        self.bb_stack = bb_stack;
    }
  
    fn submit_action(&mut self, action: Action) -> Result<(), String>{

        let street = self.streets.last_mut().unwrap();

        if !street.is_valid_action(action) {
            return Err("Invalid action".to_string());
        }

        // Apply the action
        let result = street.submit_action(action);

        // Check the result and initiate next street if necessary
        match result{
            Ok(res) => match res{
                ActionResult::BettingClosed => self.goto_next_street(),
                ActionResult::BettingOpen => (),
            }
            Err(e) => return Err(e),
        }

        // Update the pot and stacks
        self.update_pot_and_stacks();

        Ok(())

    }

    fn finished(&self) -> bool{
        false // Todo
    }

}

fn play() {
    let stdin = std::io::stdin();
    let deck: Vec<Card> = Card::generate_shuffled_deck().to_vec();
    let mut hand = Hand::new(deck, 1000, 1000, 5);
    let options = hand.streets.last().unwrap().get_available_actions();
    dbg!(&options);    

    while !hand.finished(){
        let street = hand.streets.last().unwrap();
        let (btn_added_chips,bb_added_chips,minimum_raise_size, active_player) = street.get_street_status();
        println!("Pot, BB, BTN: {}, {}, {}", hand.pot, hand.bb_stack, hand.btn_stack);
        println!("Button has: {} {}", hand.btn_hole_cards.0.to_string(), hand.btn_hole_cards.1.to_string());
        println!("BB has: {} {}", hand.bb_hole_cards.0.to_string(), hand.bb_hole_cards.1.to_string());
        println!("Street status (btn added, bb added, minraise, to act): {} {} {} {:?}", btn_added_chips, bb_added_chips, minimum_raise_size, active_player);
        print!("Board: ");
        for card in &hand.board_cards{
            print!("{} ", card.to_string());
        }
        println!();

        let options = street.get_available_actions();
        let call_to_amount = match options.iter().find(|&x| match x{
            ActionOption::Call(_) => true,
            _ => false,
        }) {
            Some(ActionOption::Call(amount)) => *amount,
            _ => 0, // Todo: make this None or something
        };

        let input = stdin.lock().lines().next().unwrap().unwrap();
        let tokens = input.split_whitespace().collect::<Vec<&str>>();

        if tokens.len() == 1 && tokens[0] == "hh"{
            dbg!(&hand.streets);
        }
        let user_action =
        if tokens.len() == 0{
            None
        } else if tokens.len() == 1 {
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

        if let Some(action) = user_action{
            match hand.submit_action(action){
                Ok(_) => {
                    let options = hand.streets.last().unwrap().get_available_actions();
                    dbg!(options);
                },
                Err(e) => println!("{}", e),
            }
        } else {
            println!("Invalid action");
        }
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
