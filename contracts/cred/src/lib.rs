#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

pub use errors::Error;
pub use types::{Credential, CredentialStats, Issuer, LeaderboardEntry};

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

/// StellarCred: an on-chain behavioral reputation and credential registry
/// for the Stellar network.
///
/// Registered protocols ("issuers") mint soul-bound credentials to wallet
/// addresses based on verified on-chain activity. The contract aggregates
/// a wallet's credentials into a 0-1000 reputation score that any Stellar
/// dApp can query directly, with no off-chain systems or KYC involved.
#[contract]
pub struct CredContract;

#[contractimpl]
impl CredContract {
    /// Initializes the contract with its admin address. Runs once, at
    /// deployment.
    pub fn __constructor(env: Env, admin: Address) {
        storage::set_admin(&env, &admin);
    }

    /// Registers a new authorized credential issuer. Only the contract
    /// admin may call this.
    ///
    /// # Arguments
    /// * `admin` - Must match the stored contract admin.
    /// * `issuer` - The address that will be authorized to issue credentials.
    /// * `name` - A human-readable name for the issuing protocol.
    /// * `credential_types` - The credential types this issuer may mint.
    ///
    /// # Errors
    /// * [`Error::NotAdmin`] if `admin` does not match the stored admin.
    /// * [`Error::IssuerAlreadyExists`] if `issuer` is already registered.
    pub fn register_issuer(
        env: Env,
        admin: Address,
        issuer: Address,
        name: String,
        credential_types: Vec<String>,
    ) -> Result<(), Error> {
        storage::require_admin(&env, &admin)?;

        if storage::issuer_exists(&env, &issuer) {
            return Err(Error::IssuerAlreadyExists);
        }

        let record = types::Issuer {
            address: issuer.clone(),
            name: name.clone(),
            credential_types: credential_types.clone(),
            registered_at: env.ledger().timestamp(),
            active: true,
        };
        storage::save_issuer(&env, &record);
        events::issuer_registered(&env, issuer, name, credential_types);
        Ok(())
    }

    /// Deactivates a registered issuer. Credentials it already issued
    /// remain valid and queryable; the issuer simply can no longer mint
    /// new ones until re-registered by the admin.
    ///
    /// # Errors
    /// * [`Error::NotAdmin`] if `admin` does not match the stored admin.
    /// * [`Error::IssuerNotFound`] if no issuer is registered at `issuer`.
    pub fn remove_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        storage::require_admin(&env, &admin)?;

        let mut record = storage::load_issuer(&env, &issuer).ok_or(Error::IssuerNotFound)?;
        record.active = false;
        storage::save_issuer(&env, &record);
        events::issuer_removed(&env, issuer);
        Ok(())
    }

    /// Issues a soul-bound credential of `credential_type` to `recipient`.
    /// The credential can never be transferred; there is no entrypoint to
    /// move it to another wallet.
    ///
    /// # Errors
    /// * [`Error::NotAuthorizedIssuer`] if `issuer` is not an active,
    ///   registered issuer.
    /// * [`Error::InvalidCredentialType`] if `issuer` is not authorized to
    ///   mint `credential_type`.
    /// * [`Error::CredentialAlreadyExists`] if `recipient` already holds a
    ///   credential of this type.
    pub fn issue_credential(
        env: Env,
        issuer: Address,
        recipient: Address,
        credential_type: String,
        metadata: String,
    ) -> Result<(), Error> {
        issuer.require_auth();

        let record = storage::load_issuer(&env, &issuer).ok_or(Error::NotAuthorizedIssuer)?;
        if !record.active {
            return Err(Error::NotAuthorizedIssuer);
        }
        if !record.credential_types.contains(&credential_type) {
            return Err(Error::InvalidCredentialType);
        }
        if storage::credential_exists(&env, &recipient, &credential_type) {
            return Err(Error::CredentialAlreadyExists);
        }

        let issued_at = env.ledger().timestamp();
        let credential = types::Credential {
            recipient: recipient.clone(),
            credential_type: credential_type.clone(),
            issuer: issuer.clone(),
            issued_at,
            metadata,
        };
        storage::save_credential(&env, &credential);
        storage::increment_stats(&env, &credential_type);

        let score = Self::get_score(env.clone(), recipient.clone());
        let count = storage::load_credential_types(&env, &recipient).len();
        storage::update_leaderboard(&env, &recipient, score, count);

        events::credential_issued(&env, recipient, credential_type, issuer, issued_at);
        Ok(())
    }

    /// Revokes a credential. Only the address that originally issued it
    /// may revoke it.
    ///
    /// # Errors
    /// * [`Error::CredentialNotFound`] if no such credential exists.
    /// * [`Error::NotOriginalIssuer`] if `issuer` did not issue it.
    pub fn revoke_credential(
        env: Env,
        issuer: Address,
        recipient: Address,
        credential_type: String,
    ) -> Result<(), Error> {
        issuer.require_auth();

        let credential = storage::load_credential(&env, &recipient, &credential_type)
            .ok_or(Error::CredentialNotFound)?;
        if credential.issuer != issuer {
            return Err(Error::NotOriginalIssuer);
        }

        storage::remove_credential(&env, &recipient, &credential_type);
        storage::decrement_stats(&env, &credential_type);

        let score = Self::get_score(env.clone(), recipient.clone());
        let count = storage::load_credential_types(&env, &recipient).len();
        storage::update_leaderboard(&env, &recipient, score, count);

        events::credential_revoked(&env, recipient, credential_type, issuer);
        Ok(())
    }

    /// Returns every credential currently held by `address`.
    pub fn get_credentials(env: Env, address: Address) -> Vec<types::Credential> {
        storage::load_credentials(&env, &address)
    }

    /// Calculates a wallet's reputation score from the sum of its
    /// credential weights, capped at 1000.
    pub fn get_score(env: Env, address: Address) -> u32 {
        let credentials = storage::load_credentials(&env, &address);
        let mut total: u32 = 0;
        for credential in credentials.iter() {
            total += storage::load_weight(&env, &credential.credential_type);
        }
        total.min(1000)
    }

    /// Returns whether `address` currently holds a credential of
    /// `credential_type`.
    pub fn has_credential(env: Env, address: Address, credential_type: String) -> bool {
        storage::credential_exists(&env, &address, &credential_type)
    }

    /// Lists every registered issuer, active and inactive.
    pub fn get_issuers(env: Env) -> Vec<types::Issuer> {
        storage::load_all_issuers(&env)
    }

    /// Returns the registration record for a single issuer.
    ///
    /// # Errors
    /// * [`Error::IssuerNotFound`] if no issuer is registered at `issuer`.
    pub fn get_issuer(env: Env, issuer: Address) -> Result<types::Issuer, Error> {
        storage::load_issuer(&env, &issuer).ok_or(Error::IssuerNotFound)
    }

    /// Sets the point value awarded for holding a credential of
    /// `credential_type`. Admin-only.
    ///
    /// # Errors
    /// * [`Error::NotAdmin`] if `admin` does not match the stored admin.
    /// * [`Error::InvalidWeight`] if `weight` exceeds 1000.
    pub fn set_credential_weight(
        env: Env,
        admin: Address,
        credential_type: String,
        weight: u32,
    ) -> Result<(), Error> {
        storage::require_admin(&env, &admin)?;
        if weight > 1000 {
            return Err(Error::InvalidWeight);
        }

        let old_weight = storage::load_weight(&env, &credential_type);
        storage::save_weight(&env, &credential_type, weight);
        events::weight_updated(&env, credential_type, old_weight, weight);
        Ok(())
    }

    /// Returns the point value for a credential type, falling back to the
    /// built-in default when no admin override has been set.
    pub fn get_credential_weight(env: Env, credential_type: String) -> u32 {
        storage::load_weight(&env, &credential_type)
    }

    /// Returns the top `limit` wallets by reputation score, highest first.
    pub fn get_leaderboard(env: Env, limit: u32) -> Vec<types::LeaderboardEntry> {
        let board = storage::load_leaderboard(&env);
        let mut result = Vec::new(&env);
        for (i, entry) in board.iter().enumerate() {
            if i as u32 >= limit {
                break;
            }
            result.push_back(entry);
        }
        result
    }

    /// Returns aggregate counts of credentials issued, in total and per
    /// credential type.
    pub fn get_credential_stats(env: Env) -> types::CredentialStats {
        storage::load_stats(&env)
    }
}
