//! Disposable protocol experiment, never part of the cloud service build.
use candid::{CandidType, Principal};
use ic_cdk::api::{msg_caller, msg_caller_info_data, msg_caller_info_signer};
use std::cell::RefCell;

#[derive(CandidType)]
struct Observation {
    caller: Principal,
    info_signer: Option<Principal>,
    info: Vec<u8>,
}

struct State {
    human: Principal,
    agent: Option<Principal>,
    edits: u64,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

#[ic_cdk::init]
fn init(human: Principal) {
    assert_ne!(human, Principal::anonymous());
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            human,
            agent: None,
            edits: 0,
        })
    });
}

fn observation() -> Observation {
    Observation {
        caller: msg_caller(),
        info_signer: msg_caller_info_signer(),
        info: msg_caller_info_data(),
    }
}

#[ic_cdk::query]
fn observe_query() -> Observation {
    observation()
}

#[ic_cdk::update]
fn observe_update() -> Observation {
    observation()
}

// Control experiment: a distinct authenticated principal has only read access.
#[ic_cdk::update]
fn grant_agent(agent: Option<Principal>) -> Result<(), String> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().unwrap();
        if msg_caller() != state.human {
            return Err("denied".into());
        }
        if agent == Some(Principal::anonymous()) || agent == Some(state.human) {
            return Err("agent must have a distinct authenticated principal".into());
        }
        state.agent = agent;
        Ok(())
    })
}

#[ic_cdk::query]
fn read() -> Result<(u64, Principal, Option<Principal>), String> {
    STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();
        let caller = msg_caller();
        if caller == state.human {
            Ok((state.edits, caller, None))
        } else if Some(caller) == state.agent {
            Ok((state.edits, caller, Some(state.human)))
        } else {
            Err("denied".into())
        }
    })
}

#[ic_cdk::update]
fn edit() -> Result<u64, String> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().unwrap();
        if msg_caller() != state.human {
            return Err("denied".into());
        }
        state.edits += 1;
        Ok(state.edits)
    })
}
