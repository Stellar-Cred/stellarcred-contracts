use soroban_sdk::{contracttype, Address, Map, String, Vec};

/// A single soul-bound credential issued to a wallet address.
///
/// Credentials are permanent once issued: the contract exposes no transfer
/// entrypoint, so a credential can only ever be read, revoked by its
/// original issuer, or left in place.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credential {
    pub recipient: Address,
    pub credential_type: String,
    pub issuer: Address,
    pub issued_at: u64,
    pub metadata: String,
}

/// A protocol authorized to mint one or more credential types.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Issuer {
    pub address: Address,
    pub name: String,
    pub credential_types: Vec<String>,
    pub registered_at: u64,
    pub active: bool,
}

/// A single row on the reputation leaderboard.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaderboardEntry {
    pub address: Address,
    pub score: u32,
    pub credential_count: u32,
}

/// Aggregate counts of credentials issued across the protocol.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialStats {
    pub total_issued: u32,
    pub by_type: Map<String, u32>,
}

/// Internal contract storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// The contract admin address, set once at construction.
    Admin,
    /// The full list of registered issuer addresses.
    IssuerAddrs,
    /// An [`Issuer`] record, keyed by issuer address.
    Issuer(Address),
    /// The list of credential types held by a wallet, keyed by recipient.
    CredTypes(Address),
    /// A [`Credential`] record, keyed by (recipient, credential_type).
    Cred(Address, String),
    /// A point-weight override for a credential type.
    Weight(String),
    /// The cached, sorted reputation leaderboard.
    Leaderboard,
    /// Total number of credentials ever issued (net of revocations).
    StatsTotal,
    /// Per-type breakdown of credentials currently issued.
    StatsByType,
}
