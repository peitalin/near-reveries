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
use schemars::JsonSchema;

#[derive(JsonSchema, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
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

#[derive(JsonSchema, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SerializableAction {
    pub action_type: ActionType,
    // Specific fields for each action type
    // For FunctionCall
    pub receiver_id: Option<AccountId>,
    pub method_name: Option<String>,
    pub args: Option<Base64VecU8>, // JSON string of args, base64 encoded
    pub deposit: Option<U128>, // yoctoNEAR
    pub gas: Option<Gas>,
    // For Transfer
    pub amount: Option<U128>, // yoctoNEAR
    // For AddKey/DeleteKey
    pub public_key: Option<PublicKey>,
    // For AddKey (FunctionCallAccessKey)
    pub allowance: Option<U128>, // yoctoNEAR
    pub method_names: Option<Vec<String>>,
    // For DeployContract
    pub code: Option<Base64VecU8>,
    // For Stake
    pub stake: Option<U128>, // yoctoNEAR
    // For DeleteAccount
    pub beneficiary_id: Option<AccountId>,
}

impl SerializableAction {
    pub fn get_allowance(&self) -> Allowance {
        if let Some(amount) = &self.allowance {
            Allowance::Limited(NonZeroU128::new(amount.0).unwrap())
        } else {
            Allowance::Unlimited
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
            let promise_target_account_id = match action_data.action_type {
                ActionType::FunctionCall | ActionType::Transfer => {
                    action_data.receiver_id.clone().expect("Receiver ID for target account missing for FunctionCall/Transfer")
                }
                _ => env::current_account_id(), // For CreateAccount, DeployContract, Stake, AddKey, DeleteKey, DeleteAccount
            };

            let mut promise = Promise::new(promise_target_account_id.clone());

            match action_data.action_type {
                ActionType::CreateAccount => {
                    // CreateAccount is an action on a promise targeting env::current_account_id()
                    assert_eq!(promise_target_account_id, env::current_account_id(), "CreateAccount must be initiated by current_account_id");
                    promise = promise.create_account();
                }
                ActionType::DeployContract => {
                    assert_eq!(promise_target_account_id, env::current_account_id(), "DeployContract must be on self");
                    promise = promise.deploy_contract(action_data.code.expect("Code required for DeployContract").0);
                }
                ActionType::FunctionCall => {
                    promise = promise.function_call(
                        action_data.method_name.expect("Method name required for FunctionCall"),
                        action_data.args.expect("Args required for FunctionCall").0,
                        action_data.deposit.map_or(NearToken::from_yoctonear(0), |d| NearToken::from_yoctonear(d.0)),
                        action_data.gas.expect("Gas required for FunctionCall"),
                    );
                }
                ActionType::Transfer => {
                    promise = promise.transfer(NearToken::from_yoctonear(action_data.amount.expect("Amount required for Transfer").0));
                }
                ActionType::Stake => {
                    assert_eq!(promise_target_account_id, env::current_account_id(), "Stake must be on self");
                    promise = promise.stake(
                        NearToken::from_yoctonear(action_data.stake.expect("Stake amount required for Stake").0),
                        action_data.public_key.expect("Public key required for Stake"),
                    );
                }
                ActionType::AddKey => {
                    assert_eq!(promise_target_account_id, env::current_account_id(), "AddKey must be on self (promise target)");
                    promise = promise.add_access_key_allowance(
                        action_data.public_key.clone().expect("Public key required for AddKey"),
                        action_data.get_allowance(),
                        action_data.receiver_id.clone().expect("receiver_id for FunctionCallAccessKey permissions missing in AddKey action_data"),
                        action_data.method_names.unwrap_or_default().join(",")
                    );
                }
                ActionType::DeleteKey => {
                    assert_eq!(promise_target_account_id, env::current_account_id(), "DeleteKey must be on self");
                    promise = promise.delete_key(action_data.public_key.expect("Public key required for DeleteKey"));
                }
                ActionType::DeleteAccount => {
                    assert_eq!(promise_target_account_id, env::current_account_id(), "DeleteAccount must be on self");
                    promise = promise.delete_account(action_data.beneficiary_id.expect("Beneficiary ID required for DeleteAccount"));
                }
            }
            // The promise created above is implicitly added to the transaction's batch of actions.
            // No explicit dispatch or return of `promise` is needed here for it to be executed.
            log!("Action {:?} prepared for target {}", action_data.action_type, promise_target_account_id);
        }
    }
}