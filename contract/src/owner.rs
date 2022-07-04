use crate::*;

#[near_bindgen]
impl Contract {
    /* withdraw internal user deposit */
    pub fn withdraw(&mut self) -> Promise {
        let account_id= env::predecessor_account_id();
        let deposit: Balance = self.internal_get_deposit(&account_id);
        assert!(deposit > 0, "ERR_MISSING_DEPOSIT");

        self.deposits.insert(&account_id, &0u128);
        Promise::new(account_id).transfer(deposit)
    }

    /* specify data for a single token */
    pub fn add_token_config (&mut self, account_id: AccountId, config: TokenConfig) {
        self.assert_owner();
        self.config.insert(&account_id, &config);
    }

    /* specify data for a set of tokens */
    pub fn add_token_configs (&mut self, configs: Vec<(AccountId, TokenConfig)>) {
        self.assert_owner();
        for (account_id, config) in configs {
            self.config.insert(&account_id, &config);
        }
    }

    /* withdraw 30% of gas spent by users */
    pub fn withdraw_gas_fees(&mut self, amount: U128) -> Promise {
        self.assert_owner();
        Promise::new(self.owner_id.clone()).transfer(amount.0)
    }

    /* whitelist contract and method where repay on fail allowed */
    pub fn whitelist(&mut self, account_id: AccountId, method_name: MethodName) {
        self.assert_owner();
        let mut whitelisted_methods = self.get_whitelisted_methods(account_id.clone());
        whitelisted_methods.push(method_name);
        self.whitelist.insert(&account_id, &whitelisted_methods);
    }
}