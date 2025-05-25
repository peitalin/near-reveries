#[cfg(test)]
mod tests_payments;

use near_sdk::{log, near, PanicOnDefault, NearToken};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::LookupMap;
use near_sdk::{env, AccountId};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

pub type ReverieId = String;

#[derive(JsonSchema, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct ReverieMetadata {
    pub reverie_type: String,
    pub description: String,
    pub access_condition: AccessCondition,
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
pub struct PaymentContract {
    greeting: String,
    trusted_account: AccountId,
    reverie_balances: LookupMap<ReverieId, LookupMap<AccountId, u128>>,
    reverie_ids: Vec<ReverieId>,
    reverie_metadata: LookupMap<ReverieId, ReverieMetadata>,
}

#[near]
impl PaymentContract {
    #[init]
    pub fn new(trusted_account: AccountId) -> Self {
        Self {
            greeting: "Hello".to_string(),
            trusted_account,
            reverie_balances: LookupMap::new(b"b"),
            reverie_ids: Vec::new(),
            reverie_metadata: LookupMap::new(b"r"),
        }
    }

    // internal method to require a reverie exists
    fn require_reverie_exists(&self, reverie_id: &str) {
        if self.reverie_metadata.get(reverie_id).is_none()
        || self.reverie_balances.get(reverie_id).is_none() {
            env::panic_str(&format!("ReverieId {} not found in registry or balances", reverie_id));
        }
    }

    // Allows users to pay for usage tokens with NEAR for a specific ReverieId
    #[payable]
    pub fn deposit(&mut self, reverie_id: String) {
        if self.reverie_metadata.get(&reverie_id).is_none() {
            env::panic_str(&format!("ReverieId {} not found in registry", reverie_id));
        }

        let user_id = env::predecessor_account_id();
        let amount_deposited = env::attached_deposit().as_yoctonear();

        let mut user_balances = self.reverie_balances
            .remove(&reverie_id)
            .unwrap_or_else(|| {
                LookupMap::new(format!("b:{}", reverie_id).as_bytes())
            });

        let current_balance = user_balances.get(&user_id).unwrap_or(&0);
        let new_balance = current_balance + amount_deposited;
        user_balances.insert(user_id.clone(), new_balance);
        self.reverie_balances.insert(reverie_id.clone(), user_balances);
        log!("Deposited {} for user {} on reverie {}", amount_deposited, user_id, reverie_id);
    }


    // Gets the balance of a user for a specific ReverieId.
    pub fn get_balance(&self, reverie_id: String, user_id: AccountId) -> U128 {
        self.require_reverie_exists(&reverie_id);
        if let Some(user_balances) = self.reverie_balances.get(&reverie_id) {
            U128(*user_balances.get(&user_id).unwrap_or(&0))
        } else {
            // Reverie is registered but no deposits made for it yet
            U128(0)
        }
    }

    // Checks if a user can spend a certain amount for a specific ReverieId.
    pub fn can_spend(&self, reverie_id: String, user_id: AccountId, amount: U128) -> bool {
        let balance = self.get_balance(reverie_id, user_id);
        balance >= amount
    }

    // internal method to get or insert balances for a reverie
    fn get_balances_for_reverie(&mut self, reverie_id: &str) -> LookupMap<AccountId, u128> {
        self.require_reverie_exists(reverie_id);
        self.reverie_balances.remove(reverie_id)
            .unwrap_or_else(|| LookupMap::new(format!("b:{}", reverie_id).as_bytes()))
    }

    // Records Usage Spend for a user for a specific ReverieId.
    pub fn record_spend(&mut self, reverie_id: String, user_id: AccountId, amount_to_spend: U128) {
        // Only callable by the trusted account.
        assert_eq!(
            env::predecessor_account_id(),
            self.trusted_account,
            "Only the trusted account can call this method"
        );

        let mut user_balances = self.get_balances_for_reverie(&reverie_id);
        let current_balance = *user_balances.get(&user_id).unwrap_or(&0);
        assert!(
            current_balance >= amount_to_spend.0,
            "Insufficient balance to record spend. User {} has {}, needed {} for reverie {}",
            user_id, current_balance, amount_to_spend.0, reverie_id
        );

        let new_balance = current_balance - amount_to_spend.0;
        if new_balance == 0 {
            user_balances.remove(&user_id);
        } else {
            user_balances.insert(user_id.clone(), new_balance);
        }

        self.reverie_balances.insert(reverie_id.clone(), user_balances);
        log!("Recorded spend of {} for user {} on reverie {}", amount_to_spend.0, user_id, reverie_id);
    }

    pub fn get_trusted_account(&self) -> AccountId {
        self.trusted_account.clone()
    }

    pub fn update_trusted_account(&mut self, new_trusted_account: AccountId) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Only the contract account can update the trusted account");
        log!("Trusted account updated from {} to {}", self.trusted_account, new_trusted_account);
        self.trusted_account = new_trusted_account;
    }

    pub fn withdraw(&mut self, reverie_id: String, amount: U128) {
        self.require_reverie_exists(&reverie_id);

        let user_id = env::predecessor_account_id();
        let mut user_balances = self.get_balances_for_reverie(&reverie_id);
        let current_balance = *user_balances.get(&user_id).unwrap_or(&0);

        assert!(amount.0 > 0, "Withdrawal amount must be greater than 0");
        assert!(
            current_balance >= amount.0,
            "Insufficient balance to withdraw. User {} has {}, requested {} for reverie {}",
            user_id, current_balance, amount.0, reverie_id
        );

        let new_balance = current_balance - amount.0;
        if new_balance == 0 {
            user_balances.remove(&user_id);
        } else {
            user_balances.insert(user_id.clone(), new_balance);
        }

        self.reverie_balances.insert(reverie_id.clone(), user_balances);

        near_sdk::Promise::new(user_id.clone()).transfer(near_sdk::NearToken::from_yoctonear(amount.0));
        log!(
            "Withdrew {} yoctoNEAR for user {} on reverie {}. New balance: {}",
            amount.0,
            user_id,
            reverie_id,
            new_balance
        );
    }

    /// Create a new reverie entry. Only the contract account can call this.
    pub fn create_reverie(
        &mut self,
        reverie_id: ReverieId,
        reverie_type: String,
        description: String,
        access_condition: AccessCondition,
    ) {
        assert_eq!(env::predecessor_account_id(), self.trusted_account, "Only the trusted account can create reveries");
        assert!(self.reverie_metadata.get(&reverie_id).is_none(), "ReverieId '{}' already exists on reverie_metadata", reverie_id);
        assert!(self.reverie_balances.get(&reverie_id).is_none(), "ReverieId '{}' already exists on reverie_balances", reverie_id);
        let metadata = ReverieMetadata {
            reverie_type,
            description,
            access_condition,
        };
        self.reverie_ids.push(reverie_id.clone());
        self.reverie_metadata.insert(reverie_id.clone(), metadata);
        self.reverie_balances.insert(reverie_id.clone(), LookupMap::new(format!("b:{}", reverie_id).as_bytes()));
    }

    /// For testing only
    pub fn delete_all_reveries(&mut self) {
        assert_eq!(env::predecessor_account_id(), self.trusted_account, "Only the trusted account can delete all reveries");
        let reverie_ids = self.reverie_ids.clone();
        for reverie_id in reverie_ids {
            self.delete_reverie_admin(reverie_id);
        }
    }

    pub fn delete_reverie_admin(&mut self, reverie_id: ReverieId) {
        assert_eq!(env::predecessor_account_id(), self.trusted_account, "Only the trusted account can delete reveries");
        self.reverie_metadata.remove(&reverie_id);
        self.reverie_balances.remove(&reverie_id);
        if let Some(index) = self.reverie_ids.iter().position(|id| id == &reverie_id) {
            self.reverie_ids.remove(index);
        }
    }

    pub fn get_reverie_metadata(&self, reverie_id: ReverieId) -> Option<ReverieMetadata> {
        self.reverie_metadata.get(&reverie_id).cloned()
    }

    pub fn get_reverie_ids(&self) -> Vec<ReverieId> {
        self.reverie_ids.clone()
    }
}
