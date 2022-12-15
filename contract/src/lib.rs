use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Serialize, Serializer, Deserialize};
use near_sdk::{env, AccountId, Balance, near_bindgen, Promise, PanicOnDefault, serde_json};
use near_sdk::collections::{LookupMap, UnorderedSet, Vector};
use serde::ser::SerializeStruct;

const NEAR: Balance = 1_000_000_000_000_000_000_000_000;

#[derive(Serialize, Deserialize)]
pub struct EventSpec {
  // Max people that can join this event
  max_num: u64,
  // Min people need to join this event to make it happen
  min_num: u64,
  // In 0.001 * Near
  price: u64,
  // if the number of participants do not meet the target by this time, all funds will be returned
  // Number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
  deadline: u64,
  // If the event raises money successfully, where the money will go to. Usually this will
  // be set as the event owner
  beneficiary: AccountId,
}

type EventId = String;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Eq, PartialEq)]
enum EventStatus {
  // Deadline is not reached, or no claim has been submitted yet
  WAITING,
  // Deadline has passed and not enough people joined, refund has been issued
  FAILED,
  // Deadline has passed and enough people joined, fund has been sent to beneficiary
  SUCCESS,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct Event {
  // Max people that can join this event
  max_num: u64,
  // Min people need to join this event to make it happen
  min_num: u64,
  cur_participants: Vector<AccountId>,
  // In yoctoNear
  price: Balance,
  // If the number of participants do not meet the target by this time, all funds will be returned
  // Number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
  deadline: u64,
  // Owner of the event
  owner: AccountId,
  // If the event raises money successfully, where the money will go to. Usually this will
  // be set as the event owner
  beneficiary: AccountId,
  status: EventStatus,
}

impl Serialize for Event {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let mut state = serializer.serialize_struct("Event", 8)?;
    state.serialize_field("max_num", &self.max_num)?;
    state.serialize_field("min_num", &self.min_num)?;
    state.serialize_field("cur_participants", &self.cur_participants.to_vec())?;
    state.serialize_field("price", &self.price)?;
    state.serialize_field("deadline", &self.deadline)?;
    state.serialize_field("owner", &self.owner)?;
    state.serialize_field("beneficiary", &self.beneficiary)?;
    state.serialize_field("status", &self.status)?;
    state.end()
  }
}

#[derive(Debug)]
struct Error(String);

impl Event {
  fn new(spec: EventSpec, event_id: &String, owner: AccountId) -> Self {
      Self {
        max_num: spec.max_num,
        min_num: spec.min_num,
        cur_participants: Vector::new(event_id.try_to_vec().unwrap()),
        price: spec.price as u128 * 1_000_000_000_000_000_000_000_u128,
        deadline: spec.deadline,
        owner,
        beneficiary: spec.beneficiary,
        status: EventStatus::WAITING,
      }
  }

  fn join(&mut self, deposit: Balance, account_id: &AccountId) -> Result<(), Error> {
    if deposit != self.price {
      return Err(Error(format!("Participant did not attach the right balance, price {} attached: {}", self.price, deposit)));
    }

    for acc in self.cur_participants.iter() {
      if &acc == account_id {
        return Err(Error(format!("{:?} has already joined", account_id)));
      }
    }

    if self.cur_participants.len() >= self.max_num {
      return Err(Error(format!("Event is already full")));
    }

    self.cur_participants.push(account_id);

    Ok(())
  }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
struct Contract {
  event_ids: Vector<EventId>,
  events: LookupMap<EventId, Event>,
  events_by_owner: LookupMap<AccountId, Vector<EventId>>,
  events_by_participants: LookupMap<AccountId, Vector<EventId>>,
}

impl Default for Contract {
  fn default() -> Self {
      Self {
        event_ids: Vector::new(b"a"),
        events: LookupMap::new(b"b"),
        events_by_owner: LookupMap::new(b"c"),
        events_by_participants: LookupMap::new(b"d"),
      }
  }
}

#[near_bindgen]
impl Contract {
  pub fn start_event(&mut self, account_id: AccountId, event_spec: EventSpec) -> EventId {
    env::setup_panic_hook();
    let event_id: EventId = format!("event_{}", self.event_ids.len());
    let event = Event::new(event_spec, &event_id, account_id.clone());

    self.events.insert(&event_id, &event);
    let mut events = self.events_by_owner.get(&account_id).unwrap_or_else(|| Vector::new(format!("o#{}", account_id).as_bytes()));
    events.push(&event_id);
    self.events_by_owner.insert(&account_id, &events);
    self.event_ids.push(&event_id);
    event_id
  }

  #[payable]
  pub fn join(&mut self, event_id: EventId, account_id: AccountId)  {
    env::setup_panic_hook();
    let mut event = self.get_event(&event_id).expect("Event does not exist");
    event.join(env::attached_deposit(), &account_id).unwrap();
    self.events.insert(&event_id, &event);

    let mut events = self.events_by_participants.get(&account_id).unwrap_or_else(|| Vector::new(format!("p#{}", account_id).as_bytes()));
    events.push(&event_id);
    self.events_by_participants.insert(&account_id, &events);
  }

  pub fn claim(&mut self, event_id: EventId) {
    let mut event = self.get_event(&event_id).expect("Event does not exist");

    if event.status == EventStatus::WAITING && env::block_timestamp() >= event.deadline {
      if event.cur_participants.len() < event.min_num {
        event.status = EventStatus::FAILED;
        // If the event doesn't happen, refund everyone
        event.cur_participants.iter().fold(None, |p: Option<Promise>, acc| {
          if let Some(p) = p {
            Some(p.and(Promise::new(acc).transfer(event.price)))
          } else {
            Some(Promise::new(acc).transfer(event.price))
          }
        }).unwrap();
      } else {
        event.status = EventStatus::SUCCESS;
        // Event will happen, send money to beneficiary
        Promise::new(event.beneficiary.clone()).transfer(event.price * (event.cur_participants.len() as u128 ));
      };
    }
    self.events.insert(&event_id, &event);
  }

  pub fn get_all_events(&self) -> Vec<EventId> {
    self.event_ids.to_vec()
  }

  pub fn get_events_by_owner(&self, account_id: AccountId) -> Vec<EventId> {
    self.events_by_owner.get(&account_id).map(|v| v.to_vec()).unwrap_or_default()
  }

  pub fn get_events_by_participants(&self, account_id: AccountId) -> Vec<EventId> {
    self.events_by_participants.get(&account_id).map(|v| v.to_vec()).unwrap_or_default()
  }

  pub fn get_event(&self, event_id: &EventId) -> Option<Event> {
    self.events.get(&event_id)
  }
}

mod tests {
  use near_sdk::serde_json;

  #[test]
  fn test() {
    let string = "1000000000000000000000000000000000";
    let value: u128 = serde_json::from_str(string).unwrap();
    assert_eq!(value, 1000000000000000000000000000000000_u128);
  }
}