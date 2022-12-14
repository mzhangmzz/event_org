use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::Serialize;
use near_sdk::{env, AccountId, Balance, near_bindgen};
use near_sdk::collections::{Vector};
use near_sdk::json_types::{U128};

const POINT_ONE: Balance = 100_000_000_000_000_000_000_000;


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
struct EventContract {
  // Max people that can join this event
  max_num: u32,
  // Min people need to join this event to make it happen
  min_num: u32,
  // Current number of people who joined
  cur_num: u32,
  // In yoctoNear
  participation_price: U128,
  // if the number of participants do not meet the target by this time, all funds will be returned
  // Number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
  deadline: u64,
}