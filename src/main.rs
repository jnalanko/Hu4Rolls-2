use std::io::BufRead;

use poker::{cards, Card, EvalClass, Evaluator, Rank};
mod street;
mod common;
mod hand;
mod game;

use street::{Action, ActionOption};
use hand::Hand;

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

}
