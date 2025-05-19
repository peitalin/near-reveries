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
    reverie_ids: Vec<String>,
    balances: LookupMap<AccountId, u128>,
    trusted_account: AccountId,
}

#[near]
impl PaymentContract {
    #[init]
    pub fn new(trusted_account: AccountId) -> Self {
        Self {
            greeting: "Hello".to_string(),
            reverie_ids: Vec::new(),
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

    // Allows users to deposit NEAR.
    // The amount is implicitly taken from the attached deposit.
    #[payable]
    pub fn deposit(&mut self) {
        let user_id = env::predecessor_account_id();
        let amount_deposited = env::attached_deposit().as_yoctonear();
        let current_balance = self.balances.get(&user_id).unwrap_or(&0);
        let new_balance = current_balance + amount_deposited;
        self.balances.insert(user_id.clone(), new_balance);
        log!("Deposited {} for user {}", amount_deposited, user_id);
    }

    // Gets the balance of a user.
    pub fn get_balance(&self, user_id: AccountId) -> U128 {
        U128(*self.balances.get(&user_id).unwrap_or(&0))
    }

    // Internal helper to get raw u128 balance
    fn get_balance_internal(&self, user_id: AccountId) -> u128 {
        *self.balances.get(&user_id).unwrap_or(&0)
    }

    // Checks if a user can spend a certain amount.
    pub fn can_spend(&self, user_id: AccountId, amount: U128) -> bool {
        self.get_balance_internal(user_id) >= amount.0
    }

    // Records a spend for a user. Only callable by the trusted account.
    pub fn record_spend(&mut self, user_id: AccountId, amount_to_spend: U128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.trusted_account,
            "Only the trusted account can call this method"
        );
        let current_balance = self.get_balance_internal(user_id.clone());
        assert!(
            current_balance >= amount_to_spend.0,
            "Insufficient balance to record spend. User has {}, needed {}", current_balance, amount_to_spend.0
        );
        let new_balance = current_balance - amount_to_spend.0;
        self.balances.insert(user_id.clone(), new_balance);
        log!("Recorded spend of {} for user {}", amount_to_spend.0, user_id);
    }

    // Getter for the trusted account (optional, for verification/management)
    pub fn get_trusted_account(&self) -> AccountId {
        self.trusted_account.clone()
    }

    // Function to update the trusted account if needed (callable by contract owner/self)
    // For security, this should be thoughtfully designed. Here, only the contract account itself can change it.
    pub fn update_trusted_account(&mut self, new_trusted_account: AccountId) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Only the contract account can update the trusted account");
        log!("Trusted account updated from {} to {}", self.trusted_account, new_trusted_account);
        self.trusted_account = new_trusted_account;
    }

    // Allows users to withdraw their deposited NEAR.
    pub fn withdraw(&mut self, amount: U128) {
        let user_id = env::predecessor_account_id();
        let current_balance = self.get_balance_internal(user_id.clone());

        assert!(amount.0 > 0, "Withdrawal amount must be greater than 0");
        assert!(
            current_balance >= amount.0,
            "Insufficient balance to withdraw. User has {}, requested {}",
            current_balance,
            amount.0
        );

        let new_balance = current_balance - amount.0;
        if new_balance == 0 {
            self.balances.remove(&user_id);
        } else {
            self.balances.insert(user_id.clone(), new_balance);
        }

        near_sdk::Promise::new(user_id.clone()).transfer(near_sdk::NearToken::from_yoctonear(amount.0));
        log!(
            "Withdrew {} yoctoNEAR for user {}. New balance: {}",
            amount.0,
            user_id,
            new_balance
        );
    }
}

/*
 * The rest of this file holds the inline tests for the code above
 * Learn more about Rust tests: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
 */
#[cfg(test)]
mod tests {
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

    // Helper to create a new contract with a designated trusted account
    fn new_contract(trusted_account: AccountId) -> PaymentContract {
        PaymentContract::new(trusted_account)
    }

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
        contract.deposit();
        assert_eq!(contract.get_balance(user.clone()), U128(100));

        testing_env!(get_context(user.clone(), 50).build());
        contract.deposit();
        assert_eq!(contract.get_balance(user.clone()), U128(150));
    }

    #[test]
    fn get_balance_of_unknown_user() {
        let trusted_account = accounts(1);
        let contract = new_contract(trusted_account.clone());
        let unknown_user = AccountId::try_from("unknown.testnet".to_string()).unwrap();
        assert_eq!(contract.get_balance(unknown_user), U128(0));
    }

    #[test]
    fn can_spend_sufficient_funds() {
        let user = accounts(1);
        let trusted_account = accounts(2);
        let mut contract = new_contract(trusted_account.clone());

        testing_env!(get_context(user.clone(), NearToken::from_near(100).as_yoctonear()).build());
        contract.deposit();

        assert!(contract.can_spend(user.clone(), U128(NearToken::from_near(50).as_yoctonear())));
        assert!(contract.can_spend(user.clone(), U128(NearToken::from_near(100).as_yoctonear())));
    }


    #[test]
    fn can_spend_insufficient_funds() {
        let user = accounts(1);
        let trusted_account = accounts(2);
        let mut contract = new_contract(trusted_account.clone());

        testing_env!(get_context(user.clone(), NearToken::from_near(100).as_yoctonear()).build());
        contract.deposit();

        assert!(!contract.can_spend(user.clone(), U128(NearToken::from_near(101).as_yoctonear())));
    }

    #[test]
    fn can_spend_zero_balance() {
        let user = accounts(1);
        let trusted_account = accounts(2);
        let contract = new_contract(trusted_account.clone());
        let unknown_user = AccountId::try_from("unknown.testnet".to_string()).unwrap();

        assert!(!contract.can_spend(unknown_user, U128(NearToken::from_near(1).as_yoctonear())));
    }

    #[test]
    fn record_spend_success() {
        let user = accounts(1);
        let trusted_account = accounts(2);
        let mut contract = new_contract(trusted_account.clone());

        testing_env!(get_context(user.clone(), 100).build());
        contract.deposit();
        assert_eq!(contract.get_balance_internal(user.clone()), 100);

        testing_env!(get_context(trusted_account.clone(), 0).build());
        contract.record_spend(user.clone(), U128(30));
        assert_eq!(contract.get_balance_internal(user.clone()), 70);
    }

    #[test]
    #[should_panic(expected = "Only the trusted account can call this method")]
    fn record_spend_unauthorized() {
        let user = accounts(1);
        let trusted_account = accounts(2);
        let unauthorized_caller = accounts(3);
        let mut contract = new_contract(trusted_account.clone());

        testing_env!(get_context(user.clone(), 100).build());
        contract.deposit();

        testing_env!(get_context(unauthorized_caller.clone(), 0).build());
        contract.record_spend(user.clone(), U128(30));
    }

    #[test]
    #[should_panic(expected = "Insufficient balance to record spend.")]
    fn record_spend_insufficient_balance() {
        let user = accounts(1);
        let trusted_account = accounts(2);
        let mut contract = new_contract(trusted_account.clone());

        testing_env!(get_context(user.clone(), 20).build());
        contract.deposit();

        testing_env!(get_context(trusted_account.clone(), 0).build());
        contract.record_spend(user.clone(), U128(30));
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
        contract.deposit();
        assert_eq!(contract.get_balance(user.clone()), U128(NearToken::from_near(10).as_yoctonear()));

        testing_env!(get_context(user.clone(), 0).build());
        contract.withdraw(U128(NearToken::from_near(3).as_yoctonear()));
        assert_eq!(contract.get_balance(user.clone()), U128(NearToken::from_near(7).as_yoctonear()));
    }

    #[test]
    #[should_panic(expected = "Insufficient balance to withdraw.")]
    fn test_withdraw_insufficient_funds() {
        let user = accounts(1);
        let trusted = accounts(2);
        let mut contract = new_contract(trusted.clone());

        testing_env!(get_context(user.clone(), NearToken::from_near(5).as_yoctonear()).build());
        contract.deposit();

        testing_env!(get_context(user.clone(), 0).build());
        contract.withdraw(U128(NearToken::from_near(10).as_yoctonear()));
    }

    #[test]
    #[should_panic(expected = "Withdrawal amount must be greater than 0")]
    fn test_withdraw_zero_amount() {
        let user = accounts(1);
        let trusted = accounts(2);
        let mut contract = new_contract(trusted.clone());

        testing_env!(get_context(user.clone(), NearToken::from_near(5).as_yoctonear()).build());
        contract.deposit();

        testing_env!(get_context(user.clone(), 0).build());
        contract.withdraw(U128(0));
    }

    #[test]
    #[should_panic(expected = "Insufficient balance to withdraw.")]
    fn test_withdraw_no_balance() {
        let user = accounts(1);
        let trusted = accounts(2);
        let mut contract = new_contract(trusted.clone());

        testing_env!(get_context(user.clone(), 0).build());
        contract.withdraw(U128(NearToken::from_near(1).as_yoctonear()));
    }

    #[test]
    fn test_withdraw_entire_balance() {
        let user = accounts(1);
        let trusted = accounts(2);
        let mut contract = new_contract(trusted.clone());

        let initial_deposit = NearToken::from_near(5).as_yoctonear();
        testing_env!(get_context(user.clone(), initial_deposit).build());
        contract.deposit();
        assert_eq!(contract.get_balance(user.clone()), U128(initial_deposit));

        testing_env!(get_context(user.clone(), 0).build());
        contract.withdraw(U128(initial_deposit));
        assert_eq!(contract.get_balance(user.clone()), U128(0));
        assert!(contract.balances.get(&user).is_none(), "User entry should be removed from balances if balance is zero");
    }

    #[test]
    fn test_can_spend_large_amount() {
        let user = accounts(1);
        let trusted = accounts(2);
        let mut contract = new_contract(trusted.clone());

        let deposit_amount = NearToken::from_near(3).as_yoctonear();
        testing_env!(get_context(user.clone(), deposit_amount).build());
        contract.deposit();
        assert_eq!(contract.get_balance(user.clone()), U128(deposit_amount));

        let large_spend_amount = "2400000000000000000000000".parse::<u128>().unwrap();
        let slightly_larger_spend_amount = "3100000000000000000000000".parse::<u128>().unwrap();

        assert!(contract.can_spend(user.clone(), U128(large_spend_amount)));
        assert!(!contract.can_spend(user.clone(), U128(slightly_larger_spend_amount)));
    }
}
