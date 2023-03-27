use poker::{cards, Card, EvalClass, Evaluator, Rank};
use std::io::BufRead;
use std::cmp::max as max; 
use std::cmp::min as min;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

struct BettingRound{
    actions: Vec<Action>,
    chips_in_pot_by_player: (u64, u64), // (Button, BigBlind)
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

    hand_history: Vec<Action>,

    cur_betting_round: BettingRound,

}

impl Hand{

    // Assumes that both players have enough chips to post blinds
    fn new(mut deck: Vec<Card>, btn_stack: u64, bb_stack: u64, sb_size: u64) -> Hand{
        let btn_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let bb_hole_cards = (deck.pop().unwrap(), deck.pop().unwrap());
        let board_cards = Vec::new();
        let pot = sb_size * 3;
        let hand_history = Vec::<Action>::new();
        let cur_betting_round = BettingRound{actions: Vec::new(), chips_in_pot_by_player: (0,0)};

        let mut hand = 
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
                hand_history,
                cur_betting_round};

        hand.hand_history.push(Action::Deal(DealerAction::Start));
        // ^ Not submitted via submit_action because otherwise it breaks
        // when it tries to split by street

        hand.submit_action(Action::Bet(sb_size)); // Small blind
        hand.submit_action(Action::Bet(2*sb_size)); // Big blind

        hand
    }

    // Splits the hand history by street. The first
    // action on each street is a Deal action.
    fn split_by_street(&self) -> Vec<&[Action]>{
        let mut start_indices: Vec<usize> = self.hand_history.iter().enumerate().filter(
            |&(_, &x)| match x{
                Action::Deal(_) => true,
                _ => false,
            }
        ).map(|(i,_)| i).collect();

        start_indices.push(self.hand_history.len()); // End sentinel

        let mut street_ranges = Vec::<&[Action]>::new();
        for i in 0..start_indices.len()-1{
            street_ranges.push(&self.hand_history[start_indices[i]..start_indices[i+1]]);
        }

        street_ranges
    }

    fn get_first_to_act(&self, street: DealerAction) -> Position{
        match street{
            DealerAction::Start => Position::Button,
            DealerAction::End => panic!("Hand has ended already"),
            _ => Position::BigBlind,
        }
    }

    // Returns money added by button, money added by sb, the minimum raise size, next-to-act player
    fn get_street_status(&self, active_street_actions: &[Action]) -> (u64, u64, u64, Position) {
        let street = self.extract_dealer_action(active_street_actions);
        let mut active_player = self.get_first_to_act(street);

        let mut btn_added_chips: u64 = 0;
        let mut bb_added_chips: u64 = 0;
        let mut minimum_raise_size: u64 = self.sb_size*2;

        for action in active_street_actions[1..].iter(){
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
            active_player = self.other_player(active_player)
        }

        (btn_added_chips, bb_added_chips, minimum_raise_size, active_player)
    }

    fn extract_dealer_action(&self, actions: &[Action]) -> DealerAction{
        match actions[0]{
            Action::Deal(street) => street,
            _ => panic!("First action on a street should be a Deal action"),
        }
    }

    // Returns the valid actions for the player in turn.
    // For bets, raises, and allins, return the minimum and maximum amounts.
    fn get_available_actions(&self) -> Vec<ActionOption>{

        let active_street_actions = *self.split_by_street().last().unwrap();
        
        let (btn_added_chips,bb_added_chips,minimum_raise_size, active_player) = self.get_street_status(active_street_actions);

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
        if true{ // TODO check that our stack has more chips than the bigger stack
            valid_actions.push(ActionOption::Raise(minimum_raise_size, active_player_stack));
        }
        
        // Can we check?
        if btn_added_chips == bb_added_chips{
            // Equal amount of added bets and raises -> check is possible
            valid_actions.push(ActionOption::Check);
        }
        
        valid_actions
    }

    fn other_player(&self, player: Position) -> Position{
        match player{
            Position::Button => Position::BigBlind,
            Position::BigBlind => Position::Button,
        }
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
        let active_street_actions = *self.split_by_street().last().unwrap();
        let (_, _, _, active_player) = self.get_street_status(active_street_actions);
        let street = self.extract_dealer_action(active_street_actions);
        let last_to_act = self.other_player(self.get_first_to_act(street));

        self.hand_history.push(action);

        match action{
            Action::Fold => self.deal_next_step(),
            Action::Check => {
                if active_player == last_to_act{
                    self.deal_next_step();
                }
            },
            Action::Call(amount) => {
                // Next step is dealt after a call unless we are before the flop
                // and the call is a limp from the button
                if street == DealerAction::Start && active_player == Position::Button && amount == 2*self.sb_size{
                    // Limp from the button -> no next step
                } else{
                    self.deal_next_step();
                }
            },
            _ => ()
        }

        // Update the pot and stack sizes
        let mut pot = 0 as u64;
        let mut btn_stack = self.btn_start_stack as u64;
        let mut bb_stack = self.bb_start_stack as u64;
        let streets = self.split_by_street();
        
        for street in streets{
            let (btn_added_chips, bb_added_chips, _, _) = self.get_street_status(street);
            dbg!(street, btn_added_chips, bb_added_chips);
            pot += btn_added_chips + bb_added_chips;
            btn_stack -= btn_added_chips;
            bb_stack -= bb_added_chips;
        }

        // Update the struct
        self.pot = pot;
        self.btn_stack = btn_stack;
        self.bb_stack = bb_stack;

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
        let active_street_actions = *hand.split_by_street().last().unwrap();
        let (btn_added_chips,bb_added_chips,minimum_raise_size, active_player) = hand.get_street_status(active_street_actions);
        println!("Pot, BB, BTN: {}, {}, {}", hand.pot, hand.bb_stack, hand.btn_stack);
        println!("Button has: {} {}", hand.btn_hole_cards.0.to_string(), hand.btn_hole_cards.1.to_string());
        println!("BB has: {} {}", hand.bb_hole_cards.0.to_string(), hand.bb_hole_cards.1.to_string());
        println!("Street status (btn added, bb added, minraise, to act): {} {} {} {:?}", btn_added_chips, bb_added_chips, minimum_raise_size, active_player);
        print!("Board: ");
        for card in &hand.board_cards{
            print!("{} ", card.to_string());
        }
        println!();

        let options = hand.get_available_actions();
        dbg!(&options);

        let call_to_amount = match options.iter().find(|&x| match x{
            ActionOption::Call(amount) => true,
            _ => false,
        }) {
            Some(ActionOption::Call(amount)) => *amount,
            _ => 0, // Todo: make this None or something
        };

        let input = stdin.lock().lines().next().unwrap().unwrap();
        let tokens = input.split_whitespace().collect::<Vec<&str>>();
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
            hand.submit_action(action); // TODO: validate
        } else {
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
