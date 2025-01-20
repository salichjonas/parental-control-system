/* Skill Exchange System - Rust Implementation */

#[macro_use]
extern crate serde;
use candid::{Decode, Encode, Principal};
use ic_cdk::api::caller;
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

use std::fmt;

// Enhanced User Struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    owner: Principal,
    username: String,
    skills: Vec<Skill>,
    rating: u64,
    completed_exchanges: u64,
    date_joined: u64,
    total_points: u64,
    level: u64,
    achievements: Vec<Achievement>,
    teaching_streak: u64,       // Consecutive days of teaching
    learning_streak: u64,       // Consecutive days of learning
    reputation_score: u64,      // Based on various factors
    endorsements_given: u64,    // Number of endorsements given to others
    endorsements_received: u64, // Number of endorsements received
}

#[derive(candid::CandidType, Serialize, Deserialize)]
struct UserStatistics {
    total_users: u64,
    total_skills: u64,
    total_exchanges: u64,
    average_skills_per_user: u64,
    average_exchanges_per_user: u64,
    highest_rated_user: Option<String>,
    highest_rating: u64,
}

// Enhanced Skill Struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Skill {
    name: String,
    category: String,
    experience_level: String, // "Beginner", "Intermediate", "Advanced", "Expert", "Master"
    description: String,
    endorsements: Vec<String>, // Principal IDs of endorsers
    verification_status: bool, // Verified by completing exchanges
    mastery_points: u64,       // Points earned in this skill
}

// Skill Offer Struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct SkillOffer {
    id: u64,
    teacher: String, // Principal as String
    skill_offered: Skill,
    skill_wanted: Skill,
    status: String, // "active", "matched", "completed"
    created_at: u64,
}

// Enhanced Match Struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Match {
    id: u64,
    offerer_id: String,
    accepter_id: String,
    skill_offer_id: u64,
    status: String,
    rating: u64,
    feedback: String,
    created_at: u64,
    points_earned: u64, // Points earned from this match
    skills_demonstrated: Vec<String>,
    learning_objectives_met: Vec<String>,
    time_spent: u64,    // Time spent in minutes
    quality_score: u64, // Based on various metrics
}

// Achievement Types
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
enum AchievementType {
    FirstExchange,
    PopularTeacher,
    HighRating,
    Specialist,
    Diversified,
    ConsistentLearner,
    CommunityBuilder,
}

// Achievement Struct
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Achievement {
    id: u64,
    achievement_type: String,
    title: String,
    description: String,
    points: u64,
    date_earned: u64,
}

// Skill Level Enum with Points
#[derive(candid::CandidType, Clone, Serialize, Deserialize, PartialEq)]
enum SkillLevel {
    Beginner = 1,
    Intermediate = 2,
    Advanced = 3,
    Expert = 4,
    Master = 5,
}

// Custom error type for user operations
#[derive(candid::CandidType, Serialize, Deserialize, Debug)]
pub enum UserError {
    InternalError(String),
    DatabaseError(String),
    NoUsersFound,
    UnauthorizedAccess,
    InvalidData(String),
}

// Implement Display trait for UserError
impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            UserError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            UserError::NoUsersFound => write!(f, "No users found"),
            UserError::UnauthorizedAccess => write!(f, "Unauthorized access"),
            UserError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

// Message Enum
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
enum Message {
    Success(String),
    Error(String),
    NotFound(String),
    InvalidPayload(String),
    UsernameTaken(String),
}

// Payload Structs
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct CreateUserPayload {
    username: String,
}

// Get All Users Payload
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct GetAllUsersPayload {
    page: Option<u32>,
    limit: Option<u32>,
}

// Get Filtered Users Payload
#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct GetFilteredUsersPayload {
    skill_filter: Option<String>,
    min_rating: Option<u64>,
    max_results: Option<u32>,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct AddSkillPayload {
    name: String,
    category: String,
    experience_level: String,
    description: String,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct CreateOfferPayload {
    skill_offered_name: String,
    skill_offered_category: String,
    skill_offered_level: String,
    skill_offered_description: String,
    skill_wanted_name: String,
    skill_wanted_category: String,
    skill_wanted_level: String,
    skill_wanted_description: String,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct AcceptOfferPayload {
    offer_id: u64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct CompleteLessonPayload {
    match_id: u64,
    rating: u64,
    feedback: String,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct EndorseSkillPayload {
    user_id: u64,
    skill_name: String,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct UpdateLearningObjectivesPayload {
    match_id: u64,
    objectives_met: Vec<String>,
}

// Implementing Storable for Skill
impl Storable for Skill {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Skill {
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

// Implementing Storable for SkillOffer
impl Storable for SkillOffer {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for SkillOffer {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Implementing Storable for Match
impl Storable for Match {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Match {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Implementing Storable for User
impl Storable for User {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for User {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Thread-local memory management
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static USERS: RefCell<StableBTreeMap<u64, User, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static OFFERS: RefCell<StableBTreeMap<u64, SkillOffer, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static MATCHES: RefCell<StableBTreeMap<u64, Match, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));
}

// HELPER FUNCTIONS

// Enhanced point calculation function
fn calculate_points(match_data: &Match, rating: u64) -> u64 {
    // Base points from rating (1-5 scale)
    let base_points = match rating {
        1 => 10,
        2 => 20,
        3 => 30,
        4 => 40,
        5 => 50,
        _ => 0,
    };

    // Quality score based on rating (as percentage)
    let quality_score = (rating as f64 * 20.0) as u64; // Convert 1-5 to percentage

    // Calculate total points
    let total_points = base_points +
        (quality_score / 2) + // Add half of quality score
        (match_data.learning_objectives_met.len() as u64 * 10); // 10 points per objective met

    total_points
}

// Check for achievements
fn check_achievements(user: &User) -> Vec<Achievement> {
    let mut new_achievements = Vec::new();
    let current_time = time();

    // First Exchange Achievement
    if user.completed_exchanges == 1 {
        new_achievements.push(Achievement {
            id: current_time,
            achievement_type: "FirstExchange".to_string(),
            title: "First Steps".to_string(),
            description: "Completed your first skill exchange".to_string(),
            points: 50,
            date_earned: current_time,
        });
    }

    // Popular Teacher Achievement
    if user.endorsements_received >= 10 {
        new_achievements.push(Achievement {
            id: current_time + 1,
            achievement_type: "PopularTeacher".to_string(),
            title: "Community Favorite".to_string(),
            description: "Received 10 skill endorsements".to_string(),
            points: 100,
            date_earned: current_time,
        });
    }

    // Consistent Learner Achievement
    if user.learning_streak >= 7 {
        new_achievements.push(Achievement {
            id: current_time + 2,
            achievement_type: "ConsistentLearner".to_string(),
            title: "Dedicated Learner".to_string(),
            description: "Maintained a 7-day learning streak".to_string(),
            points: 150,
            date_earned: current_time,
        });
    }

    new_achievements
}

// Create User
#[ic_cdk::update]
fn create_user(payload: CreateUserPayload) -> Result<User, Message> {
    if payload.username.is_empty() {
        return Err(Message::InvalidPayload(
            "Username cannot be empty".to_string(),
        ));
    }

    let caller = ic_cdk::caller(); // Get the Principal of the caller

    let user_id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Counter increment failed");

    USERS.with(|users| {
        if users
            .borrow()
            .iter()
            .any(|(_, user)| user.username == payload.username)
        {
            return Err(Message::UsernameTaken(format!(
                "Username {} is already taken",
                payload.username
            )));
        }

        let user = User {
            id: user_id,
            owner: caller,
            username: payload.username,
            skills: Vec::new(),
            rating: 0,
            completed_exchanges: 0,
            date_joined: time(),
            total_points: 0,
            level: 1,
            achievements: Vec::new(),
            teaching_streak: 0,
            learning_streak: 0,
            reputation_score: 0,
            endorsements_given: 0,
            endorsements_received: 0,
        };

        USERS.with(|users| {
            users.borrow_mut().insert(user_id, user.clone());
        });

        Ok(user)
    })
}

// Function to fetch user
#[ic_cdk::query]
fn get_user_by_id(user_id: u64) -> Result<User, Message> {
    USERS.with(|users| {
        if let Some(user) = users.borrow().get(&user_id) {
            Ok(user.clone())
        } else {
            Err(Message::NotFound("User not found".to_string()))
        }
    })
}

// Function to fetch user by owner
#[ic_cdk::query]
fn get_user_by_owner() -> Result<User, String> {
    USERS.with(|users| {
        let users = users.borrow();
        let user = users
            .iter()
            .find(|(_, user)| user.owner.to_text() == caller().to_text());
        match user {
            Some((_, user)) => Ok(user.clone()),
            None => Err("user company not found".to_string()),
        }
    })
}

// Function to fetch all users with pagination support
#[ic_cdk::query]
fn get_all_users(payload: GetAllUsersPayload) -> Result<Vec<User>, UserError> {
    // Validate pagination parameters
    let page_size = payload.limit.unwrap_or(10).min(50); // Maximum 50 users per page
    let current_page = payload.page.unwrap_or(1);

    if current_page == 0 {
        return Err(UserError::InvalidData(
            "Page number must be greater than 0".to_string(),
        ));
    }

    let start_idx = ((current_page - 1) * page_size) as usize;

    USERS
        .with(|users| {
            let users_ref = users.borrow();

            // Check if users store is accessible
            if users_ref.len() == 0 {
                return Err(UserError::NoUsersFound);
            }

            // Convert BTreeMap to Vec for pagination
            let all_users: Vec<User> = users_ref.iter().map(|(_, user)| user.clone()).collect();

            // Calculate pagination
            if start_idx >= all_users.len() {
                return Err(UserError::InvalidData(
                    "Page number exceeds available data".to_string(),
                ));
            }

            let paginated_users = all_users
                .into_iter()
                .skip(start_idx)
                .take(page_size as usize)
                .collect::<Vec<User>>();

            if paginated_users.is_empty() {
                Err(UserError::NoUsersFound)
            } else {
                Ok(paginated_users)
            }
        })
        .map_err(|e| UserError::InternalError(format!("Failed to access user store: {}", e)))
}
// Helper function to get filtered users
#[ic_cdk::query]
fn get_filtered_users(payload: GetFilteredUsersPayload) -> Result<Vec<User>, UserError> {
    let max_users = payload.max_results.unwrap_or(50).min(100); // Cap at 100 users

    USERS.with(|users| {
        let users_ref = users.borrow();

        if users_ref.len() == 0 {
            return Err(UserError::NoUsersFound);
        }

        let mut filtered_users: Vec<User> = users_ref
            .iter()
            .map(|(_, user)| user.clone())
            .filter(|user| {
                // Apply skill filter if provided
                if let Some(skill) = &payload.skill_filter {
                    if !user
                        .skills
                        .iter()
                        .any(|s| s.name.to_lowercase() == skill.to_lowercase())
                    {
                        return false;
                    }
                }

                // Apply rating filter if provided
                if let Some(min_r) = payload.min_rating {
                    if user.rating < min_r {
                        return false;
                    }
                }

                true
            })
            .take(max_users as usize)
            .collect();

        if filtered_users.is_empty() {
            Err(UserError::NoUsersFound)
        } else {
            // Sort by rating (highest first)
            filtered_users.sort_by(|a, b| b.rating.cmp(&a.rating));
            Ok(filtered_users)
        }
    })
}

// Function to get user statistics
#[ic_cdk::query]
fn get_user_statistics() -> Result<UserStatistics, UserError> {
    USERS.with(|users| {
        let users_ref = users.borrow();

        if users_ref.len() == 0 {
            return Err(UserError::NoUsersFound);
        }

        let total_users = users_ref.len() as u64;
        let mut total_skills = 0;
        let mut total_exchanges = 0;
        let mut highest_rated_user = None;
        let mut highest_rating = 0;

        for (_, user) in users_ref.iter() {
            total_skills += user.skills.len() as u64;
            total_exchanges += user.completed_exchanges;

            if user.rating > highest_rating {
                highest_rating = user.rating;
                highest_rated_user = Some(user.clone());
            }
        }

        Ok(UserStatistics {
            total_users,
            total_skills,
            total_exchanges,
            average_skills_per_user: (total_skills as f64 / total_users as f64) as u64,
            average_exchanges_per_user: (total_exchanges as f64 / total_users as f64) as u64,
            highest_rated_user: highest_rated_user.map(|u| u.username),
            highest_rating,
        })
    })
}

// Add Skill to User
#[ic_cdk::update]
fn add_skill(payload: AddSkillPayload) -> Result<Skill, Message> {
    let caller = ic_cdk::caller(); // Get the Principal of the caller
    let caller_as_string = caller.to_text(); // Convert Principal to String

    USERS.with(|users| {
        let mut users_ref = users.borrow_mut();
        let user_id = users_ref
            .iter()
            .find(|(_, user)| user.owner.to_string() == caller_as_string) // Compare as Strings
            .map(|(id, _)| id);

        match user_id {
            Some(user_id) => {
                let skill = Skill {
                    name: payload.name,
                    category: payload.category,
                    experience_level: payload.experience_level,
                    description: payload.description,
                    endorsements: Vec::new(),
                    verification_status: false,
                    mastery_points: 0,
                };

                if let Some(user_data) = users_ref.get(&user_id) {
                    let mut updated_user = user_data.clone();
                    updated_user.skills.push(skill.clone());
                    users_ref.insert(user_id, updated_user);
                    Ok(skill)
                } else {
                    Err(Message::NotFound("User not found".to_string()))
                }
            }
            None => Err(Message::NotFound("User not found".to_string())),
        }
    })
}

// Endorse Skill
#[ic_cdk::update]
fn endorse_skill(payload: EndorseSkillPayload) -> Result<String, Message> {
    let endorser_id = ic_cdk::caller().to_string();

    USERS.with(|users| {
        let mut users_ref = users.borrow_mut();
        if let Some(user) = users_ref.get(&payload.user_id) {
            let mut updated_user = user.clone();

            // Find the skill and update endorsements
            for skill in updated_user.skills.iter_mut() {
                if skill.name == payload.skill_name {
                    if !skill.endorsements.contains(&endorser_id) {
                        skill.endorsements.push(endorser_id.clone());
                        updated_user.endorsements_received += 1;

                        // Update mastery points based on endorsements
                        skill.mastery_points += 10;

                        users_ref.insert(payload.user_id, updated_user);
                        return Ok("Skill endorsed successfully".to_string());
                    }
                    return Err(Message::Error("Already endorsed this skill".to_string()));
                }
            }
            Err(Message::NotFound("Skill not found".to_string()))
        } else {
            Err(Message::NotFound("User not found".to_string()))
        }
    })
}

// Create Offer
#[ic_cdk::update]
fn create_offer(payload: CreateOfferPayload) -> Result<SkillOffer, Message> {
    let caller_principal = ic_cdk::caller(); // Get the Principal of the caller

    // First check if user exists by matching the caller's Principal
    let user_exists = USERS.with(|users| {
        users
            .borrow()
            .iter()
            .any(|(_, user)| user.owner == caller_principal)
    });

    if !user_exists {
        return Err(Message::NotFound("User not found".to_string()));
    }

    // Increment the offer ID
    let offer_id = ID_COUNTER.with(|counter| {
        let current_value = *counter.borrow().get();
        counter.borrow_mut().set(current_value + 1).unwrap();
        current_value
    });

    // Define the offered and wanted skills
    let skill_offered = Skill {
        name: payload.skill_offered_name,
        category: payload.skill_offered_category,
        experience_level: payload.skill_offered_level,
        description: payload.skill_offered_description,
        endorsements: Vec::new(),
        verification_status: false,
        mastery_points: 0,
    };

    let skill_wanted = Skill {
        name: payload.skill_wanted_name,
        category: payload.skill_wanted_category,
        experience_level: payload.skill_wanted_level,
        description: payload.skill_wanted_description,
        endorsements: Vec::new(),
        verification_status: false,
        mastery_points: 0,
    };

    // Create the skill offer
    let skill_offer = SkillOffer {
        id: offer_id,
        teacher: caller_principal.to_text(), // Store the Principal as a string in `teacher`
        skill_offered,
        skill_wanted,
        status: "active".to_string(),
        created_at: time(),
    };

    // Insert the offer into the OFFERS stable map
    OFFERS.with(|offers| offers.borrow_mut().insert(offer_id, skill_offer.clone()));

    Ok(skill_offer)
}

// Accept Offer
#[ic_cdk::update]
fn accept_offer(payload: AcceptOfferPayload) -> Result<Match, Message> {
    let principal_id = ic_cdk::caller().to_string();

    OFFERS.with(|offers| {
        let mut offers_ref = offers.borrow_mut();
        if let Some(offer) = offers_ref.get(&payload.offer_id) {
            if offer.status != "active" {
                return Err(Message::Error("Offer is no longer active".to_string()));
            }

            let match_id = ID_COUNTER.with(|counter| {
                let current_value = *counter.borrow().get();
                counter.borrow_mut().set(current_value + 1).unwrap();
                current_value
            });

            let new_match = Match {
                id: match_id,
                offerer_id: offer.teacher.clone(),
                accepter_id: principal_id.clone(),
                skill_offer_id: payload.offer_id,
                status: "ongoing".to_string(),
                rating: 0,
                feedback: String::new(),
                created_at: time(),
                points_earned: 0,
                skills_demonstrated: Vec::new(),
                learning_objectives_met: Vec::new(),
                time_spent: 0,
                quality_score: 0,
            };

            // Update the offer status
            let mut updated_offer = offer.clone();
            updated_offer.status = "matched".to_string();
            offers_ref.insert(payload.offer_id, updated_offer);

            // Insert the new match
            MATCHES.with(|matches| matches.borrow_mut().insert(match_id, new_match.clone()));

            Ok(new_match)
        } else {
            Err(Message::NotFound("Offer not found".to_string()))
        }
    })
}

// Enhanced Complete Lesson

// Enhanced Complete Lesson
#[ic_cdk::update]
fn complete_lesson(payload: CompleteLessonPayload) -> Result<String, Message> {
    if payload.rating == 0 || payload.rating > 5 {
        return Err(Message::InvalidPayload(
            "Rating must be between 1 and 5".to_string(),
        ));
    }

    MATCHES.with(|matches| {
        let mut matches_ref = matches.borrow_mut();
        if let Some(match_item) = matches_ref.get(&payload.match_id) {
            if match_item.status != "ongoing" {
                return Err(Message::Error("Match is not ongoing".to_string()));
            }

            let mut updated_match = match_item.clone();
            updated_match.status = "completed".to_string();
            updated_match.rating = payload.rating;
            updated_match.feedback = payload.feedback.clone();

            // Calculate quality score (rating converted to percentage)
            let quality_score = payload.rating * 20;

            // Base points from rating (1-5 scale)
            let base_points = match payload.rating {
                1 => 10,
                2 => 20,
                3 => 30,
                4 => 40,
                5 => 50,
                _ => 0,
            };

            // Calculate total points
            let objectives_bonus = (updated_match.learning_objectives_met.len() as u64) * 10; // 10 points per objective met
            let total_points = base_points + (quality_score / 2) + objectives_bonus;
            updated_match.points_earned = total_points;

            matches_ref.insert(payload.match_id, updated_match.clone());

            // Update user statistics and check for achievements
            USERS.with(|users| {
                let mut users_ref = users.borrow_mut();

                match Principal::from_text(&match_item.offerer_id) {
                    Ok(teacher_principal) => {
                        // Find user by owner (Principal)
                        let user_opt = users_ref.iter().find(|(_, user)| user.owner == teacher_principal);

                        if let Some((user_id, user)) = user_opt {
                            let mut updated_user = user.clone();

                            // Update user metrics
                            updated_user.rating = (updated_user.rating + payload.rating) / 2;
                            updated_user.completed_exchanges += 1;
                            updated_user.total_points += total_points;

                            // Check for new achievements
                            let new_achievements = check_achievements(&updated_user);
                            for achievement in new_achievements {
                                updated_user.total_points += achievement.points;
                                updated_user.achievements.push(achievement);
                            }

                            // Update level based on total points
                            updated_user.level = (updated_user.total_points / 1000) + 1;

                            users_ref.insert(user_id, updated_user);
                            Ok(format!(
                                "Lesson completed successfully. Points earned: {}. Points breakdown: Base points: {}, Quality score: {}, Objectives bonus: {}",
                                total_points, base_points, quality_score / 2, objectives_bonus
                            ))
                        } else {
                            Err(Message::Error("Teacher not found".to_string()))
                        }
                    },
                    Err(_) => Err(Message::Error("Invalid teacher principal ID".to_string())),
                }
            })
        } else {
            Err(Message::NotFound("Match not found".to_string()))
        }
    })
}

// Export Candid Interface
ic_cdk::export_candid!();
