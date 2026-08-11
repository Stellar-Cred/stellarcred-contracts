use soroban_sdk::contracterror;

/// Errors returned by the StellarCred credential contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Caller does not match the stored contract admin.
    NotAdmin = 1,
    /// No issuer is registered at the given address.
    IssuerNotFound = 2,
    /// An issuer is already registered at the given address.
    IssuerAlreadyExists = 3,
    /// The address is not an active, registered issuer.
    NotAuthorizedIssuer = 4,
    /// The recipient already holds a credential of this type.
    CredentialAlreadyExists = 5,
    /// No credential of this type exists for the recipient.
    CredentialNotFound = 6,
    /// The issuer is not authorized to mint this credential type.
    InvalidCredentialType = 7,
    /// Only the original issuing address may revoke this credential.
    NotOriginalIssuer = 8,
    /// The weight value is outside the allowed range.
    InvalidWeight = 9,
}
