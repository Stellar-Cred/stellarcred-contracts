use soroban_sdk::{Address, Env, Map, String, Vec};

use crate::errors::Error;
use crate::types::{Credential, CredentialStats, DataKey, Issuer, LeaderboardEntry};

/// Approximate number of ledgers produced per day (5s close time).
const DAY_IN_LEDGERS: u32 = 17280;
/// How far to extend an entry's TTL on every write.
const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// Extend once the remaining TTL drops below this threshold.
const LIFETIME_THRESHOLD: u32 = BUMP_AMOUNT - DAY_IN_LEDGERS;
/// Maximum number of wallets tracked on the on-chain leaderboard.
const MAX_LEADERBOARD_SIZE: u32 = 200;

// ---------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------

/// Sets the contract admin. Only ever called from the constructor.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
    env.storage()
        .instance()
        .extend_ttl(LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

/// Returns the stored contract admin, if the contract has been initialized.
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

/// Requires that `admin` has authorized this call and matches the stored
/// contract admin.
pub fn require_admin(env: &Env, admin: &Address) -> Result<(), Error> {
    admin.require_auth();
    let stored = get_admin(env).ok_or(Error::NotAdmin)?;
    if stored != *admin {
        return Err(Error::NotAdmin);
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Issuers
// ---------------------------------------------------------------------

/// Returns whether an issuer record exists for `issuer`.
pub fn issuer_exists(env: &Env, issuer: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Issuer(issuer.clone()))
}

/// Persists an issuer record, adding it to the issuer index if new.
pub fn save_issuer(env: &Env, issuer: &Issuer) {
    let key = DataKey::Issuer(issuer.address.clone());
    env.storage().persistent().set(&key, issuer);
    env.storage()
        .persistent()
        .extend_ttl(&key, LIFETIME_THRESHOLD, BUMP_AMOUNT);

    let mut addrs = load_issuer_addrs(env);
    if !addrs.contains(&issuer.address) {
        addrs.push_back(issuer.address.clone());
        let addrs_key = DataKey::IssuerAddrs;
        env.storage().persistent().set(&addrs_key, &addrs);
        env.storage()
            .persistent()
            .extend_ttl(&addrs_key, LIFETIME_THRESHOLD, BUMP_AMOUNT);
    }
}

/// Loads a single issuer record.
pub fn load_issuer(env: &Env, issuer: &Address) -> Option<Issuer> {
    env.storage()
        .persistent()
        .get(&DataKey::Issuer(issuer.clone()))
}

/// Loads the full index of registered issuer addresses.
pub fn load_issuer_addrs(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::IssuerAddrs)
        .unwrap_or(Vec::new(env))
}

/// Loads every registered issuer record (active and inactive).
pub fn load_all_issuers(env: &Env) -> Vec<Issuer> {
    let addrs = load_issuer_addrs(env);
    let mut result = Vec::new(env);
    for addr in addrs.iter() {
        if let Some(issuer) = load_issuer(env, &addr) {
            result.push_back(issuer);
        }
    }
    result
}

// ---------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------

/// Returns whether `recipient` already holds a credential of `credential_type`.
pub fn credential_exists(env: &Env, recipient: &Address, credential_type: &String) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Cred(recipient.clone(), credential_type.clone()))
}

/// Persists a credential and indexes its type under the recipient.
pub fn save_credential(env: &Env, credential: &Credential) {
    let key = DataKey::Cred(
        credential.recipient.clone(),
        credential.credential_type.clone(),
    );
    env.storage().persistent().set(&key, credential);
    env.storage()
        .persistent()
        .extend_ttl(&key, LIFETIME_THRESHOLD, BUMP_AMOUNT);

    let mut types = load_credential_types(env, &credential.recipient);
    if !types.contains(&credential.credential_type) {
        types.push_back(credential.credential_type.clone());
        let types_key = DataKey::CredTypes(credential.recipient.clone());
        env.storage().persistent().set(&types_key, &types);
        env.storage()
            .persistent()
            .extend_ttl(&types_key, LIFETIME_THRESHOLD, BUMP_AMOUNT);
    }
}

/// Removes a credential and drops its type from the recipient's index.
pub fn remove_credential(env: &Env, recipient: &Address, credential_type: &String) {
    env.storage()
        .persistent()
        .remove(&DataKey::Cred(recipient.clone(), credential_type.clone()));

    let existing = load_credential_types(env, recipient);
    let mut remaining: Vec<String> = Vec::new(env);
    for t in existing.iter() {
        if t != *credential_type {
            remaining.push_back(t);
        }
    }
    let types_key = DataKey::CredTypes(recipient.clone());
    env.storage().persistent().set(&types_key, &remaining);
    env.storage()
        .persistent()
        .extend_ttl(&types_key, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

/// Loads a single credential, if it exists.
pub fn load_credential(
    env: &Env,
    recipient: &Address,
    credential_type: &String,
) -> Option<Credential> {
    env.storage()
        .persistent()
        .get(&DataKey::Cred(recipient.clone(), credential_type.clone()))
}

/// Loads the list of credential type names held by `recipient`.
pub fn load_credential_types(env: &Env, recipient: &Address) -> Vec<String> {
    env.storage()
        .persistent()
        .get(&DataKey::CredTypes(recipient.clone()))
        .unwrap_or(Vec::new(env))
}

/// Loads every credential currently held by `recipient`.
pub fn load_credentials(env: &Env, recipient: &Address) -> Vec<Credential> {
    let types = load_credential_types(env, recipient);
    let mut result = Vec::new(env);
    for t in types.iter() {
        if let Some(credential) = load_credential(env, recipient, &t) {
            result.push_back(credential);
        }
    }
    result
}

// ---------------------------------------------------------------------
// Weights
// ---------------------------------------------------------------------

/// Persists a point-weight override for a credential type.
pub fn save_weight(env: &Env, credential_type: &String, weight: u32) {
    let key = DataKey::Weight(credential_type.clone());
    env.storage().persistent().set(&key, &weight);
    env.storage()
        .persistent()
        .extend_ttl(&key, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

/// Loads the point-weight for a credential type, falling back to the
/// built-in default when no admin override has been set.
pub fn load_weight(env: &Env, credential_type: &String) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::Weight(credential_type.clone()))
        .unwrap_or_else(|| default_weight(env, credential_type))
}

/// The built-in point value for each of StellarCred's default credential
/// types. Unknown types default to zero.
pub fn default_weight(env: &Env, credential_type: &String) -> u32 {
    if *credential_type == String::from_str(env, "PaymentRecord") {
        10
    } else if *credential_type == String::from_str(env, "StreamCompleted") {
        20
    } else if *credential_type == String::from_str(env, "InvoiceCreator") {
        15
    } else if *credential_type == String::from_str(env, "WillOwner") {
        25
    } else if *credential_type == String::from_str(env, "DeveloperContrib") {
        30
    } else if *credential_type == String::from_str(env, "LongTermHolder") {
        50
    } else if *credential_type == String::from_str(env, "Verified") {
        100
    } else {
        0
    }
}

// ---------------------------------------------------------------------
// Leaderboard
// ---------------------------------------------------------------------

/// Loads the cached, sorted reputation leaderboard.
pub fn load_leaderboard(env: &Env) -> Vec<LeaderboardEntry> {
    env.storage()
        .persistent()
        .get(&DataKey::Leaderboard)
        .unwrap_or(Vec::new(env))
}

/// Upserts a wallet's score into the leaderboard and re-sorts it,
/// capping the tracked set at [`MAX_LEADERBOARD_SIZE`] entries.
pub fn update_leaderboard(env: &Env, address: &Address, score: u32, credential_count: u32) {
    let existing = load_leaderboard(env);
    let mut merged: Vec<LeaderboardEntry> = Vec::new(env);
    let mut found = false;

    for entry in existing.iter() {
        if entry.address == *address {
            merged.push_back(LeaderboardEntry {
                address: address.clone(),
                score,
                credential_count,
            });
            found = true;
        } else {
            merged.push_back(entry);
        }
    }
    if !found {
        merged.push_back(LeaderboardEntry {
            address: address.clone(),
            score,
            credential_count,
        });
    }

    let sorted = sort_leaderboard(env, merged);

    let mut capped: Vec<LeaderboardEntry> = Vec::new(env);
    for (i, entry) in sorted.iter().enumerate() {
        if i as u32 >= MAX_LEADERBOARD_SIZE {
            break;
        }
        capped.push_back(entry);
    }

    let key = DataKey::Leaderboard;
    env.storage().persistent().set(&key, &capped);
    env.storage()
        .persistent()
        .extend_ttl(&key, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

/// Stable insertion sort, descending by score.
fn sort_leaderboard(env: &Env, board: Vec<LeaderboardEntry>) -> Vec<LeaderboardEntry> {
    let mut sorted: Vec<LeaderboardEntry> = Vec::new(env);
    for entry in board.iter() {
        let mut inserted = false;
        let mut next: Vec<LeaderboardEntry> = Vec::new(env);
        for existing in sorted.iter() {
            if !inserted && entry.score > existing.score {
                next.push_back(entry.clone());
                inserted = true;
            }
            next.push_back(existing);
        }
        if !inserted {
            next.push_back(entry.clone());
        }
        sorted = next;
    }
    sorted
}

// ---------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------

/// Loads aggregate credential-issuance statistics.
pub fn load_stats(env: &Env) -> CredentialStats {
    let total_issued = env
        .storage()
        .persistent()
        .get(&DataKey::StatsTotal)
        .unwrap_or(0u32);
    let by_type: Map<String, u32> = env
        .storage()
        .persistent()
        .get(&DataKey::StatsByType)
        .unwrap_or(Map::new(env));
    CredentialStats {
        total_issued,
        by_type,
    }
}

fn save_stats(env: &Env, stats: &CredentialStats) {
    env.storage()
        .persistent()
        .set(&DataKey::StatsTotal, &stats.total_issued);
    env.storage()
        .persistent()
        .set(&DataKey::StatsByType, &stats.by_type);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::StatsTotal, LIFETIME_THRESHOLD, BUMP_AMOUNT);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::StatsByType, LIFETIME_THRESHOLD, BUMP_AMOUNT);
}

/// Increments the total-issued and per-type counters for `credential_type`.
pub fn increment_stats(env: &Env, credential_type: &String) {
    let mut stats = load_stats(env);
    stats.total_issued += 1;
    let current = stats.by_type.get(credential_type.clone()).unwrap_or(0);
    stats.by_type.set(credential_type.clone(), current + 1);
    save_stats(env, &stats);
}

/// Decrements the total-issued and per-type counters for `credential_type`,
/// used when a credential is revoked.
pub fn decrement_stats(env: &Env, credential_type: &String) {
    let mut stats = load_stats(env);
    if stats.total_issued > 0 {
        stats.total_issued -= 1;
    }
    let current = stats.by_type.get(credential_type.clone()).unwrap_or(0);
    if current > 0 {
        stats.by_type.set(credential_type.clone(), current - 1);
    }
    save_stats(env, &stats);
}
