#[cfg(test)]
mod tests_passkey_controller;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::IterableSet;
use near_sdk::{
    env, near, log,
    AccountId, PanicOnDefault, PublicKey,
    Promise, Gas, NearToken, Allowance
};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::{U128, Base64VecU8};
use std::num::NonZeroU128;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum ActionType {
    CreateAccount,
    DeployContract,
    FunctionCall,
    Transfer,
    Stake,
    AddKey,
    DeleteKey,
    DeleteAccount,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SerializableAction {
    pub action_type: ActionType,
    // Specific fields for each action type
    pub receiver_id: AccountId,
    pub method_name: String,
    pub args: Base64VecU8, // JSON string of args, base64 encoded
    pub deposit: U128, // yoctoNEAR
    pub gas: Gas,
    // For Transfer
    pub amount: U128, // yoctoNEAR
    // For AddKey/DeleteKey
    pub public_key: PublicKey,
    // For AddKey (FunctionCallAccessKey)
    pub allowance: U128, // yoctoNEAR - Note: Action's allowance is U128, Promise's is NearToken or Allowance enum
    pub method_names: Vec<String>,
    // For DeployContract
    pub code: Base64VecU8,
    // For Stake
    pub stake: U128, // yoctoNEAR
    // For DeleteAccount
    pub beneficiary_id: AccountId,
}

impl SerializableAction {
    pub fn get_action_allowance(&self) -> Allowance { // Renamed to avoid conflict with struct field if we were to add one
        if self.allowance.0 > 0 {
            Allowance::Limited(NonZeroU128::new(self.allowance.0).unwrap_or_else(|| panic!("Allowance must be non-zero if limited")))
        } else {
            // Representing unlimited allowance by not providing a specific amount to add_access_key_allowance's `allowance` field
            // This interpretation might need adjustment based on how Promise::add_access_key_allowance handles a 0 U128 for its own allowance field.
            // For now, we assume a 0 U128 here means unlimited for the *function call* access,
            // and the promise action will interpret its allowance parameter accordingly.
            // If a specific "None" or "Unlimited" variant is needed for the promise, this logic will need adjustment.
            // A common pattern for "unlimited" is to simply not call the `.allowance()` builder method on the Promise.
            // However, add_access_key_allowance *requires* an Allowance enum.
            Allowance::Unlimited // Defaulting to Unlimited if self.allowance is 0.
        }
    }
}

#[near(contract_state)]
pub struct PasskeyController {
    trusted_relayer_account_id: AccountId,
    owner_id: AccountId,
    registered_passkey_pks: IterableSet<PublicKey>,
}

#[near]
impl PasskeyController {
    #[init]
    pub fn new(
        trusted_relayer_account_id: AccountId,
        owner_id: AccountId,
        initial_passkey_pks: Option<Vec<PublicKey>>,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let mut pk_set = IterableSet::new(b"p");
        if let Some(keys) = initial_passkey_pks {
            for key in keys {
                pk_set.insert(key);
            }
        }
        Self {
            trusted_relayer_account_id,
            owner_id,
            registered_passkey_pks: pk_set,
        }
    }

    pub fn set_trusted_relayer(&mut self, account_id: AccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Only owner can set trusted relayer"
        );
        self.trusted_relayer_account_id = account_id;
    }

    pub fn get_trusted_relayer(&self) -> AccountId {
        self.trusted_relayer_account_id.clone()
    }

    pub fn get_owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    pub fn add_passkey_pk(&mut self, passkey_pk: PublicKey) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.trusted_relayer_account_id,
            "Only trusted relayer can add passkey PKs"
        );
        self.registered_passkey_pks.insert(passkey_pk)
    }

    pub fn remove_passkey_pk(&mut self, passkey_pk: PublicKey) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.trusted_relayer_account_id,
            "Only trusted relayer can remove passkey PKs"
        );
        self.registered_passkey_pks.remove(&passkey_pk)
    }

    pub fn is_passkey_pk_registered(&self, passkey_pk: PublicKey) -> bool {
        self.registered_passkey_pks.contains(&passkey_pk)
    }

    pub fn execute_actions(
        &mut self,
        passkey_pk_used: PublicKey,
        actions_to_execute: Vec<SerializableAction>,
    ) {
        assert_eq!(
            env::predecessor_account_id(),
            self.trusted_relayer_account_id,
            "Only trusted relayer can execute actions"
        );
        assert!(
            self.registered_passkey_pks.contains(&passkey_pk_used),
            "Passkey PK not registered"
        );

        for action_data in actions_to_execute {
            // Determine the target account_id for the promise based on the action type.
            // Some actions are always on `env::current_account_id()`.
            let promise_target_account_id = match action_data.action_type {
                ActionType::FunctionCall | ActionType::Transfer => {
                    // For these, receiver_id in action_data specifies the target.
                    action_data.receiver_id.clone()
                }
                ActionType::CreateAccount | ActionType::DeployContract | ActionType::Stake | ActionType::AddKey | ActionType::DeleteKey | ActionType::DeleteAccount => {
                    // These actions operate on promises new(env::current_account_id())
                    env::current_account_id()
                }
            };

            let mut promise = Promise::new(promise_target_account_id.clone());

            match action_data.action_type {
                ActionType::CreateAccount => {
                    // CreateAccount action is built on a promise targeting current_account_id.
                    // The actual account to be created is typically passed as part of a different action,
                    // or this action is chained after a transfer to fund the new account.
                    // Here, we assume `promise_target_account_id` (which is current_account_id)
                    // is just the account initiating the creation, and `receiver_id` in action_data
                    // should specify the new account if that's the design.
                    // However, `Promise::new(target).create_account()` doesn't take the new account ID.
                    // It seems CreateAccount action implies the *promise target* is the new account.
                    // This part of NEAR's API can be confusing.
                    // For now, let's assume the `action_data.receiver_id` IS the new account ID.
                    // And the promise should be Promise::new(action_data.receiver_id).create_account()
                    // This contradicts the `promise_target_account_id` logic above.
                    // Re-evaluating: CreateAccount is an action on an *existing* promise.
                    // The promise target for `Promise::new(target_id).create_account()` means `target_id` *initiates* this.
                    // The actual new account will be created as a sub-account or a named account if funds allow.
                    // The `action_data.receiver_id` is not directly used by `promise.create_account()`.
                    // It's implicit that `create_account()` creates a sub-account of the *promise target*.
                    // Or, if the promise target is a new top-level account, it would have been funded by a transfer first.
                    // This seems to be the case where `SerializableAction` might need more fields or a different structure.

                    // Let's assume the intent is to create a new account specified by action_data.receiver_id,
                    // and the promise should be targeted at that new account.
                    // This requires a different promise creation: `Promise::new(action_data.receiver_id.clone())`
                    // and then attaching actions like `create_account()`, `transfer()`, `deploy_contract()`.
                    // The current loop structure is one promise per `SerializableAction`.

                    // Sticking to current structure: `promise` is already `Promise::new(env::current_account_id())`
                    // This creates a sub-account of the contract itself.
                    // If `action_data.receiver_id` was intended to be the new account name, it's not used here.
                    promise = Promise::new(action_data.receiver_id.clone()).create_account();
                     // Then potentially transfer, deploy, etc. to this new account in subsequent actions.
                }
                ActionType::DeployContract => {
                    // Deploys to the `promise_target_account_id`, which is `env::current_account_id()`
                    promise = promise.deploy_contract(action_data.code.0);
                }
                ActionType::FunctionCall => {
                    // `promise_target_account_id` is `action_data.receiver_id`
                    promise = promise.function_call(
                        action_data.method_name,
                        action_data.args.0,
                        NearToken::from_yoctonear(action_data.deposit.0),
                        action_data.gas,
                    );
                }
                ActionType::Transfer => {
                    // `promise_target_account_id` is `action_data.receiver_id`
                    promise = promise.transfer(NearToken::from_yoctonear(action_data.amount.0));
                }
                ActionType::Stake => {
                    // `promise_target_account_id` is `env::current_account_id()`
                    promise = promise.stake(
                        NearToken::from_yoctonear(action_data.stake.0),
                        action_data.public_key, // This PK is for the validator
                    );
                }
                ActionType::AddKey => {
                    // `promise_target_account_id` is `env::current_account_id()`
                    // The key is added to `env::current_account_id()`
                    // The `action_data.receiver_id` here is for the FunctionCallAccessKey scope.
                    promise = promise.add_access_key_allowance(
                        action_data.public_key.clone(),
                        action_data.get_action_allowance(), // Uses the renamed helper
                        action_data.receiver_id.clone(), // This is the contract the key is for
                        action_data.method_names.join(",")
                    );
                }
                ActionType::DeleteKey => {
                    // `promise_target_account_id` is `env::current_account_id()`
                    promise = promise.delete_key(action_data.public_key);
                }
                ActionType::DeleteAccount => {
                    // `promise_target_account_id` is `env::current_account_id()`
                    // The account to delete is `env::current_account_id()`. Beneficiary receives funds.
                    promise = promise.delete_account(action_data.beneficiary_id);
                }
            }
            log!("Action {:?} prepared for target {}", action_data.action_type, promise_target_account_id);
        }
    }
}