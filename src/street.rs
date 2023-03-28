use std::cmp::max as max;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ActionOption{
    Fold,
    Check,
    Call(u64), // Call *to* not *by*
    Bet(u64,u64), // Min bet, max bet. Bet *to*, not *by*
    Raise(u64,u64), // Min bet, max bet. Raise *to*, not *by*
}

pub enum ActionResult{
    BettingOpen,
    BettingClosed,
}

#[derive(Debug)]
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
            }

            // Switch active player
            active_player = other_player(active_player)
        }

        (btn_added_chips, bb_added_chips, minimum_raise_size, active_player)
    }

    // Returns the valid actions for the player in turn.
    // For bets, raises, and allins, return the minimum and maximum amounts.
    pub fn get_available_actions(&self) -> Vec<ActionOption>{
        
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
            Action::Fold => result = ActionResult::BettingClosed,
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