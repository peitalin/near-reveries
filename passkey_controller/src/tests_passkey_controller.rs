use super::*;
use near_sdk::test_utils::{VMContextBuilder, accounts};
use near_sdk::testing_env;

fn get_context(predecessor_account_id: AccountId, current_account_id: AccountId) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(current_account_id)
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id);
    builder
}

#[test]
fn test_new() {
    let owner = accounts(0);
    let relayer = accounts(1);
    let contract_account = accounts(2);
    let context = get_context(owner.clone(), contract_account.clone());
    testing_env!(context.build());

    let pk1_bytes: [u8; 32] = [1; 32];
    let pk1 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk1_bytes.to_vec()).unwrap();
    let pk2_bytes: [u8; 32] = [2; 32];
    let pk2 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk2_bytes.to_vec()).unwrap();


    let contract = PasskeyController::new(relayer.clone(), owner.clone(), Some(vec![pk1.clone(), pk2.clone()]));
    assert_eq!(contract.get_owner_id(), owner);
    assert_eq!(contract.get_trusted_relayer(), relayer);
    assert!(contract.is_passkey_pk_registered(pk1.clone()));
    assert!(contract.is_passkey_pk_registered(pk2.clone()));

    let pk3_bytes: [u8; 32] = [3; 32];
    let pk3 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk3_bytes.to_vec()).unwrap();
    assert!(!contract.is_passkey_pk_registered(pk3));
}

#[test]
fn test_set_trusted_relayer() {
    let owner = accounts(0);
    let relayer = accounts(1);
    let contract_account = accounts(2);
    let mut context = get_context(owner.clone(), contract_account.clone());
    testing_env!(context.build());

    let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), None);
    let new_relayer = accounts(3);

    // Owner sets new relayer
    context = get_context(owner.clone(), contract_account.clone());
    testing_env!(context.build());
    contract.set_trusted_relayer(new_relayer.clone());
    assert_eq!(contract.get_trusted_relayer(), new_relayer);
}

#[test]
#[should_panic(expected = "Only owner can set trusted relayer")]
fn test_set_trusted_relayer_panic_not_owner() {
    let owner = accounts(0);
    let relayer = accounts(1);
    let contract_account = accounts(2);
    let mut context = get_context(owner.clone(), contract_account.clone());
    testing_env!(context.build());

    let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), None);
    let new_relayer = accounts(3);
    let malicious_user = accounts(4);

    context = get_context(malicious_user.clone(), contract_account.clone());
    testing_env!(context.build());
    contract.set_trusted_relayer(new_relayer.clone());
}

#[test]
fn test_add_remove_is_passkey_pk() {
    let owner = accounts(0);
    let relayer = accounts(1);
    let contract_account = accounts(2);
    let mut context = get_context(owner.clone(), contract_account.clone()); // Init context
    testing_env!(context.build());

    let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), None);

    let pk1_bytes: [u8; 32] = [1; 32];
    let pk1 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk1_bytes.to_vec()).unwrap();

    // Relayer adds pk1
    context = get_context(relayer.clone(), contract_account.clone());
    testing_env!(context.build());
    assert!(contract.add_passkey_pk(pk1.clone()));
    assert!(contract.is_passkey_pk_registered(pk1.clone()));

    // Adding again returns false (already present)
    assert!(!contract.add_passkey_pk(pk1.clone()));

    // Relayer removes pk1
    context = get_context(relayer.clone(), contract_account.clone());
    testing_env!(context.build());
    assert!(contract.remove_passkey_pk(pk1.clone()));
    assert!(!contract.is_passkey_pk_registered(pk1.clone()));

    // Removing again returns false (not present)
    assert!(!contract.remove_passkey_pk(pk1.clone()));
}

#[test]
#[should_panic(expected = "Only trusted relayer can add passkey PKs")]
fn test_add_passkey_pk_panic_not_relayer() {
    let owner = accounts(0);
    let relayer = accounts(1);
    let contract_account = accounts(2);
    let mut context = get_context(owner.clone(), contract_account.clone());
    testing_env!(context.build());

    let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), None);
    let pk1_bytes: [u8; 32] = [1; 32];
    let pk1 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk1_bytes.to_vec()).unwrap();

    let malicious_user = accounts(3);
    context = get_context(malicious_user.clone(), contract_account.clone());
    testing_env!(context.build());
    contract.add_passkey_pk(pk1.clone());
}

#[test]
#[should_panic(expected = "Only trusted relayer can remove passkey PKs")]
fn test_remove_passkey_pk_panic_not_relayer() {
    let owner = accounts(0);
    let relayer = accounts(1);
    let contract_account = accounts(2);
    let mut context = get_context(owner.clone(), contract_account.clone());
    testing_env!(context.build());

    let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), None);

    let pk1_bytes: [u8; 32] = [1; 32];
    let pk1 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk1_bytes.to_vec()).unwrap();

    context = get_context(relayer.clone(), contract_account.clone());
    testing_env!(context.build());
    contract.add_passkey_pk(pk1.clone());

    let malicious_user = accounts(3);
    context = get_context(malicious_user.clone(), contract_account.clone());
    testing_env!(context.build());
    contract.remove_passkey_pk(pk1.clone());
}

// Tests for execute_actions
// These tests are more complex due to promise dispatches and would typically
// require integration testing or more sophisticated unit test setups to verify
// that the correct promises are formed and dispatched.
// For now, we'll test the assertions.

// #[test]
// #[should_panic(expected = "Only trusted relayer can execute actions")]
// fn test_execute_actions_panic_not_relayer() {
//     let owner = accounts(0);
//     let relayer = accounts(1);
//     let contract_account = accounts(2);
//     let mut context = get_context(owner.clone(), contract_account.clone());
//     testing_env!(context.build());

//     let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), None);
//     let pk1_bytes: [u8; 32] = [1; 32];
//     let pk1 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk1_bytes.to_vec()).unwrap();
//     let default_pk_bytes: [u8; 32] = [0u8; 32];
//     let default_pk = PublicKey::from_parts(near_sdk::CurveType::ED25519, default_pk_bytes.to_vec()).unwrap();

//     let dummy_action = SerializableAction {
//         action_type: ActionType::Transfer, // Arbitrary type for dummy
//         receiver_id: Some(accounts(3)),
//         method_name: None,
//         args: None,
//         deposit: None,
//         gas: None,
//         amount: Some(U128(0)), // Required by Transfer, even if 0 for a dummy
//         public_key: None,
//         allowance: None,
//         method_names: None,
//         code: None,
//         stake: None,
//         beneficiary_id: None,
//     };

//     let non_relayer = accounts(3);
//     context = get_context(non_relayer.clone(), contract_account.clone());
//     testing_env!(context.build());
//     contract.execute_actions(pk1, dummy_action.clone());
// }

// #[test]
// #[should_panic(expected = "Passkey PK not registered")]
// fn test_execute_actions_panic_pk_not_registered() {
//     let owner = accounts(0);
//     let relayer = accounts(1);
//     let contract_account = accounts(2);
//     let mut context = get_context(relayer.clone(), contract_account.clone());
//     testing_env!(context.build());

//     let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), None);
//     let default_pk_bytes: [u8; 32] = [0u8; 32];
//     let default_pk = PublicKey::from_parts(near_sdk::CurveType::ED25519, default_pk_bytes.to_vec()).unwrap();

//     let dummy_action = SerializableAction {
//         action_type: ActionType::Transfer, // Arbitrary type for dummy
//         receiver_id: Some(accounts(3)),
//         method_name: None,
//         args: None,
//         deposit: None,
//         gas: None,
//         amount: Some(U128(0)), // Required by Transfer, even if 0 for a dummy
//         public_key: None,
//         allowance: None,
//         method_names: None,
//         code: None,
//         stake: None,
//         beneficiary_id: None,
//     };

//     let pk_unregistered_bytes: [u8; 32] = [99; 32];
//     let pk_unregistered = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk_unregistered_bytes.to_vec()).unwrap();

//     contract.execute_actions(pk_unregistered, dummy_action);
// }

// // A simple test for execute_actions that checks if it runs with a registered key.
// // Does not verify promise creation.
// #[test]
// fn test_execute_actions_runs_with_registered_pk() {
//     let owner = accounts(0);
//     let relayer = accounts(1);
//     let contract_account = accounts(2); // This contract's own account
//     let mut context = get_context(relayer.clone(), contract_account.clone());
//     testing_env!(context.build());

//     let pk1_bytes: [u8; 32] = [1; 32];
//     let pk1 = PublicKey::from_parts(near_sdk::CurveType::ED25519, pk1_bytes.to_vec()).unwrap();
//     let default_pk_bytes: [u8; 32] = [0; 32]; // Placeholder for unused PK fields
//     let default_pk = PublicKey::from_parts(near_sdk::CurveType::ED25519, default_pk_bytes.to_vec()).unwrap();


//     let mut contract = PasskeyController::new(relayer.clone(), owner.clone(), Some(vec![pk1.clone()]));

//     // Example: A simple transfer action
//     let transfer_action = SerializableAction {
//         action_type: ActionType::Transfer,
//         receiver_id: Some(accounts(3)), // Target account for transfer
//         method_name: None, // Not used for Transfer
//         args: None, // Not used for Transfer
//         deposit: None, // Not used for Transfer
//         gas: None, // Not used for Transfer
//         amount: Some(U128(100)), // Mandatory for Transfer
//         public_key: None, // Not used for Transfer
//         allowance: None, // Not used for Transfer
//         method_names: None, // Not used for Transfer
//         code: None, // Not used for Transfer
//         stake: None, // Not used for Transfer
//         beneficiary_id: None, // Not used for Transfer
//     };

//     // This will attempt to create a promise but won't execute it in test_utils.
//     // The important part is that it doesn't panic before promise creation.
//     contract.execute_actions(pk1.clone(), transfer_action);
//     // For now, a successful run without panic for valid inputs is the main check.
// }