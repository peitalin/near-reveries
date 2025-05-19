use super::*;
use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::testing_env;
use std::convert::TryFrom;

fn get_context(predecessor_account_id: AccountId, attached_deposit_yocto: u128) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(accounts(0)) // Contract account
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id)
        .attached_deposit(NearToken::from_yoctonear(attached_deposit_yocto));
    builder
}

fn new_contract(trusted_account: AccountId) -> PaymentContract {
    PaymentContract::new(trusted_account)
}

const TEST_REVERIE_ID: &str = "rev1";

#[test]
fn get_default_greeting() {
    let trusted = accounts(1);
    let contract = new_contract(trusted.clone());
    assert_eq!(contract.get_greeting(), "Hello");
}

#[test]
fn set_then_get_greeting() {
    let trusted = accounts(1);
    let mut contract = new_contract(trusted.clone());
    contract.set_greeting("howdy".to_string());
    assert_eq!(contract.get_greeting(), "howdy");
}

#[test]
fn deposit_and_get_balance() {
    let user = accounts(1);
    let trusted_account = accounts(2);
    let mut contract = new_contract(trusted_account.clone());

    testing_env!(get_context(user.clone(), 100).build());
    contract.deposit(TEST_REVERIE_ID.to_string());
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(100));

    testing_env!(get_context(user.clone(), 50).build());
    contract.deposit(TEST_REVERIE_ID.to_string());
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(150));
}

#[test]
fn get_balance_of_unknown_user() {
    let trusted_account = accounts(1);
    let contract = new_contract(trusted_account.clone());
    let unknown_user = AccountId::try_from("unknown.testnet".to_string()).unwrap();
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), unknown_user), U128(0));
}

#[test]
fn can_spend_sufficient_funds() {
    let user = accounts(1);
    let trusted_account = accounts(2);
    let mut contract = new_contract(trusted_account.clone());

    testing_env!(get_context(user.clone(), NearToken::from_near(100).as_yoctonear()).build());
    contract.deposit(TEST_REVERIE_ID.to_string());

    assert!(contract.can_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(NearToken::from_near(50).as_yoctonear())));
    assert!(contract.can_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(NearToken::from_near(100).as_yoctonear())));
}

#[test]
fn can_spend_insufficient_funds() {
    let user = accounts(1);
    let trusted_account = accounts(2);
    let mut contract = new_contract(trusted_account.clone());

    testing_env!(get_context(user.clone(), NearToken::from_near(100).as_yoctonear()).build());
    contract.deposit(TEST_REVERIE_ID.to_string());

    assert!(!contract.can_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(NearToken::from_near(101).as_yoctonear())));
}

#[test]
fn can_spend_zero_balance() {
    let user = accounts(1);
    let trusted_account = accounts(2);
    let contract = new_contract(trusted_account.clone());
    let unknown_user = AccountId::try_from("unknown.testnet".to_string()).unwrap();

    assert!(!contract.can_spend(TEST_REVERIE_ID.to_string(), unknown_user, U128(NearToken::from_near(1).as_yoctonear())));
}

#[test]
fn record_spend_success() {
    let user = accounts(1);
    let trusted_account = accounts(2);
    let mut contract = new_contract(trusted_account.clone());

    testing_env!(get_context(user.clone(), 100).build());
    contract.deposit(TEST_REVERIE_ID.to_string());
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(100));

    testing_env!(get_context(trusted_account.clone(), 0).build());
    contract.record_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(30));
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(70));
}

#[test]
#[should_panic(expected = "Only the trusted account can call this method")]
fn record_spend_unauthorized() {
    let user = accounts(1);
    let trusted_account = accounts(2);
    let unauthorized_caller = accounts(3);
    let mut contract = new_contract(trusted_account.clone());

    testing_env!(get_context(user.clone(), 100).build());
    contract.deposit(TEST_REVERIE_ID.to_string());

    testing_env!(get_context(unauthorized_caller.clone(), 0).build());
    contract.record_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(30));
}

#[test]
#[should_panic(expected = "Insufficient balance to record spend.")]
fn record_spend_insufficient_balance() {
    let user = accounts(1);
    let trusted_account = accounts(2);
    let mut contract = new_contract(trusted_account.clone());

    testing_env!(get_context(user.clone(), 20).build());
    contract.deposit(TEST_REVERIE_ID.to_string());

    testing_env!(get_context(trusted_account.clone(), 0).build());
    contract.record_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(30));
}

#[test]
fn test_get_and_update_trusted_account() {
    let initial_trusted_account = accounts(1);
    let mut contract = new_contract(initial_trusted_account.clone());
    assert_eq!(contract.get_trusted_account(), initial_trusted_account);

    let new_trusted_account = accounts(2);
    let contract_account = accounts(0); // Contract's own account

    testing_env!(get_context(contract_account.clone(), 0).build());
    contract.update_trusted_account(new_trusted_account.clone());
    assert_eq!(contract.get_trusted_account(), new_trusted_account);
}

#[test]
#[should_panic(expected = "Only the contract account can update the trusted account")]
fn test_update_trusted_account_unauthorized() {
    let initial_trusted_account = accounts(1);
    let mut contract = new_contract(initial_trusted_account.clone());

    let non_contract_account = accounts(3);
    let new_trusted_account = accounts(2);

    testing_env!(get_context(non_contract_account.clone(), 0).build());
    contract.update_trusted_account(new_trusted_account.clone());
}

#[test]
fn test_withdraw_successful() {
    let user = accounts(1);
    let trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());

    testing_env!(get_context(user.clone(), NearToken::from_near(10).as_yoctonear()).build());
    contract.deposit(TEST_REVERIE_ID.to_string());
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(NearToken::from_near(10).as_yoctonear()));

    testing_env!(get_context(user.clone(), 0).build());
    contract.withdraw(TEST_REVERIE_ID.to_string(), U128(NearToken::from_near(3).as_yoctonear()));
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(NearToken::from_near(7).as_yoctonear()));
}

#[test]
#[should_panic(expected = "Insufficient balance to withdraw.")]
fn test_withdraw_insufficient_funds() {
    let user = accounts(1);
    let trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());

    testing_env!(get_context(user.clone(), NearToken::from_near(5).as_yoctonear()).build());
    contract.deposit(TEST_REVERIE_ID.to_string());

    testing_env!(get_context(user.clone(), 0).build());
    contract.withdraw(TEST_REVERIE_ID.to_string(), U128(NearToken::from_near(10).as_yoctonear()));
}

#[test]
#[should_panic(expected = "Withdrawal amount must be greater than 0")]
fn test_withdraw_zero_amount() {
    let user = accounts(1);
    let trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());

    testing_env!(get_context(user.clone(), NearToken::from_near(5).as_yoctonear()).build());
    contract.deposit(TEST_REVERIE_ID.to_string());

    testing_env!(get_context(user.clone(), 0).build());
    contract.withdraw(TEST_REVERIE_ID.to_string(), U128(0));
}

#[test]
#[should_panic(expected = "Insufficient balance to withdraw.")]
fn test_withdraw_no_balance() {
    let user = accounts(1);
    let trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());

    testing_env!(get_context(user.clone(), 0).build());
    contract.withdraw(TEST_REVERIE_ID.to_string(), U128(NearToken::from_near(1).as_yoctonear()));
}

#[test]
fn test_withdraw_entire_balance() {
    let user = accounts(1);
    let trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());

    let initial_deposit = NearToken::from_near(5).as_yoctonear();
    testing_env!(get_context(user.clone(), initial_deposit).build());
    contract.deposit(TEST_REVERIE_ID.to_string());
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(initial_deposit));

    testing_env!(get_context(user.clone(), 0).build());
    contract.withdraw(TEST_REVERIE_ID.to_string(), U128(initial_deposit));
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(0));
    // Check that the user entry is removed from the inner map
    let user_balances = contract.balances.get(TEST_REVERIE_ID).unwrap();
    assert!(user_balances.get(&user).is_none(), "User entry should be removed from balances if balance is zero");
}

#[test]
fn test_can_spend_large_amount() {
    let user = accounts(1);
    let trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());

    let deposit_amount = NearToken::from_near(3).as_yoctonear();
    testing_env!(get_context(user.clone(), deposit_amount).build());
    contract.deposit(TEST_REVERIE_ID.to_string());
    assert_eq!(contract.get_balance(TEST_REVERIE_ID.to_string(), user.clone()), U128(deposit_amount));

    let large_spend_amount = "2400000000000000000000000".parse::<u128>().unwrap();
    let slightly_larger_spend_amount = "3100000000000000000000000".parse::<u128>().unwrap();

    assert!(contract.can_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(large_spend_amount)));
    assert!(!contract.can_spend(TEST_REVERIE_ID.to_string(), user.clone(), U128(slightly_larger_spend_amount)));
}

#[test]
fn test_create_reverie_success() {
    let trusted = accounts(1);
    let mut contract = new_contract(trusted.clone());
    testing_env!(get_context(trusted.clone(), 0).build());
    contract.create_reverie(
        TEST_REVERIE_ID.to_string(),
        "type1".to_string(),
        "desc1".to_string(),
        AccessCondition::Ed25519("pubkey1".to_string()),
    );
    let meta = contract.reverie_registry.get(TEST_REVERIE_ID).expect("Reverie should exist");
    assert_eq!(meta.reverie_type, "type1");
    assert_eq!(meta.description, "desc1");
    match &meta.access_condition {
        AccessCondition::Ed25519(pk) => assert_eq!(pk, "pubkey1"),
        _ => panic!("Wrong access condition variant"),
    }
}

#[test]
#[should_panic(expected = "Only the trusted account can create reveries")]
fn test_create_reverie_unauthorized() {
    let trusted = accounts(1);
    let not_trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());
    testing_env!(get_context(not_trusted.clone(), 0).build());
    contract.create_reverie(
        "unauth".to_string(),
        "type1".to_string(),
        "desc1".to_string(),
        AccessCondition::Ed25519("pubkey1".to_string()),
    );
}

#[test]
#[should_panic(expected = "ReverieId already exists")]
fn test_create_reverie_duplicate() {
    let trusted = accounts(1);
    let mut contract = new_contract(trusted.clone());
    testing_env!(get_context(trusted.clone(), 0).build());
    contract.create_reverie(
        "dup".to_string(),
        "type1".to_string(),
        "desc1".to_string(),
        AccessCondition::Ed25519("pubkey1".to_string()),
    );
    contract.create_reverie(
        "dup".to_string(),
        "type2".to_string(),
        "desc2".to_string(),
        AccessCondition::Ed25519("pubkey2".to_string()),
    );
}

#[test]
fn test_delete_all_reveries() {
    let trusted = accounts(1);
    let mut contract = new_contract(trusted.clone());
    testing_env!(get_context(trusted.clone(), 0).build());
    // Add two reveries
    contract.create_reverie(
        "r1".to_string(),
        "type1".to_string(),
        "desc1".to_string(),
        AccessCondition::Ed25519("pk1".to_string()),
    );
    contract.create_reverie(
        "r2".to_string(),
        "type2".to_string(),
        "desc2".to_string(),
        AccessCondition::Ecdsa("pk2".to_string()),
    );
    assert!(contract.reverie_registry.get("r1").is_some());
    assert!(contract.reverie_registry.get("r2").is_some());
    // Delete all
    contract.delete_all_reveries();
    assert!(contract.reverie_registry.get("r1").is_none());
    assert!(contract.reverie_registry.get("r2").is_none());
    assert!(contract.reverie_ids.is_empty());
}

#[test]
#[should_panic(expected = "Only the trusted account can delete all reveries")]
fn test_delete_all_reveries_unauthorized() {
    let trusted = accounts(1);
    let not_trusted = accounts(2);
    let mut contract = new_contract(trusted.clone());
    testing_env!(get_context(trusted.clone(), 0).build());
    contract.create_reverie(
        "r1".to_string(),
        "type1".to_string(),
        "desc1".to_string(),
        AccessCondition::Ed25519("pk1".to_string()),
    );
    testing_env!(get_context(not_trusted.clone(), 0).build());
    contract.delete_all_reveries();
}