use poker::{cards, Card, EvalClass, Evaluator, Rank};
use crate::street::{Action, ActionOption, ActionResult, Street, StreetName};

pub struct Hand{

    pub btn_hole_cards: (Card, Card),
    pub bb_hole_cards: (Card, Card),
    pub board_cards: Vec<Card>,
    pub deck: Vec<Card>,

    pub sb_size: u64,

    pub btn_start_stack: u64, // Stack at the start of the hand
    pub bb_start_stack: u64, // Stack at the start of the hand

    pub btn_stack: u64, // Remaining stack after all action in the hand so far
    pub bb_stack: u64, // Remaining stack after all action in the hand so far
    pub pot: u64,

    pub streets: Vec<Street>,

}

pub enum ShowdownResult{
    ButtonWins,
    BigBlindWins,
    SplitPot,
}

impl Hand{

    // Assumes that both players have enough chips to post blinds
    pub fn new(mut deck: Vec<Card>, btn_stack: u64, bb_stack: u64, sb_size: u64) -> Hand{
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

    pub fn run_showdown(&mut self) -> ShowdownResult{

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
            ShowdownResult::ButtonWins
        } else if btn_hand_eval.is_worse_than(bb_hand_eval){
            ShowdownResult::BigBlindWins
        } else {
            ShowdownResult::SplitPot
        }

    }

    pub fn goto_next_street(&mut self){

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

    pub fn update_pot_and_stacks(&mut self){

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
  
    pub fn submit_action(&mut self, action: Action) -> Result<(), String>{

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

    pub fn finished(&self) -> bool{
        false // Todo
    }

}