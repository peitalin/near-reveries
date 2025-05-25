#[cfg(test)]
mod tests_passkey_controller;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, near, log,
    AccountId, PanicOnDefault, PublicKey, Allowance,
    Promise, Gas, NearToken,
};
use near_sdk::json_types::{U128, Base64VecU8};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::store::IterableSet;
use std::num::NonZeroU128;
use schemars::JsonSchema;

// #[derive(JsonSchema, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, Clone)]
// #[serde(tag = "type", content = "value", crate = "near_sdk::serde")]
#[near_sdk::near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
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

// #[derive(JsonSchema, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
// #[serde(crate = "near_sdk::serde")]
#[near_sdk::near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct SerializableAction {
    pub action_type: ActionType,
    // Specific fields for each action type, now optional
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
    pub fn get_action_allowance(&self) -> Allowance {
        match self.allowance {
            Some(allowance_amount) if allowance_amount.0 > 0 => {
                Allowance::Limited(NonZeroU128::new(allowance_amount.0).unwrap_or_else(|| panic!("Allowance must be non-zero if limited")))
            }
            _ => Allowance::Unlimited, // Default to Unlimited if None or 0.
        }
    }
}

#[derive(JsonSchema, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "value", crate = "near_sdk::serde")]
pub enum AccessCondition {
    Umbral(String), // Use String for public key serialization
    Ecdsa(String), // Use String for address serialization
    Ed25519(String),
    Contract {
        address: String,
        access_function_name: String,
        access_function_args: String, // Store as JSON string
    },
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
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
        action_to_execute: SerializableAction,
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

        let action_data = action_to_execute;

        let promise_target_account_id = match action_data.action_type {
            ActionType::FunctionCall | ActionType::Transfer => {
                action_data.receiver_id.clone().unwrap_or_else(|| panic!("receiver_id is required for FunctionCall/Transfer"))
            }
            ActionType::CreateAccount => {
                 action_data.receiver_id.clone().unwrap_or_else(|| panic!("receiver_id is required for CreateAccount (as the new account_id)"))
            }
            ActionType::DeployContract | ActionType::Stake | ActionType::AddKey | ActionType::DeleteKey | ActionType::DeleteAccount => {
                env::current_account_id()
            }
        };

        let mut promise = Promise::new(promise_target_account_id.clone());

        match action_data.action_type {
            ActionType::CreateAccount => {
                // receiver_id (the new account) is already extracted for promise_target_account_id
                promise = promise.create_account();
            }
            ActionType::DeployContract => {
                promise = promise.deploy_contract(action_data.code.unwrap_or_else(|| Base64VecU8(vec![])).0);
            }
            ActionType::FunctionCall => {
                promise = promise.function_call(
                    action_data.method_name.unwrap_or_else(|| String::new()),
                    action_data.args.unwrap_or_else(|| Base64VecU8(vec![])).0,
                    NearToken::from_yoctonear(action_data.deposit.unwrap_or_else(|| U128(0)).0),
                    action_data.gas.unwrap_or_else(|| Gas::from_gas(0)),
                );
            }
            ActionType::Transfer => {
                promise = promise.transfer(NearToken::from_yoctonear(action_data.amount.unwrap_or_else(|| panic!("amount is required for Transfer")).0));
            }
            ActionType::Stake => {
                promise = promise.stake(
                    NearToken::from_yoctonear(action_data.stake.unwrap_or_else(|| panic!("stake amount is required for Stake")).0),
                    action_data.public_key.clone().unwrap_or_else(|| panic!("public_key is required for Stake")).clone(),
                );
            }
            ActionType::AddKey => {
                promise = promise.add_access_key_allowance(
                    action_data.public_key.clone().unwrap_or_else(|| panic!("public_key is required for AddKey")).clone(),
                    action_data.get_action_allowance(),
                    action_data.receiver_id.clone().unwrap_or_else(|| panic!("receiver_id for allowance scope is required for AddKey")),
                    action_data.method_names.unwrap_or_else(|| vec![]).join(",")
                );
            }
            ActionType::DeleteKey => {
                promise = promise.delete_key(action_data.public_key.clone().unwrap_or_else(|| panic!("public_key is required for DeleteKey")).clone());
            }
            ActionType::DeleteAccount => {
                promise = promise.delete_account(action_data.beneficiary_id.unwrap_or_else(|| panic!("beneficiary_id is required for DeleteAccount")));
            }
        }
        log!("Action {:?} prepared for target {}", action_data.action_type, promise_target_account_id);
    }
}