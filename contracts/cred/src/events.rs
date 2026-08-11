use soroban_sdk::{symbol_short, Address, Env, String, Vec};

/// Emitted when a new issuer is registered.
pub fn issuer_registered(env: &Env, issuer: Address, name: String, credential_types: Vec<String>) {
    env.events()
        .publish((symbol_short!("iss_reg"), issuer), (name, credential_types));
}

/// Emitted when a credential is minted to a wallet.
pub fn credential_issued(
    env: &Env,
    recipient: Address,
    credential_type: String,
    issuer: Address,
    issued_at: u64,
) {
    env.events().publish(
        (symbol_short!("cred_iss"), recipient),
        (credential_type, issuer, issued_at),
    );
}

/// Emitted when a credential is revoked by its original issuer.
pub fn credential_revoked(env: &Env, recipient: Address, credential_type: String, issuer: Address) {
    env.events().publish(
        (symbol_short!("cred_rev"), recipient),
        (credential_type, issuer),
    );
}

/// Emitted when an admin updates a credential type's point weight.
pub fn weight_updated(env: &Env, credential_type: String, old_weight: u32, new_weight: u32) {
    env.events().publish(
        (symbol_short!("wt_upd"), credential_type),
        (old_weight, new_weight),
    );
}

/// Emitted when an issuer is deactivated.
pub fn issuer_removed(env: &Env, issuer: Address) {
    env.events().publish((symbol_short!("iss_rem"),), issuer);
}
