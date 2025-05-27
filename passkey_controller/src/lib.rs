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
    // For CreateAccount
    pub initial_deposit_for_new_account: Option<U128>, // yoctoNEAR
    pub public_key_for_new_account: Option<PublicKey>,
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

    pub fn execute_direct_actions(
        &mut self,
        action_to_execute: SerializableAction,
    ) {
        let signer_pk = env::signer_account_pk();
        assert!(
            self.registered_passkey_pks.contains(&signer_pk),
            "ERR_SIGNER_PK_NOT_REGISTERED_AS_PASSKEY"
        );

        let signer_account_id = env::signer_account_id(); // This is Derp's account
        log!(
            "Direct action initiated by: {} using PK: {:?}",
            signer_account_id,
            signer_pk
        );

        let action_data = action_to_execute;

        // Determine the target account for the promise based on the action type.
        // When actions are initiated directly by `signer_account_id` (Derp),
        // some actions inherently target the signer's account.
        let promise_target_account_id = match action_data.action_type {
            ActionType::FunctionCall | ActionType::Transfer => {
                action_data.receiver_id.clone().unwrap_or_else(|| {
                    panic!("receiver_id is required for FunctionCall/Transfer")
                })
            }
            ActionType::CreateAccount => {
                // For CreateAccount, the promise is typically initiated by an existing account (the signer).
                // The `receiver_id` in `action_data` specifies the new account_id to be created.
                // The initial promise should be targeted at the signer_account_id, which will then create the new account.
                signer_account_id.clone()
            }
            ActionType::DeployContract => {
                // Deploying to Derp's own account.
                signer_account_id.clone()
            }
            ActionType::Stake => {
                // Derp stakes from their own account.
                // The public_key in action_data for Stake is the validator's public key.
                signer_account_id.clone()
            }
            ActionType::AddKey => {
                // Adding a key to Derp's own account.
                // The public_key in action_data is the new key to add.
                // The receiver_id and method_names in action_data are for FunctionCall access key permissions.
                signer_account_id.clone()
            }
            ActionType::DeleteKey => {
                // Deleting a key from Derp's own account.
                // The public_key in action_data is the key to delete.
                signer_account_id.clone()
            }
            ActionType::DeleteAccount => {
                // Derp deleting their own account.
                // The beneficiary_id in action_data is where remaining funds go.
                signer_account_id.clone()
            }
        };

        let mut promise = Promise::new(promise_target_account_id.clone());

        match action_data.action_type {
            ActionType::CreateAccount => {
                let _new_account_id = action_data.receiver_id.clone().unwrap_or_else(|| {
                    panic!("receiver_id (new account_id) is required for CreateAccount")
                });
                promise = promise.create_account();
                if let Some(deposit) = action_data.initial_deposit_for_new_account {
                    if deposit.0 > 0 {
                        promise = promise.transfer(NearToken::from_yoctonear(deposit.0));
                    }
                }
                if let Some(pk) = action_data.public_key_for_new_account.clone() {
                    promise = promise.add_full_access_key(pk);
                }
            }
            ActionType::DeployContract => {
                promise = promise.deploy_contract(
                    action_data
                        .code
                        .unwrap_or_else(|| Base64VecU8(vec![]))
                        .0,
                );
            }
            ActionType::FunctionCall => {
                promise = promise.function_call(
                    action_data.method_name.unwrap_or_else(|| String::new()),
                    action_data
                        .args
                        .unwrap_or_else(|| Base64VecU8(vec![]))
                        .0,
                    NearToken::from_yoctonear(action_data.deposit.unwrap_or_else(|| U128(0)).0),
                    action_data.gas.unwrap_or_else(|| Gas::from_gas(5_000_000_000_000)), // Default to 5 TGas
                );
            }
            ActionType::Transfer => {
                promise = promise.transfer(NearToken::from_yoctonear(
                    action_data
                        .amount
                        .unwrap_or_else(|| panic!("amount is required for Transfer"))
                        .0,
                ));
            }
            ActionType::Stake => {
                promise = promise.stake(
                    NearToken::from_yoctonear(action_data
                        .stake
                        .unwrap_or_else(|| panic!("stake amount is required for Stake"))
                        .0),
                    action_data
                        .public_key
                        .clone()
                        .unwrap_or_else(|| panic!("validator public_key is required for Stake"))
                        .clone(),
                );
            }
            ActionType::AddKey => {
                promise = promise.add_access_key_allowance(
                    action_data
                        .public_key
                        .clone()
                        .unwrap_or_else(|| panic!("public_key is required for AddKey"))
                        .clone(),
                    action_data.get_action_allowance(),
                    action_data.receiver_id.clone().unwrap_or_else(|| {
                        panic!("receiver_id for allowance scope is required for AddKey")
                    }), // This is the contract_id for the function call access key
                    action_data
                        .method_names
                        .unwrap_or_else(|| vec![])
                        .join(","),
                );
            }
            ActionType::DeleteKey => {
                promise = promise.delete_key(
                    action_data
                        .public_key
                        .clone()
                        .unwrap_or_else(|| panic!("public_key is required for DeleteKey"))
                        .clone(),
                );
            }
            ActionType::DeleteAccount => {
                promise = promise.delete_account(
                    action_data
                        .beneficiary_id
                        .unwrap_or_else(|| panic!("beneficiary_id is required for DeleteAccount")),
                );
            }
        }
        log!(
            "Direct action {:?} prepared by {} for target {}",
            action_data.action_type,
            signer_account_id,
            promise_target_account_id
        );
        // The promise is scheduled implicitly by its creation.
    }

    pub fn execute_delegated_actions(
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
                // receiver_id (the new account) is already correctly set as promise_target_account_id for delegated creation.
                promise = promise.create_account();
                if let Some(deposit) = action_data.initial_deposit_for_new_account {
                    if deposit.0 > 0 {
                        promise = promise.transfer(NearToken::from_yoctonear(deposit.0));
                    }
                }
                if let Some(pk) = action_data.public_key_for_new_account.clone() {
                    promise = promise.add_full_access_key(pk);
                }
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