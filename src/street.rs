use std::cmp::max as max;
use std::cmp::min as min;
use serde::{Serialize,Deserialize};

use crate::common::{Position, other_player};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Action{
    Fold,
    Check,
    PostBlind(u64),
    Call(u64), // Call *to* not *by*
    Bet(u64), // Bet *to*, not *by*
    Raise(u64), // Raise *to*, not *by*
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StreetName{
    Preflop,
    Flop,
    Turn,
    River,
    End
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionOption{
    Fold,
    Check,
    PostBlind(u64),
    Call(u64), // Call *to* not *by*
    Bet(u64,u64), // Min bet, max bet. Bet *to*, not *by*
    Raise(u64,u64), // Min bet, max bet. Raise *to*, not *by*
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionResult{
    BettingOpen,
    BettingClosed,
    Fold(Position), // Player who folded
}

// This struct represents the state of a single betting round
#[derive(Debug, Clone)]
pub struct Street{
    pub street: StreetName,
    pub actions: Vec<Action>,
    pub min_open_raise: u64,

    pub btn_start_stack: u64, // Stack at the start of the street
    pub bb_start_stack: u64, // Stack at the start of the street

    pub btn_stack: u64, // Remaining stack after all action in the street so far
    pub bb_stack: u64, // Remaining stack after all action in the street so far
}

// These functions implement the betting logic of a single betting round
impl Street{

    pub fn new(street: StreetName, min_open_raise: u64, btn_start_stack: u64, bb_start_stack: u64) -> Street{
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

    pub fn get_first_to_act(&self) -> Position{
        match self.street{
            StreetName::Preflop => Position::Button,
            _ => Position::BigBlind,
        }
    }

    // Returns money added by button, money added by sb, the minimum raise size, next-to-act player
    pub fn get_street_status(&self) -> (u64, u64, u64, Position) {
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
                Action::PostBlind(amount) => {
                    *active_player_added_chips = *amount;
                    minimum_raise_size = 2 * amount;
                }
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
            }

            // Switch active player
            active_player = other_player(active_player)
        }

        (btn_added_chips, bb_added_chips, minimum_raise_size, active_player)
    }

    // Returns the valid actions for the player in turn.
    // For bets, raises, and allins, return the minimum and maximum amounts.
    pub fn get_available_actions(&self) -> Vec<ActionOption>{

        if self.street == StreetName::Preflop{
            match self.actions.len(){
                0 => {return vec![ActionOption::PostBlind(self.min_open_raise/2)];}, // Small blind
                1 => {return vec![ActionOption::PostBlind(self.min_open_raise)];}, // Big blind
                _ => (),
            }
        }
        
        let (btn_added_chips,bb_added_chips,minimum_raise_size, active_player) = self.get_street_status();

        let active_player_stack = match active_player{
            Position::Button => self.btn_stack,
            Position::BigBlind => self.bb_stack,
        };

        let active_player_initial_stack = match active_player{
            Position::Button => self.btn_start_stack,
            Position::BigBlind => self.bb_start_stack,
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
            let amount = min(max(btn_added_chips, bb_added_chips), active_player_initial_stack);
            valid_actions.push(ActionOption::Call(amount));
        }

        // Can we raise?
        if btn_added_chips + bb_added_chips > 0 && active_player_stack > max(btn_added_chips, bb_added_chips){
            valid_actions.push(ActionOption::Raise(minimum_raise_size, active_player_initial_stack));
        }
        
        // Can we check?
        if btn_added_chips == bb_added_chips{
            // Equal amount of added bets and raises -> check is possible
            valid_actions.push(ActionOption::Check);
        }
        
        valid_actions
    }

    pub fn submit_action(&mut self, action: Action) -> Result<ActionResult, String>{

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
            Action::Fold => result = ActionResult::Fold(active_player),
            Action::Check => {
                if active_player == last_to_act{
                    result = ActionResult::BettingClosed;
                }
            },
            Action::Call(amount) => {
                // Next step is dealt after a call unless we are before the flop
                // and the call is a limp from the button
                if self.street == StreetName::Preflop && active_player == Position::Button && amount == self.min_open_raise{
                    // Limp from the button -> Betting is still open
                } else{
                    result = ActionResult::BettingClosed;
                }
            },
            _ => ()
        }

        // Update stacks
        let (btn_added_chips, bb_added_chips, _, _) = self.get_street_status();
        self.btn_stack = self.btn_start_stack - btn_added_chips;
        self.bb_stack = self.bb_start_stack - bb_added_chips;

        Ok(result)

    }

    pub fn is_valid_action(&self, action: Action) -> bool{
        let available_actions = self.get_available_actions();
        match action{
            Action::Fold => available_actions.contains(&ActionOption::Fold),
            Action::Check => available_actions.contains(&ActionOption::Check),
            Action::Call(amount) => available_actions.contains(&ActionOption::Call(amount)),
            Action::PostBlind(_) => true, // We assume blind posting are always valid
            Action::Bet(amount) => {
                available_actions.iter().any(|x| match x{
                    ActionOption::Bet(minimum, maximum) => amount >= *minimum && amount <= *maximum,
                    _ => false,
                })
            },
            Action::Raise(amount) => {
                available_actions.iter().any(|x| match x{
                    ActionOption::Raise(minimum, maximum) => amount >= *minimum && amount <= *maximum,
                    _ => false,
                })
            },
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preflop_raise_raise_call(){
        let mut street = Street::new(StreetName::Preflop, 10, 1000, 2000);

        // Button's turn. Needs to post the small blind
        let actions = street.get_available_actions();

        assert_eq!(actions.len(), 1);
        assert!(actions.contains(&ActionOption::PostBlind(5)));

        assert!(street.submit_action(Action::PostBlind(5)).unwrap() == ActionResult::BettingOpen);

        // Big blind's turn. Needs to post the big blind
        let actions = street.get_available_actions();

        assert_eq!(actions.len(), 1);
        assert!(actions.contains(&ActionOption::PostBlind(10)));

        assert!(street.submit_action(Action::PostBlind(10)).unwrap() == ActionResult::BettingOpen);

        // Button's turn. Can raise, call or fold
        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ActionOption::Call(10)));
        assert!(actions.contains(&ActionOption::Raise(20, 1000)));
        assert!(actions.contains(&ActionOption::Fold));

        assert!(street.submit_action(Action::Raise(50)).unwrap() == ActionResult::BettingOpen);

        // Big blind's turn. Can raise, call or fold

        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ActionOption::Call(50)));
        assert!(actions.contains(&ActionOption::Raise(90, 2000)));
        assert!(actions.contains(&ActionOption::Fold));

        assert!(street.submit_action(Action::Raise(200)).unwrap() == ActionResult::BettingOpen);

        // Button's turn. Can raise, call or fold

        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ActionOption::Call(200)));
        assert!(actions.contains(&ActionOption::Raise(350, 1000)));
        assert!(actions.contains(&ActionOption::Fold));

        assert!(street.submit_action(Action::Call(200)).unwrap() == ActionResult::BettingClosed);

        // Check final status
        let (btn_added_chips, bb_added_chips, _, _) = street.get_street_status();
        assert_eq!(btn_added_chips, 200);
        assert_eq!(bb_added_chips, 200);


    }

    #[test]
    fn test_preflop_limp(){
        let mut street = Street::new(StreetName::Preflop, 10, 1000, 2000);
        assert!(street.submit_action(Action::PostBlind(5)).unwrap() == ActionResult::BettingOpen);
        assert!(street.submit_action(Action::PostBlind(10)).unwrap() == ActionResult::BettingOpen);

        // Button's turn. Can raise, call or fold
        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ActionOption::Call(10)));
        assert!(actions.contains(&ActionOption::Raise(20, 1000)));
        assert!(actions.contains(&ActionOption::Fold));

        assert!(street.submit_action(Action::Call(10)).unwrap() == ActionResult::BettingOpen);

        // Big blind's turn. Can check, raise, or fold
        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ActionOption::Check));
        assert!(actions.contains(&ActionOption::Raise(20, 2000)));
        assert!(actions.contains(&ActionOption::Fold));

        // Check that raising keeps the action open (street is cloned so this has no effect on the original street)
        assert!(street.clone().submit_action(Action::Raise(150)).unwrap() == ActionResult::BettingOpen);

        // Check that checking closes the action
        assert!(street.submit_action(Action::Check).unwrap() == ActionResult::BettingClosed);
        
        // Check final status
        let (btn_added_chips, bb_added_chips, _, _) = street.get_street_status();
        assert_eq!(btn_added_chips, 10);
        assert_eq!(bb_added_chips, 10);
    }

    #[test]
    fn test_bet_raise_all_in_call_on_flop(){

        // Sequence: bb bets 10, btn raises to 100, bb goes all in, btn calls

        let mut street = Street::new(StreetName::Flop, 10, 1000, 2000);
        
        // Big blind's turn
        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ActionOption::Check));
        assert!(actions.contains(&ActionOption::Bet(10,2000)));
        assert!(actions.contains(&ActionOption::Fold));

        let (btn_added_chips, bb_added_chips, _, active_player) = street.get_street_status();
        assert_eq!(btn_added_chips, 0);
        assert_eq!(bb_added_chips, 0);
        assert_eq!(street.btn_stack, 1000);
        assert_eq!(street.bb_stack, 2000);
        assert_eq!(active_player, Position::BigBlind);

        match street.submit_action(Action::Bet(10)){
            Ok(ActionResult::BettingOpen) => (),
            _ => assert!(false),
        }

        // Button's turn
        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(street.get_available_actions().contains(&ActionOption::Call(10)));
        assert!(street.get_available_actions().contains(&ActionOption::Raise(20,1000)));
        assert!(street.get_available_actions().contains(&ActionOption::Fold));

        let (btn_added_chips, bb_added_chips, _, active_player) = street.get_street_status();
        assert_eq!(btn_added_chips, 0);
        assert_eq!(bb_added_chips, 10);
        assert_eq!(street.btn_stack, 1000);
        assert_eq!(street.bb_stack, 1990);
        assert_eq!(active_player, Position::Button);

        match street.submit_action(Action::Raise(100)){
            Ok(ActionResult::BettingOpen) => (),
            _ => assert!(false),
        }

        // Big blinds's turn
        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(street.get_available_actions().contains(&ActionOption::Call(100)));
        assert!(street.get_available_actions().contains(&ActionOption::Raise(190,2000)));
        assert!(street.get_available_actions().contains(&ActionOption::Fold));

        let (btn_added_chips, bb_added_chips, _, active_player) = street.get_street_status();
        assert_eq!(btn_added_chips, 100);
        assert_eq!(bb_added_chips, 10);
        assert_eq!(street.btn_stack, 900);
        assert_eq!(street.bb_stack, 1990);
        assert_eq!(active_player, Position::BigBlind);

        match street.submit_action(Action::Raise(2000)){ // All in
            Ok(ActionResult::BettingOpen) => (),
            _ => assert!(false),
        }

        // Button's turn
        let actions = street.get_available_actions();
        dbg!(&actions);
        assert_eq!(actions.len(), 2);
        assert!(street.get_available_actions().contains(&ActionOption::Call(1000))); // Raise was 2000 but we have only 1000 left
        assert!(street.get_available_actions().contains(&ActionOption::Fold));

        let (btn_added_chips, bb_added_chips, _, active_player) = street.get_street_status();
        assert_eq!(btn_added_chips, 100);
        assert_eq!(bb_added_chips, 2000);
        assert_eq!(street.btn_stack, 900);
        assert_eq!(street.bb_stack, 0);
        assert_eq!(active_player, Position::Button);

        match street.submit_action(Action::Call(1000)){
            Ok(ActionResult::BettingClosed) => (),
            _ => assert!(false),
        }

        // Check the final state

        let (btn_added_chips, bb_added_chips, _, _) = street.get_street_status();
        assert_eq!(btn_added_chips, 1000);
        assert_eq!(bb_added_chips, 2000); // Returning the extra 1000 is not the responsibility of the street
        assert_eq!(street.btn_stack, 0);
        assert_eq!(street.bb_stack, 0);

    }

    #[test]
    fn test_check_check_on_the_flop(){
        let mut street = Street::new(StreetName::Flop, 10, 1000, 2000);

        let actions = street.get_available_actions();
        assert_eq!(actions.len(), 3);
        assert!(actions.contains(&ActionOption::Check));
        assert!(actions.contains(&ActionOption::Bet(10,2000)));
        assert!(actions.contains(&ActionOption::Fold));

        assert_eq!(street.submit_action(Action::Check).unwrap(), ActionResult::BettingOpen);
        assert_eq!(street.submit_action(Action::Check).unwrap(), ActionResult::BettingClosed);

        let (btn_added_chips, bb_added_chips, _, _) = street.get_street_status();
        assert_eq!(btn_added_chips, 0);
        assert_eq!(bb_added_chips, 0);
        assert_eq!(street.btn_stack, 1000);
        assert_eq!(street.bb_stack, 2000);
    }

}