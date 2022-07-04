use crate::*;

pub type AssetId = String;
pub type MethodName = String;
pub type DurationSec = u32;

const MAX_VALID_DECIMALS: u8 = 77;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenConfig {
    pub token_name: String,
    pub decimals: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetOptionalPrice {
    pub asset_id: AssetId,
    pub price: Option<Price>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PriceData {
    #[serde(with = "u64_dec_format")]
    pub timestamp: Timestamp,
    pub recency_duration_sec: DurationSec,

    pub prices: Vec<AssetOptionalPrice>,
}

pub trait OraclePriceReceiver {
    fn oracle_on_call(&mut self, sender_id: AccountId, data: PriceData, msg: String) -> U128;
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Copy)]
#[serde(crate = "near_sdk::serde")]
pub struct Price {
    #[serde(with = "u128_dec_format")]
    pub multiplier: Balance,
    pub decimals: u8,
}

impl Price {
    pub fn assert_valid(&self) {
        assert!(self.decimals <= MAX_VALID_DECIMALS);
    }
}

impl Contract {
    pub fn assert_owner(&self) {
        assert_eq!(&self.owner_id, &env::predecessor_account_id(), "ERR_NOT_AN_OWNER");
    }

    pub fn internal_get_deposit(&self, account_id: &AccountId) -> Balance {
        self.deposits.get(account_id).unwrap_or_default()
    }

    pub fn internal_get_asset(&self, asset_id: &AccountId, asset_price: &Price, amount: Balance) -> (String, f64, f64){
        if let Some(config) = self.config.get(asset_id) {
            (
                config.token_name,
                asset_price.multiplier as f64 / (10u128.pow((asset_price.decimals - config.decimals) as u32)) as f64,
                ((amount / 10u128.pow((config.decimals - 2) as u32) ) as f64 / 100f64) as f64
            )
        }
        else {
            (
                asset_id.to_string(),
                0f64,
                0f64
            )
        }
    }

    /* get master account for subaccounts */
    pub fn get_master_account(&self, account_id: &AccountId) -> AccountId {
        let account: String = account_id.to_string();
        let parts: Vec<&str> = account.split('.').collect();
        let parts_count = parts.to_owned().clone().len();

        if parts_count <= 2 {
            AccountId::new_unchecked(account_id.to_string())
        }
        else {
            AccountId::new_unchecked(format!("{}.{}", parts[parts_count - 2], parts[parts_count - 1]))
        }
    }
}

pub mod u128_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
        where
            D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

pub mod u64_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
        where
            D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

uint::construct_uint!(
    pub struct U256(4);
);

pub(crate) fn u128_ratio(a: u128, num: u128, denom: u128) -> Balance {
    (U256::from(a) * U256::from(num) / U256::from(denom)).as_u128()
}