use candid::{Decode, Encode, Principal};
use ic_cdk::api::caller;
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};
use serde::{Deserialize, Serialize};

// Types
type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// Models
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct ChildProfile {
    id: u64,
    parent: Principal,
    name: String,
    age: u8,
    daily_screen_time_limit: u64,  // in seconds
    current_screen_time: u64,      // in seconds
    token_rewards: u64,
    privacy_preserving_logs: Vec<ActivityLog>,
    restricted_apps: Vec<String>,
    parental_notifications: Vec<Notification>,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct ActivityLog {
    timestamp: u64,
    activity_type: String,
    duration: u64,  // in seconds
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Notification {
    timestamp: u64,
    message: String,
}

// Payloads
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct SetScreenTimePayload {
    child_id: u64,
    time_limit: u64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct LogActivityPayload {
    child_id: u64,
    activity_type: String,
    duration: u64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct RewardTokenPayload {
    child_id: u64,
    tokens: u64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct RestrictAppPayload {
    child_id: u64,
    app_name: String,
}

// Storage Implementation
impl Storable for ChildProfile {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for ChildProfile {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Thread-local Storage
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static CHILD_PROFILES: RefCell<StableBTreeMap<u64, ChildProfile, Memory>> = RefCell::new(
        StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))))
    );
}

// Profile Management
/// Create a new child profile.
/// Validates that the name is not empty and the age is a positive number.
#[ic_cdk::update]
fn create_child_profile(name: String, age: u8, daily_screen_time_limit: u64) -> Result<ChildProfile, String> {
    if name.is_empty() || age == 0 || daily_screen_time_limit == 0 {
        return Err("Invalid input: name, age, and daily screen time limit must be valid".to_string());
    }

    let child_id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1).unwrap();
        current_value
    });

    let new_profile = ChildProfile {
        id: child_id,
        parent: caller(),
        name,
        age,
        daily_screen_time_limit,
        current_screen_time: 0,
        token_rewards: 0,
        privacy_preserving_logs: Vec::new(),
        restricted_apps: Vec::new(),
        parental_notifications: Vec::new(),
    };

    CHILD_PROFILES.with(|profiles| profiles.borrow_mut().insert(child_id, new_profile.clone()));
    Ok(new_profile)
}

/// Update the daily screen time limit for a child.
/// Validates that the caller is the parent and the time limit is non-zero.
#[ic_cdk::update]
fn set_screen_time(payload: SetScreenTimePayload) -> Result<ChildProfile, String> {
    if payload.time_limit == 0 {
        return Err("Invalid input: Time limit must be greater than 0".to_string());
    }

    CHILD_PROFILES.with(|profiles| {
        let mut profiles_ref = profiles.borrow_mut();
        if let Some(mut profile) = profiles_ref.get(&payload.child_id) {
            if profile.parent != caller() {
                return Err("Unauthorized: Only the parent can modify screen time".to_string());
            }
            profile.daily_screen_time_limit = payload.time_limit;
            profiles_ref.insert(payload.child_id, profile.clone());
            Ok(profile)
        } else {
            Err("Child profile not found".to_string())
        }
    })
}

/// Log an activity for a child, updating screen time and sending notifications if limits are exceeded.
#[ic_cdk::update]
fn log_activity(payload: LogActivityPayload) -> Result<ChildProfile, String> {
    if payload.duration == 0 || payload.activity_type.is_empty() {
        return Err("Invalid input: Activity type and duration must be valid".to_string());
    }

    CHILD_PROFILES.with(|profiles| {
        let mut profiles_ref = profiles.borrow_mut();
        if let Some(mut profile) = profiles_ref.get(&payload.child_id) {
            let new_log = ActivityLog {
                timestamp: time(),
                activity_type: payload.activity_type.clone(),
                duration: payload.duration,
            };

            profile.current_screen_time += payload.duration;

            if profile.current_screen_time > profile.daily_screen_time_limit {
                profile.parental_notifications.push(Notification {
                    timestamp: time(),
                    message: "Daily screen time limit exceeded".to_string(),
                });
            }

            profile.privacy_preserving_logs.push(new_log);
            profiles_ref.insert(payload.child_id, profile.clone());
            Ok(profile)
        } else {
            Err("Child profile not found".to_string())
        }
    })
}

/// Reward tokens to a child. Validates that the caller is the parent.
#[ic_cdk::update]
fn reward_tokens(payload: RewardTokenPayload) -> Result<ChildProfile, String> {
    if payload.tokens == 0 {
        return Err("Invalid input: Tokens must be greater than 0".to_string());
    }

    CHILD_PROFILES.with(|profiles| {
        let mut profiles_ref = profiles.borrow_mut();
        if let Some(mut profile) = profiles_ref.get(&payload.child_id) {
            if profile.parent != caller() {
                return Err("Unauthorized: Only the parent can reward tokens".to_string());
            }
            profile.token_rewards += payload.tokens;
            profiles_ref.insert(payload.child_id, profile.clone());
            Ok(profile)
        } else {
            Err("Child profile not found".to_string())
        }
    })
}

/// Add an app restriction for a child. Validates that the caller is the parent.
#[ic_cdk::update]
fn restrict_app(payload: RestrictAppPayload) -> Result<ChildProfile, String> {
    if payload.app_name.is_empty() {
        return Err("Invalid input: App name cannot be empty".to_string());
    }

    CHILD_PROFILES.with(|profiles| {
        let mut profiles_ref = profiles.borrow_mut();
        if let Some(mut profile) = profiles_ref.get(&payload.child_id) {
            if profile.parent != caller() {
                return Err("Unauthorized: Only the parent can restrict apps".to_string());
            }

            if !profile.restricted_apps.contains(&payload.app_name) {
                profile.restricted_apps.push(payload.app_name);
                profile.parental_notifications.push(Notification {
                    timestamp: time(),
                    message: format!("New app restriction added: {}", payload.app_name),
                });
            }
            profiles_ref.insert(payload.child_id, profile.clone());
            Ok(profile)
        } else {
            Err("Child profile not found".to_string())
        }
    })
}

/// Retrieve a child's profile. Validates that the caller is the parent.
#[ic_cdk::query]
fn get_child_profile(child_id: u64) -> Result<ChildProfile, String> {
    CHILD_PROFILES.with(|profiles| {
        if let Some(profile) = profiles.borrow().get(&child_id) {
            if profile.parent != caller() {
                return Err("Unauthorized: Only the parent can view the profile".to_string());
            }
            Ok(profile)
        } else {
            Err("Child profile not found".to_string())
        }
    })
}

/// Retrieve a child's screen time report. Validates that the caller is the parent.
#[ic_cdk::query]
fn get_screen_time_report(child_id: u64) -> Result<(u64, u64), String> {
    CHILD_PROFILES.with(|profiles| {
        if let Some(profile) = profiles.borrow().get(&child_id) {
            if profile.parent != caller() {
                return Err("Unauthorized: Only the parent can view reports".to_string());
            }
            Ok((profile.current_screen_time, profile.daily_screen_time_limit))
        } else {
            Err("Child profile not found".to_string())
        }
    })
}

/// Retrieve a child's activity logs. Validates that the caller is the parent.
#[ic_cdk::query]
fn get_activity_logs(child_id: u64) -> Result<Vec<ActivityLog>, String> {
    CHILD_PROFILES.with(|profiles| {
        if let Some(profile) = profiles.borrow().get(&child_id) {
            if profile.parent != caller() {
                return Err("Unauthorized: Only the parent can view logs".to_string());
            }
            Ok(profile.privacy_preserving_logs.clone())
        } else {
            Err("Child profile not found".to_string())
        }
    })
}

// Candid interface export
ic_cdk::export_candid!();
