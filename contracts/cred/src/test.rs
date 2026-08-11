#![cfg(test)]

use crate::{CredContract, CredContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn types(env: &Env, items: &[&str]) -> Vec<String> {
    let mut v = Vec::new(env);
    for item in items {
        v.push_back(String::from_str(env, item));
    }
    v
}

fn setup(env: &Env) -> (CredContractClient<'_>, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register(CredContract, (admin.clone(),));
    let client = CredContractClient::new(env, &contract_id);
    (client, admin)
}

#[test]
fn test_register_issuer_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let name = String::from_str(&env, "PaySys");
    let cred_types = types(&env, &["PaymentRecord"]);

    client.register_issuer(&admin, &issuer, &name, &cred_types);

    let stored = client.get_issuer(&issuer);
    assert_eq!(stored.name, name);
    assert_eq!(stored.address, issuer);
    assert!(stored.active);
}

#[test]
fn test_issue_credential_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let cred_type = String::from_str(&env, "PaymentRecord");

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord"]),
    );
    client.issue_credential(
        &issuer,
        &recipient,
        &cred_type,
        &String::from_str(&env, "invoice #42"),
    );

    let creds = client.get_credentials(&recipient);
    assert_eq!(creds.len(), 1);
    let cred = creds.get(0).unwrap();
    assert_eq!(cred.credential_type, cred_type);
    assert_eq!(cred.issuer, issuer);
    assert_eq!(cred.recipient, recipient);
}

#[test]
fn test_credential_is_soulbound() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let stranger = Address::generate(&env);
    let cred_type = String::from_str(&env, "PaymentRecord");

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord"]),
    );
    client.issue_credential(
        &issuer,
        &recipient,
        &cred_type,
        &String::from_str(&env, "invoice #1"),
    );

    // The contract exposes no transfer entrypoint, so the credential can
    // only ever belong to the wallet it was minted to.
    assert!(client.has_credential(&recipient, &cred_type));
    assert!(!client.has_credential(&stranger, &cred_type));

    let creds = client.get_credentials(&recipient);
    assert_eq!(creds.len(), 1);
    assert_eq!(creds.get(0).unwrap().recipient, recipient);
}

#[test]
fn test_duplicate_credential_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let cred_type = String::from_str(&env, "PaymentRecord");

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord"]),
    );
    client.issue_credential(
        &issuer,
        &recipient,
        &cred_type,
        &String::from_str(&env, "first"),
    );

    let result = client.try_issue_credential(
        &issuer,
        &recipient,
        &cred_type,
        &String::from_str(&env, "second"),
    );
    assert!(result.is_err());
}

#[test]
fn test_score_calculation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord", "StreamCompleted", "Verified"]),
    );

    client.issue_credential(
        &issuer,
        &recipient,
        &String::from_str(&env, "PaymentRecord"),
        &String::from_str(&env, ""),
    );
    client.issue_credential(
        &issuer,
        &recipient,
        &String::from_str(&env, "StreamCompleted"),
        &String::from_str(&env, ""),
    );
    client.issue_credential(
        &issuer,
        &recipient,
        &String::from_str(&env, "Verified"),
        &String::from_str(&env, ""),
    );

    // PaymentRecord(10) + StreamCompleted(20) + Verified(100) = 130
    assert_eq!(client.get_score(&recipient), 130);
}

#[test]
fn test_has_credential_true_and_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let cred_type = String::from_str(&env, "PaymentRecord");

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord"]),
    );

    assert!(!client.has_credential(&recipient, &cred_type));

    client.issue_credential(&issuer, &recipient, &cred_type, &String::from_str(&env, ""));

    assert!(client.has_credential(&recipient, &cred_type));
    assert!(!client.has_credential(&recipient, &String::from_str(&env, "StreamCompleted")));
}

#[test]
fn test_revoke_credential() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let other_issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let cred_type = String::from_str(&env, "PaymentRecord");

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord"]),
    );
    client.register_issuer(
        &admin,
        &other_issuer,
        &String::from_str(&env, "Other"),
        &types(&env, &["PaymentRecord"]),
    );

    client.issue_credential(&issuer, &recipient, &cred_type, &String::from_str(&env, ""));

    let result = client.try_revoke_credential(&other_issuer, &recipient, &cred_type);
    assert!(result.is_err());
    assert!(client.has_credential(&recipient, &cred_type));

    client.revoke_credential(&issuer, &recipient, &cred_type);
    assert!(!client.has_credential(&recipient, &cred_type));
}

#[test]
fn test_unauthorized_issuer_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);

    let result = client.try_issue_credential(
        &issuer,
        &recipient,
        &String::from_str(&env, "PaymentRecord"),
        &String::from_str(&env, ""),
    );
    assert!(result.is_err());
}

#[test]
fn test_set_and_get_weight() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let cred_type = String::from_str(&env, "PaymentRecord");

    assert_eq!(client.get_credential_weight(&cred_type), 10);

    client.set_credential_weight(&admin, &cred_type, &500);
    assert_eq!(client.get_credential_weight(&cred_type), 500);

    let result = client.try_set_credential_weight(&admin, &cred_type, &5000);
    assert!(result.is_err());
}

#[test]
fn test_leaderboard_ordering() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let low = Address::generate(&env);
    let mid = Address::generate(&env);
    let high = Address::generate(&env);

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord", "StreamCompleted", "Verified"]),
    );

    client.issue_credential(
        &issuer,
        &low,
        &String::from_str(&env, "PaymentRecord"),
        &String::from_str(&env, ""),
    );
    client.issue_credential(
        &issuer,
        &mid,
        &String::from_str(&env, "StreamCompleted"),
        &String::from_str(&env, ""),
    );
    client.issue_credential(
        &issuer,
        &high,
        &String::from_str(&env, "Verified"),
        &String::from_str(&env, ""),
    );

    let board = client.get_leaderboard(&10);
    assert_eq!(board.len(), 3);
    assert_eq!(board.get(0).unwrap().address, high);
    assert_eq!(board.get(1).unwrap().address, mid);
    assert_eq!(board.get(2).unwrap().address, low);

    let top_one = client.get_leaderboard(&1);
    assert_eq!(top_one.len(), 1);
    assert_eq!(top_one.get(0).unwrap().address, high);
}

#[test]
fn test_get_all_issuers() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);

    client.register_issuer(
        &admin,
        &issuer_a,
        &String::from_str(&env, "A"),
        &types(&env, &["PaymentRecord"]),
    );
    client.register_issuer(
        &admin,
        &issuer_b,
        &String::from_str(&env, "B"),
        &types(&env, &["StreamCompleted"]),
    );

    let issuers = client.get_issuers();
    assert_eq!(issuers.len(), 2);
}

#[test]
fn test_remove_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.register_issuer(
        &admin,
        &issuer,
        &String::from_str(&env, "PaySys"),
        &types(&env, &["PaymentRecord"]),
    );
    client.remove_issuer(&admin, &issuer);

    let stored = client.get_issuer(&issuer);
    assert!(!stored.active);

    let result = client.try_issue_credential(
        &issuer,
        &recipient,
        &String::from_str(&env, "PaymentRecord"),
        &String::from_str(&env, ""),
    );
    assert!(result.is_err());
}
