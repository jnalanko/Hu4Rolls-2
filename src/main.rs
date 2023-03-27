use poker::{cards, Card, EvalClass, Evaluator, Rank};
use std::io::BufRead;
use std::cmp::max as max; 
use std::cmp::min as min;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Position{
    Button,
    BigBlind,
}

fn other_player(player: Position) -> Position{
    match player{
        Position::Button => Position::BigBlind,
        Position::BigBlind => Position::Button,
    }
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
    PostBlind(u64),
    Call(u64), // Call *to* not *by*
    Bet(u64), // Bet *to*, not *by*
    Raise(u64), // Raise *to*, not *by*
    Deal(DealerAction),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ActionOption{
    Fold,
    Check,
    Call(u64), // Call *to* not *by*
    Bet(u64,u64), // Min bet, max bet. Bet *to*, not *by*
    Raise(u64,u64), // Min bet, max bet. Raise *to*, not *by*
}

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

#[derive(Debug)]
struct Street{
    street: DealerAction,
    actions: Vec<Action>,
    min_open_raise: u64,

    btn_start_stack: u64, // Stack at the start of the street
    bb_start_stack: u64, // Stack at the start of the street

    btn_stack: u64, // Remaining stack after all action in the street so far
    bb_stack: u64, // Remaining stack after all action in the street so far
}

enum ActionResult{
    BettingOpen,
    BettingClosed,
}

// These functions implement the betting logic of a single betting round
impl Street{

    fn new(street: DealerAction, min_open_raise: u64, btn_start_stack: u64, bb_start_stack: u64) -> Street{
        Street{
            street,
            actions: Vec::new(),
            min_open_raise,
            btn_start_stack,
            bb_start_stack,
            btn_stack: btn_start_stack,
            bb_stack: bb_start_stack,
        }
    }

    fn get_first_to_act(&self) -> Position{
        match self.street{
            DealerAction::Start => Position::Button,
            DealerAction::End => panic!("Hand has ended already"),
            _ => Position::BigBlind,
        }
    }

    // Returns money added by button, money added by sb, the minimum raise size, next-to-act player
    fn get_street_status(&self) -> (u64, u64, u64, Position) {
        let mut active_player = self.get_first_to_act();

        let mut btn_added_chips: u64 = 0;
        let mut bb_added_chips: u64 = 0;
        let mut minimum_raise_size: u64 = self.min_open_raise;

        for action in &self.actions{
            let bigger_added_chips_before_action = max(btn_added_chips, bb_added_chips);

            // Get a reference to the added chips of the active player
            let active_player_added_chips = match active_player{
                Position::Button => &mut btn_added_chips,
                Position::BigBlind => &mut bb_added_chips,
            };

            match action{
                Action::Fold => (),
                Action::Check => (),
                Action::Call(amount) => *active_player_added_chips = *amount,
                Action::PostBlind(amount) => *active_player_added_chips = *amount,
                Action::Bet(amount) => {
                    minimum_raise_size = 2 * amount;
                    *active_player_added_chips = *amount;
                },
                Action::Raise(amount) => {
                    // Minimum raise size can not be None because there can only be a raise if there is has been a bet
                    let raise_by_amount = amount - bigger_added_chips_before_action;
                    minimum_raise_size = bigger_added_chips_before_action + 2 * raise_by_amount;
                    *active_player_added_chips = *amount;
                },
                Action::Deal(_) => (),
            }

            // Switch active player
            active_player = other_player(active_player)
        }

        (btn_added_chips, bb_added_chips, minimum_raise_size, active_player)
    }

    // Returns the valid actions for the player in turn.
    // For bets, raises, and allins, return the minimum and maximum amounts.
    fn get_available_actions(&self) -> Vec<ActionOption>{
        
        let (btn_added_chips,bb_added_chips,minimum_raise_size, active_player) = self.get_street_status();

        let active_player_stack = match active_player{
            Position::Button => self.btn_stack,
            Position::BigBlind => self.bb_stack,
        };

        // Figure out valid actions
        let mut valid_actions =Vec::<ActionOption>::new();

        // We can always fold if it's our turn to act
        valid_actions.push(ActionOption::Fold);

        // Can we bet?
        if btn_added_chips == 0 && bb_added_chips == 0{
            // Bet must be possible if no chips have been added yet and the hand has not ended yet
            valid_actions.push(ActionOption::Bet(minimum_raise_size, active_player_stack));
        }

        // Can we call?
        if btn_added_chips != bb_added_chips{
            valid_actions.push(ActionOption::Call(max(btn_added_chips, bb_added_chips)));
        }

        // Can we raise?
        if btn_added_chips + bb_added_chips > 0{ // TODO check that our stack has more chips than the bigger stack
            valid_actions.push(ActionOption::Raise(minimum_raise_size, active_player_stack));
        }
        
        // Can we check?
        if btn_added_chips == bb_added_chips{
            // Equal amount of added bets and raises -> check is possible
            valid_actions.push(ActionOption::Check);
        }
        
        valid_actions
    }

    fn submit_action(&mut self, action: Action) -> Result<ActionResult, String>{

        if !self.is_valid_action(action) {
            return Err("Invalid action".to_string());
        }

        // Get status before applying the action
        let (_, _, _, active_player) = self.get_street_status();
        let last_to_act = other_player(self.get_first_to_act());

        // Apply the action
        self.actions.push(action);

        let mut result = ActionResult::BettingOpen;

        // Determine if this action closes the betting round
        match action{
            Action::Fold => result = ActionResult::BettingClosed,
            Action::Check => {
                if active_player == last_to_act{
                    result = ActionResult::BettingClosed;
                }
            },
            Action::Call(amount) => {
                // Next step is dealt after a call unless we are before the flop
                // and the call is a limp from the button
                if self.street == DealerAction::Start && active_player == Position::Button && amount == self.min_open_raise{
                    // Limp from the button -> Betting is still open
                } else{
                    result = ActionResult::BettingClosed;
                }
            },
            _ => ()
        }

        // Update stacks
        let (btn_added_chips, bb_added_chips, _, _) = self.get_street_status();
        self.btn_stack = self.bb_start_stack - btn_added_chips;
        self.bb_stack = self.bb_start_stack - bb_added_chips;

        Ok(result)

    }

    fn is_valid_action(&self, action: Action) -> bool{
        let available_actions = self.get_available_actions();
        match action{
            Action::Fold => available_actions.contains(&ActionOption::Fold),
            Action::Check => available_actions.contains(&ActionOption::Check),
            Action::Call(amount) => available_actions.contains(&ActionOption::Call(amount)),
            Action::PostBlind(_) => true, // We assume blind posting are always valid
            Action::Bet(amount) => {
                available_actions.iter().any(|x| match x{
                    ActionOption::Bet(minimum, maximum) => (amount >= *minimum && amount <= *maximum),
                    _ => false,
                })
            },
            Action::Raise(amount) => {
                available_actions.iter().any(|x| match x{
                    ActionOption::Raise(minimum, maximum) => (amount >= *minimum && amount <= *maximum),
                    _ => false,
                })
            },
            Action::Deal(_) => false,
        }
    }

}

impl Hand{

    // Assumes that both players have enough chips to post blinds
    fn new(mut deck: Vec<Card>, btn_stack: u64, bb_stack: u64, sb_size: u64) -> Hand{
        let btn_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let bb_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let board_cards = Vec::new();
        let pot = sb_size * 3;

        let mut streets = Vec::<Street>::new();

        let mut preflop = Street::new(DealerAction::Start, 2*sb_size, btn_stack, bb_stack);
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
        let mut next_street_name = DealerAction::Start;

        match street_name {
            DealerAction::Start => {
                next_street_name = DealerAction::Flop;
                self.board_cards.push(self.deck.pop().unwrap());
                self.board_cards.push(self.deck.pop().unwrap());
                self.board_cards.push(self.deck.pop().unwrap());
            },
            DealerAction::Flop => {
                next_street_name = DealerAction::Turn;
                self.board_cards.push(self.deck.pop().unwrap());
            },
            DealerAction::Turn => {
                next_street_name = DealerAction::River;
                self.board_cards.push(self.deck.pop().unwrap());
            },
            DealerAction::River => {
                next_street_name = DealerAction::End;
                self.run_showdown();
            },
            DealerAction::End => (),
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
