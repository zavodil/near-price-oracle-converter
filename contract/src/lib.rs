use std::collections::HashMap;
use std::convert::TryFrom;
use near_sdk::{near_bindgen, ext_contract, AccountId, BorshStorageKey, PanicOnDefault, borsh::{self, BorshDeserialize, BorshSerialize}, collections::{UnorderedMap}, serde::{Deserialize, Serialize}, Promise, env, Gas, Balance, Timestamp, log, is_promise_success, require};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;

mod utils;
mod owner;

use crate::utils::*;

const ORACLE_CALL_GAS: Gas = Gas(50_000_000_000_000);
const RECEIVER_METHOD_GAS: Gas = Gas(30_000_000_000_000);
const ON_RECEIVER_METHOD_GAS: Gas = Gas(10_000_000_000_000);

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
		Config,
		Deposits,
		Whitelist
}

#[ext_contract(ext_oracle)]
pub trait ExtOracleContract {
	/* request prices from the oracle */
	fn oracle_call(&mut self,
				   receiver_id: AccountId,
				   asset_ids: Option<Vec<AssetId>>,
				   msg: String);

}

#[ext_contract(ext_self)]
pub trait ExtSelf {
	/* callback to check if requested method succeeds */
	fn on_receiver_method(&mut self,
				   account_id: AccountId,
				   deposit: U128,
				   contract_id: AccountId,
				   method_name: MethodName);

}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
	/* user to perform admin actions */
	owner_id: AccountId,
	/* price oracle contract id */
	oracle_contract_id: AccountId,
	/* internal user deposits */
	deposits: LookupMap<AccountId, Balance>,
	/* token tickers and decimals */
	config: UnorderedMap<AccountId, TokenConfig>,
	/* list of method and contracts where repay on failed enabled */
	whitelist: UnorderedMap<AccountId, Vec<MethodName>>
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(oracle_contract_id: AccountId, owner_id: AccountId) -> Self {
        Self {
			owner_id,
			oracle_contract_id,
			deposits: LookupMap::new(StorageKey::Deposits),
			config: UnorderedMap::new(StorageKey::Config),
			whitelist: UnorderedMap::new(StorageKey::Whitelist)
		}
    }

	#[payable]
	/* main function to convert assets and callback receiver */
	pub fn convert(&mut self,
				    asset_in: AccountId,
				    asset_out: AccountId,
				    amount: U128,
					receiver_id: AccountId,
					receiver_method: String,
					receiver_args: String,
				    receiver_deposit: Option<U128>,
				   	receiver_gas: Option<Gas>,
	) -> Promise {
		let receiver_deposit = receiver_deposit.unwrap_or(U128(0));
		assert!(receiver_deposit.0 == 0 || receiver_deposit.0 == env::attached_deposit(), "ERR_ILLEGAL_DEPOSIT");

		let account_id = env::predecessor_account_id();
		self.deposits.insert(&account_id, &(self.internal_get_deposit(&account_id) + receiver_deposit.0));

		ext_oracle::ext(self.oracle_contract_id.clone())
			.with_static_gas(ORACLE_CALL_GAS)
			.with_attached_deposit(1)
			.oracle_call(
				env::current_account_id(),
				Some(vec![asset_in.to_string(), asset_out.to_string()]),
				json!({
                	"asset_in": asset_in,
					"asset_out": asset_out,
					"amount": amount,
					"receiver_id": receiver_id,
					"receiver_method": receiver_method,
					"receiver_args": receiver_args,
					"receiver_deposit": receiver_deposit,
					"receiver_gas": receiver_gas
        		}).to_string()
			)
	}

	#[private]
	pub fn on_receiver_method(&mut self, account_id: AccountId, deposit: U128, contract_id: AccountId, method_name: MethodName) {
		if !is_promise_success(){
			let master_account_id = self.get_master_account(&contract_id);
			let whitelisted_methods = self.get_whitelisted_methods(master_account_id);
			require!(whitelisted_methods.contains(&method_name), "ERR_ONLY_WHITELISTED_CONTRACTS_REPAY_DEPOSITS");

			let user_deposit = self.internal_get_deposit(&account_id);
			log!("Promise failed. Increase balance of {} with {} yNEAR. Previous balance: {} yNEAR", account_id, deposit.0, user_deposit);
			self.deposits.insert(&account_id, &(user_deposit + deposit.0));
		}
	}

	/* get internal user deposit */
	pub fn get_deposit(&self, account_id: &AccountId) -> U128 {
		U128(self.internal_get_deposit(account_id))
	}

	pub fn get_whitelisted_methods(&self, account_id: AccountId) -> Vec<MethodName> {
		self.whitelist.get(&account_id).unwrap_or_default()
	}
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TransferArgs {
	pub asset_in: AccountId,
	pub asset_out: AccountId,
	pub amount: U128,
	pub receiver_id: AccountId,
	pub receiver_method: String,
	pub receiver_args: String,
	pub receiver_deposit: Option<U128>,
	pub receiver_gas: Option<Gas>
}

#[near_bindgen]
impl OraclePriceReceiver for Contract {
	/* oracle callback with the requested price data */
	fn oracle_on_call(&mut self, sender_id: AccountId, data: PriceData, msg: String) -> U128 {
		assert_eq!(env::predecessor_account_id(), self.oracle_contract_id);

		assert!(
			data.recency_duration_sec <= 90,
			"Recency duration in the oracle call is larger than allowed maximum"
		);
		let timestamp = env::block_timestamp();
		assert!(
			data.timestamp <= timestamp,
			"Price data timestamp is in the future"
		);
		assert!(
			timestamp - data.timestamp <= 15_000_000_000,
			"Price data timestamp is too stale"
		);

		let TransferArgs {
			asset_in,
			asset_out,
			amount,
			receiver_id,
			receiver_method,
			receiver_args,
			receiver_deposit,
			receiver_gas
		} = near_sdk::serde_json::from_str(&msg).expect("Invalid TransferArgs");

		let receiver_deposit: Balance = receiver_deposit.unwrap_or(U128(0)).0;
		let account_id = env::signer_account_id();
		let user_deposit = self.internal_get_deposit(&account_id);
		assert!(user_deposit >= receiver_deposit, "ERR_MISSING_USER_DEPOSIT");
		self.deposits.insert(&account_id, &(user_deposit - receiver_deposit));

		let prices: HashMap<AccountId, Price> = data
			.prices
			.into_iter()
			.filter_map(|AssetOptionalPrice { asset_id, price }| {
				let token_id =
					AccountId::try_from(asset_id).expect("Asset is not a valid token ID");
				price.map(|price| {
					price.assert_valid();
					(token_id, price)
				})
			})
			.collect();

		let asset_in_price = prices
			.get(&asset_in)
			.expect("Missing Input Asset price");
		let asset_out_price = prices
			.get(&asset_out)
			.expect("Missing Output Asset price");

		let asset_out_extra = if asset_out_price.decimals < asset_in_price.decimals {
			10u128.pow((asset_in_price.decimals - asset_out_price.decimals) as _)
		} else {
			1
		};

		let asset_in_extra = if asset_in_price.decimals < asset_out_price.decimals {
			10u128.pow((asset_out_price.decimals - asset_in_price.decimals) as _)
		} else {
			1
		};

		let oracle_amount_out = u128_ratio(
			amount.0,
			asset_in_price.multiplier * asset_in_extra,
			asset_out_price.multiplier * asset_out_extra,

		);

		let (asset_in_ticker, asset_in_usd_price, asset_in_amount) = self.internal_get_asset(&asset_in, asset_in_price, amount.0);
		let (asset_out_ticker, asset_out_usd_price, asset_out_amount) = self.internal_get_asset(&asset_out, asset_out_price, oracle_amount_out);

		let exchange_info =
		if asset_in_amount > 0f64 {
			format!("{} {} = {} {}",
					asset_in_amount, asset_in_ticker, asset_out_amount, asset_out_ticker)
		}
		else {
			format!("{} {} = {} {}",
					amount.0, asset_in_ticker, oracle_amount_out, asset_out_ticker)
		};

		let rate_info = format!("1 {} = ${}, 1 {} = ${}",
									asset_in_ticker, asset_in_usd_price, asset_out_ticker, asset_out_usd_price);

		log!("Conversion complete: {}. BlockTimestamp: {}", exchange_info, data.timestamp);
		log!("Conversion rates: {}", rate_info);

		let receiver_args = receiver_args
			.replace("%AMOUNT%", &oracle_amount_out.to_string())
			.replace("%EXCHANGE_INFO%", &exchange_info)
			.replace("%RATE_INFO%", &rate_info)
			.replace("%SIGNER_ID%", &env::signer_account_id().to_string())
			.replace("%CONVERTER_ID%", &sender_id.to_string())
			.replace("%ORACLE_ID%", &self.oracle_contract_id.to_string());

		Promise::new(receiver_id.clone())
			.function_call(
				receiver_method.clone(),
				receiver_args.as_bytes().to_vec(),
				receiver_deposit,
				receiver_gas.unwrap_or(RECEIVER_METHOD_GAS),
			)
		.then(
			ext_self::ext(env::current_account_id())
				.with_static_gas(ON_RECEIVER_METHOD_GAS)
				.on_receiver_method(
					env::signer_account_id(),
					U128::from(receiver_deposit),
					receiver_id,
					receiver_method
				)
		);

		U128(oracle_amount_out)
	}
}

