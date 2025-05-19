// Find all our documentation at https://docs.near.org
use near_sdk::{log, near, PanicOnDefault, NearToken};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::LookupMap;
use near_sdk::{env, near_bindgen, AccountId};
use near_sdk::json_types::U128;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct PaymentContract {
    greeting: String,
    balances: LookupMap<String, LookupMap<AccountId, u128>>,
    trusted_account: AccountId,
}

#[near]
impl PaymentContract {
    #[init]
    pub fn new(trusted_account: AccountId) -> Self {
        Self {
            greeting: "Hello".to_string(),
            balances: LookupMap::new(b"b"),
            trusted_account,
        }
    }

    pub fn get_greeting(&self) -> String {
        self.greeting.clone()
    }

    pub fn set_greeting(&mut self, greeting: String) {
        log!("Saving greeting: {greeting}");
        self.greeting = greeting;
    }

    // Allows users to pay for usage tokens with NEAR for a specific ReverieId
    #[payable]
    pub fn deposit(&mut self, reverie_id: String) {
        let user_id = env::predecessor_account_id();
        let amount_deposited = env::attached_deposit().as_yoctonear();
        let mut user_balances = self.balances.remove(&reverie_id).unwrap_or_else(|| LookupMap::new(format!("b:{}", reverie_id).as_bytes()));
        let current_balance = user_balances.get(&user_id).unwrap_or(&0);
        let new_balance = current_balance + amount_deposited;
        user_balances.insert(user_id.clone(), new_balance);
        self.balances.insert(reverie_id.clone(), user_balances);
        log!("Deposited {} for user {} on reverie {}", amount_deposited, user_id, reverie_id);
    }

    // Gets the balance of a user for a specific ReverieId.
    pub fn get_balance(&self, reverie_id: String, user_id: AccountId) -> U128 {
        if let Some(user_balances) = self.balances.get(&reverie_id) {
            U128(*user_balances.get(&user_id).unwrap_or(&0))
        } else {
            U128(0)
        }
    }

    // Checks if a user can spend a certain amount for a specific ReverieId.
    pub fn can_spend(&self, reverie_id: String, user_id: AccountId, amount: U128) -> bool {
        let balance = self.get_balance(reverie_id, user_id);
        balance >= amount
    }

    // Records Usage Spend for a user for a specific ReverieId.
    pub fn record_spend(&mut self, reverie_id: String, user_id: AccountId, amount_to_spend: U128) {
        // Only callable by the trusted account.
        assert_eq!(
            env::predecessor_account_id(),
            self.trusted_account,
            "Only the trusted account can call this method"
        );
        let mut user_balances = self.balances.remove(&reverie_id).unwrap_or_else(|| LookupMap::new(format!("b:{}", reverie_id).as_bytes()));
        let current_balance = user_balances.get(&user_id).unwrap_or(&0);
        assert!(
            *current_balance >= amount_to_spend.0,
            "Insufficient balance to record spend. User has {}, needed {}", current_balance, amount_to_spend.0
        );
        let new_balance = *current_balance - amount_to_spend.0;
        if new_balance == 0 {
            user_balances.remove(&user_id);
        } else {
            user_balances.insert(user_id.clone(), new_balance);
        }
        self.balances.insert(reverie_id.clone(), user_balances);
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
        let user_id = env::predecessor_account_id();
        let mut user_balances = self.balances.remove(&reverie_id).unwrap_or_else(|| LookupMap::new(format!("b:{}", reverie_id).as_bytes()));
        let current_balance = user_balances.get(&user_id).unwrap_or(&0);

        assert!(amount.0 > 0, "Withdrawal amount must be greater than 0");
        assert!(
            *current_balance >= amount.0,
            "Insufficient balance to withdraw. User has {}, requested {}",
            current_balance,
            amount.0
        );

        let new_balance = *current_balance - amount.0;
        if new_balance == 0 {
            user_balances.remove(&user_id);
        } else {
            user_balances.insert(user_id.clone(), new_balance);
        }
        self.balances.insert(reverie_id.clone(), user_balances);

        near_sdk::Promise::new(user_id.clone()).transfer(near_sdk::NearToken::from_yoctonear(amount.0));
        log!(
            "Withdrew {} yoctoNEAR for user {} on reverie {}. New balance: {}",
            amount.0,
            user_id,
            reverie_id,
            new_balance
        );
    }
}

#[cfg(test)]
mod tests;
