use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::Serialize;
use near_sdk::{env, AccountId, Balance, near_bindgen, Promise, PanicOnDefault};
use near_sdk::collections::{Vector};

const NEAR: Balance = 1_000_000_000_000_000_000_000_000;
const MIN_BALANCE: Balance = NEAR;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
#[derive(PanicOnDefault)]
struct EventContract {
  // Max people that can join this event
  max_num: u32,
  // Min people need to join this event to make it happen
  min_num: u32,
  cur_participants: Vector<AccountId>,
  // In yoctoNear
  price: Balance,
  // if the number of participants do not meet the target by this time, all funds will be returned
  // Number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
  deadline: u64,
}

#[near_bindgen]
impl EventContract {
  #[init]
  #[private] // Public - but only callable by env::current_account_id()
  pub fn init(max_num: u32, min_num: u32, price: Balance, deadline: u64) -> Self {
    Self {
      max_num,
      min_num,
      cur_participants: Vector::new(b"m"),
      price,
      deadline,
    }
  }

  #[payable]
  pub fn join(&mut self, account_id: AccountId) {
    if env::attached_deposit() != self.price {
      // TODO: not sure what's the right way to error here
      env::panic_str(&"Participant did not attach the right balance")
    }

    for acc in self.cur_participants.iter() {
      if acc == account_id {
        env::panic_str(&format!("{:?} has already joined", account_id));
      }
    }
    self.cur_participants.push(&account_id)
  }

  pub fn view_participants(&self) -> Vec<AccountId> {
    self.cur_participants.to_vec()
  }

  pub fn price(&self) -> Balance {
    self.price
  }

  pub fn check_deadline(&mut self) {
    if env::block_timestamp() >= self.deadline {
        // TODO: not complete now just experimenting
      let refund = (env::account_balance() - MIN_BALANCE) / self.cur_participants.len() as u128;
      self.cur_participants.iter().fold(None, |p: Option<Promise>, acc| {
        if let Some(p) = p {
          Some(p.then(Promise::new(acc).transfer(refund)))
        } else {
          Some(Promise::new(acc).transfer(refund))
        }
      });
    }
  }
}