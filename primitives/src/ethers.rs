// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Convert from Ethers types.

use alloy_primitives::{Bloom, B160, B256, U256};
use anyhow::{anyhow, Context};
use ethers_core::types::{
    transaction::eip2930::{
        AccessList as EthersAccessList, AccessListItem as EthersAccessListItem,
    },
    Block as EthersBlock, Transaction as EthersTransaction, Withdrawal as EthersWithdrawal,
    H160 as EthersH160, H256 as EthersH256, U256 as EthersU256,
};

use crate::{
    access_list::{AccessList, AccessListItem},
    block::Header,
    signature::TxSignature,
    transaction::{
        Transaction, TransactionKind, TxEssence, TxEssenceEip1559, TxEssenceEip2930,
        TxEssenceLegacy,
    },
    withdrawal::Withdrawal,
};

#[inline]
pub fn from_ethers_u256(v: EthersU256) -> U256 {
    U256::from_limbs(v.0)
}

#[inline]
pub fn from_ethers_h160(v: EthersH160) -> B160 {
    v.0.into()
}

#[inline]
pub fn from_ethers_h256(v: EthersH256) -> B256 {
    v.0.into()
}

impl From<EthersAccessListItem> for AccessListItem {
    fn from(item: EthersAccessListItem) -> Self {
        AccessListItem {
            address: item.address.0.into(),
            storage_keys: item
                .storage_keys
                .into_iter()
                .map(|key| key.0.into())
                .collect(),
        }
    }
}

impl From<EthersAccessList> for AccessList {
    fn from(list: EthersAccessList) -> Self {
        AccessList(list.0.into_iter().map(|item| item.into()).collect())
    }
}

impl From<Option<EthersH160>> for TransactionKind {
    fn from(addr: Option<EthersH160>) -> Self {
        match addr {
            Some(address) => TransactionKind::Call(address.0.into()),
            None => TransactionKind::Create,
        }
    }
}

impl<T> TryFrom<EthersBlock<T>> for Header {
    type Error = anyhow::Error;

    fn try_from(block: EthersBlock<T>) -> Result<Self, Self::Error> {
        Ok(Header {
            parent_hash: from_ethers_h256(block.parent_hash),
            ommers_hash: from_ethers_h256(block.uncles_hash),
            beneficiary: from_ethers_h160(block.author.context("author missing")?),
            state_root: from_ethers_h256(block.state_root),
            transactions_root: from_ethers_h256(block.transactions_root),
            receipts_root: from_ethers_h256(block.receipts_root),
            logs_bloom: Bloom::from_slice(
                block.logs_bloom.context("logs_bloom missing")?.as_bytes(),
            ),
            difficulty: from_ethers_u256(block.difficulty),
            number: block.number.context("number missing")?.as_u64(),
            gas_limit: from_ethers_u256(block.gas_limit),
            gas_used: from_ethers_u256(block.gas_used),
            timestamp: from_ethers_u256(block.timestamp),
            extra_data: block.extra_data.0.into(),
            mix_hash: block.mix_hash.context("mix_hash missing")?.0.into(),
            nonce: block.nonce.context("nonce missing")?.0.into(),
            base_fee_per_gas: from_ethers_u256(
                block.base_fee_per_gas.context("base_fee_per_gas missing")?,
            ),
            withdrawals_root: block.withdrawals_root.map(from_ethers_h256),
        })
    }
}

impl TryFrom<EthersTransaction> for Transaction {
    type Error = anyhow::Error;

    fn try_from(tx: EthersTransaction) -> Result<Self, Self::Error> {
        let essence = match tx.transaction_type.map(|t| t.as_u64()) {
            None | Some(0) => TxEssence::Legacy(TxEssenceLegacy {
                chain_id: match tx.chain_id {
                    None => None,
                    Some(chain_id) => Some(
                        chain_id
                            .try_into()
                            .map_err(|err| anyhow!("invalid chain_id: {}", err))?,
                    ),
                },
                nonce: tx
                    .nonce
                    .try_into()
                    .map_err(|err| anyhow!("invalid nonce: {}", err))?,
                gas_price: from_ethers_u256(tx.gas_price.context("gas_price missing")?),
                gas_limit: from_ethers_u256(tx.gas),
                to: tx.to.into(),
                value: from_ethers_u256(tx.value),
                data: tx.input.0.into(),
            }),
            Some(1) => TxEssence::Eip2930(TxEssenceEip2930 {
                chain_id: tx
                    .chain_id
                    .context("chain_id missing")?
                    .try_into()
                    .map_err(|err| anyhow!("invalid chain_id: {}", err))?,
                nonce: tx
                    .nonce
                    .try_into()
                    .map_err(|err| anyhow!("invalid nonce: {}", err))?,
                gas_price: from_ethers_u256(tx.gas_price.context("gas_price missing")?),
                gas_limit: from_ethers_u256(tx.gas),
                to: tx.to.into(),
                value: from_ethers_u256(tx.value),
                access_list: tx.access_list.context("access_list missing")?.into(),
                data: tx.input.0.into(),
            }),
            Some(2) => TxEssence::Eip1559(TxEssenceEip1559 {
                chain_id: tx
                    .chain_id
                    .context("chain_id missing")?
                    .try_into()
                    .map_err(|err| anyhow!("invalid chain_id: {}", err))?,
                nonce: tx
                    .nonce
                    .try_into()
                    .map_err(|err| anyhow!("invalid nonce: {}", err))?,
                max_priority_fee_per_gas: from_ethers_u256(
                    tx.max_priority_fee_per_gas
                        .context("max_priority_fee_per_gas missing")?,
                ),
                max_fee_per_gas: from_ethers_u256(
                    tx.max_fee_per_gas.context("max_fee_per_gas missing")?,
                ),
                gas_limit: from_ethers_u256(tx.gas),
                to: tx.to.into(),
                value: from_ethers_u256(tx.value),
                access_list: tx.access_list.context("access_list missing")?.into(),
                data: tx.input.0.into(),
            }),
            _ => unreachable!(),
        };
        let signature = TxSignature {
            v: tx.v.as_u64(),
            r: from_ethers_u256(tx.r),
            s: from_ethers_u256(tx.s),
        };

        Ok(Transaction { essence, signature })
    }
}

impl TryFrom<EthersWithdrawal> for Withdrawal {
    type Error = anyhow::Error;

    fn try_from(withdrawal: EthersWithdrawal) -> Result<Self, Self::Error> {
        Ok(Withdrawal {
            index: withdrawal.index.as_u64(),
            validator_index: withdrawal.validator_index.as_u64(),
            address: withdrawal.address.0.into(),
            amount: withdrawal
                .amount
                .try_into()
                .map_err(|err| anyhow!("invalid amount: {}", err))?,
        })
    }
}