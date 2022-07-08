Near Price Oracle Converter
===

Proxy contract built on top of [NEAR Native price oracle](https://github.com/NearDeFi/price-oracle).

Use the Near Price Oracle Converter to request asset prices, convert assets using current exchange rates, and provide this data to a designated recipient.

**Input data**

```
asset_in: AccountId,
asset_out: AccountId,
amount: U128,
receiver_id: AccountId,
receiver_method: String,
receiver_args: String,
receiver_deposit: Option<U128>,
receiver_gas: Option<Gas>,
```

**Keywords in args to be overwritten**

- `%AMOUNT%` - amount received after conversion
- `%EXCHANGE_INFO%` - exchange log
- `%RATE_INFO%` - log of exchange rates
- `%SIGNER_ID%` - user account 
- `%CONVERTER_ID%` - converter account
- `%ORACLE_ID%` - oracle account 

**Example:**

User has a salary in USD, and is requesting a payout from the DAO in $NEAR tokens.

1. The user opens `coinmarketcap`/`coingecko` and calculates approximate amount in $NEAR tokens for his proposal using the rounded current rate
2. Every DAO council checks `coinmarketcap`/`coingecko` and  rejects the proposal if the exchange rate was incorrect.

Instead, user can create a DAO proposal using a `price oracle converter` proxy contract.
1. DAO council checks that payout proposer is a `price oracle converter` contract and skip checking exchange rates for a given proposal.
2. User receive exact amount in $NEAR tokens without rounding.

**Example request:**

Convert 300 USD into NEAR and submit a proposal to `mydao.sputnikv2.testnet` with the text `Payout for community building in May` and current exchange rate details:
```rust
near call $CONTRACT_ID convert '{
    "asset_in": "usdt.fakes.testnet", 
    "asset_out": "wrap.testnet", 
    "amount": "30000000", 
    "receiver_id": "mydao.sputnikv2.testnet", 
    "receiver_method": "add_proposal", 
    "receiver_deposit": "1000000000000000000000000", 
    "receiver_gas": "30000000000000", 
    "receiver_args": "{
        \\"proposal\\": {\\"description\\": \\"Payout for community building in May (%EXCHANGE_INFO%)\\", 
        \\"kind\\": { 
            \\"Transfer\\": { 
                \\"token_id\\": \\"\\", 
                \\"receiver_id\\": \\"%SIGNER_ID%\\", 
                \\"amount\\": \\"%AMOUNT%\\" 
                    } 
                } 
            } 
        }" 
    }' --accountId zavodil.testnet --gas 200000000000000 --deposit 1
```
*Keep `receiver_args` in one line to call in CLI*
