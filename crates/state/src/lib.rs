use std::collections::BTreeMap;
use freedom_primitives::{address_from_public_key, Address, Hash, Supply, GENESIS_SUPPLY};
use freedom_types::{Genesis, Transaction, TxKind, TransferPayload, Validator};

#[derive(Clone, Debug)]
pub struct Account {
    pub address: Address,
    pub public_key: [u8; 32],
    pub nonce: u64,
    pub balance: u128,
    pub locked: u128,
    pub staked: u128,
}

impl Account {
    pub fn new(public_key: [u8; 32], balance: u128) -> Self {
        Self {
            address: address_from_public_key(&public_key),
            public_key,
            nonce: 0,
            balance,
            locked: 0,
            staked: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChainState {
    pub height: u64,
    pub last_block_hash: Hash,
    pub accounts: BTreeMap<Address, Account>,
    pub validators: BTreeMap<Hash, Validator>,
    pub supply: Supply,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StateError {
    AccountNotFound,
    InvalidNonce { expected: u64, actual: u64 },
    InsufficientBalance,
    InvalidTransferPayload,
    InvalidRecipient,
    ArithmeticOverflow,
    UnsupportedTransaction,
}

impl ChainState {
    pub fn genesis(genesis: &Genesis) -> Self {
        assert_eq!(genesis.initial_supply, GENESIS_SUPPLY, "Genesis FDM must be zero");

        let validators = genesis.validators
            .iter()
            .cloned()
            .map(|v| (v.validator_id, v))
            .collect();

        Self {
            height: 0,
            last_block_hash: [0u8; 32],
            accounts: BTreeMap::new(),
            validators,
            supply: Supply::genesis(),
        }
    }

    pub fn insert_account(&mut self, account: Account) {
        self.accounts.insert(account.address, account);
    }

    pub fn execute_transaction(&mut self, tx: &Transaction) -> Result<(), StateError> {
        let sender = self.accounts.get(&tx.sender).ok_or(StateError::AccountNotFound)?;

        if sender.nonce != tx.nonce {
            return Err(StateError::InvalidNonce {
                expected: sender.nonce,
                actual: tx.nonce,
            });
        }

        match &tx.kind {
            TxKind::Transfer => {
                let transfer = TransferPayload::decode(&tx.payload)
                    .map_err(|_| StateError::InvalidTransferPayload)?;

                if transfer.to == [0u8; 32] {
                    return Err(StateError::InvalidRecipient);
                }

                let total_cost = transfer.amount
                    .checked_add(tx.fee)
                    .ok_or(StateError::ArithmeticOverflow)?;

                if sender.balance < total_cost {
                    return Err(StateError::InsufficientBalance);
                }

                let sender_balance = sender.balance - total_cost;
                let sender_nonce = sender.nonce
                    .checked_add(1)
                    .ok_or(StateError::ArithmeticOverflow)?;

                if transfer.to == tx.sender {
                    let sender = self.accounts.get_mut(&tx.sender).unwrap();
                    sender.balance = sender_balance
                        .checked_add(transfer.amount)
                        .ok_or(StateError::ArithmeticOverflow)?;
                    sender.nonce = sender_nonce;
                } else {
                    let sender = self.accounts.get_mut(&tx.sender).unwrap();
                    sender.balance = sender_balance;
                    sender.nonce = sender_nonce;

                    let recipient = self.accounts.entry(transfer.to).or_insert(Account {
                        address: transfer.to,
                        public_key: [0u8; 32],
                        nonce: 0,
                        balance: 0,
                        locked: 0,
                        staked: 0,
                    });

                    recipient.balance = recipient.balance
                        .checked_add(transfer.amount)
                        .ok_or(StateError::ArithmeticOverflow)?;
                }

                self.supply.circulating = self.supply.circulating
                    .checked_sub(tx.fee)
                    .ok_or(StateError::ArithmeticOverflow)?;
                self.supply.treasury = self.supply.treasury
                    .checked_add(tx.fee)
                    .ok_or(StateError::ArithmeticOverflow)?;
            }
            _ => return Err(StateError::UnsupportedTransaction),
        }

        Ok(())
    }

    pub fn assert_invariants(&self) {
        assert!(self.supply.is_consistent());

        let account_sum = self.accounts.values().try_fold(0u128, |sum, a| {
            let holdings = a.balance
                .checked_add(a.locked)
                .and_then(|v| v.checked_add(a.staked))?;
            sum.checked_add(holdings)
        });

        assert!(account_sum.is_some(), "account balance overflow");
        let account_sum = account_sum.unwrap();
        assert_eq!(
            self.supply.circulating,
            account_sum,
            "circulating supply does not equal account holdings"
        );
    }
}
