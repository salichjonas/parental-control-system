# Blockchain-Based Parental Control System

A decentralized parental control system built on the Internet Computer (IC) blockchain platform, providing secure and transparent management of children's digital activities.

## Features

### Profile Management
- Create and manage child profiles
- Set age-appropriate restrictions
- Track daily screen time limits
- Monitor activity logs
- Implement app restrictions

### Security & Privacy
- Blockchain-based authentication
- Parent-only access controls
- Privacy-preserving activity logging
- Secure token management
- Transparent notification system

### Reward System
- Token-based incentives
- Screen time rewards
- Positive behavior reinforcement
- Achievement tracking
- Customizable reward parameters

## Technical Stack

- **Platform**: Internet Computer (IC)
- **Language**: Rust
- **Smart Contract Framework**: Candid
- **Storage**: Stable memory structures
- **Authentication**: Principal-based access control

## Getting Started

### Prerequisites

```bash
# Install the IC SDK
sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Building the Project

```bash
# Clone the repository
git clone https://github.com/salichjonas/parental-control-system.git
cd parental-control-system

# Deploy locally
dfx start --background
npm run gen-deploy
```

## Usage

### Creating a Child Profile

```bash
dfx canister call parental_control create_child_profile '(
  record {
    name = "Child Name";
    age = 10;
    daily_screen_time_limit = 7200;
  }
)'
```

### Setting Screen Time Limits

```bash
dfx canister call parental_control set_screen_time '(
  record {
    child_id = 1;
    time_limit = 3600;
  }
)'
```

### Logging Activity

```bash
dfx canister call parental_control log_activity '(
  record {
    child_id = 1;
    activity_type = "gaming";
    duration = 1800;
  }
)'
```

### Managing App Restrictions

```bash
dfx canister call parental_control restrict_app '(
  record {
    child_id = 1;
    app_name = "RestrictedApp";
  }
)'
```

## API Reference

### Update Methods

- `create_child_profile(name: String, age: u8, daily_screen_time_limit: u64) -> ChildProfile`
- `set_screen_time(payload: SetScreenTimePayload) -> Result<ChildProfile, String>`
- `log_activity(payload: LogActivityPayload) -> Result<ChildProfile, String>`
- `reward_tokens(payload: RewardTokenPayload) -> Result<ChildProfile, String>`
- `restrict_app(payload: RestrictAppPayload) -> Result<ChildProfile, String>`

### Query Methods

- `get_child_profile(child_id: u64) -> Result<ChildProfile, String>`
- `get_screen_time_report(child_id: u64) -> Result<(u64, u64), String>`
- `get_activity_logs(child_id: u64) -> Result<Vec<ActivityLog>, String>`

## Data Structures

### ChildProfile
```rust
struct ChildProfile {
    id: u64,
    parent: Principal,
    name: String,
    age: u8,
    daily_screen_time_limit: u64,
    current_screen_time: u64,
    token_rewards: u64,
    privacy_preserving_logs: Vec<ActivityLog>,
    restricted_apps: Vec<String>,
    parental_notifications: Vec<Notification>,
}
```


