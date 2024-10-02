use super::utils::as_checksum;
use serde::{Serialize, Deserialize};
use rustc_hex::FromHexError;
use ssz_rs::Sized;
use std::{
    ops::{AddAssign, Deref},
    slice::Windows,
    str::FromStr,
};
use ethers::{
    abi::AbiEncode, contract::{EthAbiCodec, EthAbiType}, core::k256::elliptic_curve::consts::U245, middleware::transformer::ds_proxy::factory, types::{Address, Bytes, Log, TransactionReceipt, H256, U256, U64}, utils::keccak256
};

#[derive(
    Default,
    Clone,
    Debug,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
    Serialize, 
    Deserialize,
    EthAbiCodec,
    EthAbiType,
)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    pub sender: Address,
    pub nonce: U256,
    pub factory: Address,
    pub factory_data: Bytes,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster: String,
    pub paymaster_verification_gas_limit: U256,
    pub paymaster_post_op_gas_limit: U256,
    pub paymaster_data: Bytes,
    pub signature: Bytes
}

impl UserOperation {

    pub fn pack(&self) -> Bytes {
        self.clone().encode().into()
    }

    pub fn pack_without_signature(&self) -> Bytes {
        let user_operation_packed = UserOperationUnsigned::from(self.clone());
        user_operation_packed.encode().into()
    }

    pub fn hash(&self, entry_point: &Address, chain_id: &U256) -> UserOperationHash {
        H256::from_slice(
            keccak256(
                [
                    keccak256(self.pack_without_signature().deref()).to_vec(),
                    entry_point.encode(),
                    chain_id.encode(),
                ]
                .concat(),
            )
            .as_slice(),
        )
        .into()
    }

    pub fn sender(mut self, sender: Address) -> Self {
        self.sender = sender;
        self
    }

    pub fn nonce(mut self, nonce: U256) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn factory(mut self, facotry: Address) -> Self {
        self.factory = facotry;
        self
    }

    pub fn factory_data(mut self, factory_data: Bytes) -> Self {
        self.factory_data = factory_data;
        self
    }
    
    pub fn call_data(mut self, call_data: Bytes) -> Self {
        self.call_data = call_data;
        self
    }

    pub fn call_gas_limit(mut self, call_gas_limit: U256) -> Self {
        self.call_gas_limit = call_gas_limit;
        self
    }

    pub fn verification_gas_limit(mut self, verification_gas_limit: U256) -> Self {
        self.verification_gas_limit = verification_gas_limit;
        self
    }

    pub fn pre_verification_gas(mut self, pre_verification_gas: U256) -> Self {
        self.pre_verification_gas = pre_verification_gas;
        self
    }

    pub fn max_fee_per_gas(mut self, max_fee_per_gas: U256) -> Self {
        self.max_fee_per_gas = max_fee_per_gas;
        self
    }

    pub fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: U256) -> Self {
        self.max_priority_fee_per_gas = max_priority_fee_per_gas;
        self
    }

    pub fn paymaster(mut self, paymaster: String) -> Self {
        self.paymaster = paymaster;
        self
    }

    pub fn paymaster_verification_gas_limit(mut self, paymaster_verification_gas_limit: U256) -> Self {
        self.paymaster_verification_gas_limit = paymaster_verification_gas_limit;
        self
    }

    pub fn paymaster_post_op_gas_limit(mut self, paymaster_post_op_gas_limit: U256) -> Self {
        self.paymaster_post_op_gas_limit = paymaster_post_op_gas_limit;
        self
    }

    pub fn paymaster_data(mut self, paymaster_data: Bytes) -> Self {
        self.paymaster_data = paymaster_data;
        self
    }

    pub fn signature(mut self, signature: Bytes) -> Self {
        self.signature = signature;
        self
    }

}

// Here starts for UserOperationHash
#[derive(
    Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone, Copy, Default, PartialOrd, Ord
)]
pub struct UserOperationHash(pub H256);

impl From<H256> for UserOperationHash {
    fn from(value: H256) -> Self {
        Self(value)
    }
}

impl From<UserOperationHash> for H256 {
    fn from(value: UserOperationHash) -> Self {
        value.0
    }
}

impl From<[u8; 32]> for UserOperationHash {
    fn from(value: [u8; 32]) -> Self {
        Self(H256::from_slice(&value))
    }
}

impl FromStr for UserOperationHash {
    type Err = FromHexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        H256::from_str(s).map(|h| h.into())
    }
}

impl UserOperationHash {
    #[inline]
    pub const fn as_fixed_bytes(&self) -> &[u8; 32] {
        &self.0 .0
    }

    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0 .0
    }

    #[inline]
    pub const fn repeat_byte(byte: u8) -> UserOperationHash {
        UserOperationHash(H256([byte; 32]))
    }

    #[inline]
    pub const fn zero() -> UserOperationHash {
        UserOperationHash::repeat_byte(0u8)
    }

    pub fn assign_from_slice(&mut self, src: &[u8]) {
        self.as_bytes_mut().copy_from_slice(src);
    }

    pub fn from_slice(src: &[u8]) -> Self {
        let mut ret = Self::zero();
        ret.assign_from_slice(src);
        ret
    }
}

#[derive(EthAbiCodec, EthAbiType)]
pub struct UserOperationUnsigned {
    pub sender: Address,
    pub nonce: U256,
    pub factory: Address,
    pub factory_data: Bytes,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster: String,
    pub paymaster_verification_gas_limit: U256,
    pub paymaster_post_op_gas_limit: U256,
    pub paymaster_data: Bytes,
}

impl From<UserOperation> for UserOperationUnsigned {
    fn from(value: UserOperation) -> Self {
        Self {
            sender: value.sender,
            nonce: value.nonce,
            factory: value.factory,
            factory_data: value.factory_data,
            call_data: keccak256(value.call_data.deref()).into(),
            call_gas_limit: value.call_gas_limit,
            verification_gas_limit: value.verification_gas_limit,
            pre_verification_gas: value.pre_verification_gas,
            max_fee_per_gas: value.max_fee_per_gas,
            max_priority_fee_per_gas: value.max_priority_fee_per_gas,
            paymaster: value.paymaster,
            paymaster_verification_gas_limit: value.paymaster_verification_gas_limit,
            paymaster_post_op_gas_limit: value.paymaster_post_op_gas_limit,
            paymaster_data: value.paymaster_data,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationReceipt {
    #[serde(rename = "userOpHash")]
    pub user_operation_hash: UserOperationHash,
    #[serde(serialize_with = "as_checksum")]
    pub sender: Address,
    pub nonce: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paymaster: Option<Address>,
    pub actual_gas_cost: U256,
    pub actual_gas_used: U256,
    pub success: bool,
    pub reason: String,
    pub logs: Vec<Log>,
    #[serde(rename = "receipt")]
    pub tx_receipt: TransactionReceipt
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationByHash {
    pub user_operation: UserOperation,
    #[serde(serialize_with = "as_checksum")]
    pub entry_point: Address,
    pub transaction_hash: H256,
    pub block_hash: H256,
    pub block_number: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationPartial {
    pub sender: Option<Address>,
    pub nonce: Option<U256>,
    pub factory: Option<Address>,
    pub factory_data: Option<Bytes>,
    pub call_data: Option<Bytes>,
    pub call_gas_limit: Option<U256>,
    pub verification_gas_limit: Option<U256>,
    pub pre_verification_gas: Option<U256>,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub paymaster: Option<String>,
    pub paymaster_verification_gas_limit: Option<U256>,
    pub paymaster_post_op_gas_limit: Option<U256>,
    pub paymaster_data: Option<Bytes>,
    pub signature: Option<Bytes>
}

impl From<UserOperationPartial> for UserOperation {
    fn from(user_operation: UserOperationPartial) -> Self {
        Self {
            sender: {
                if let Some(sender) = user_operation.sender {
                    sender
                } else {
                    Address::zero()
                }
            },
            nonce: {
                if let Some(nonce) = user_operation.nonce {
                    nonce
                } else {
                    U256::zero()
                }
            },
            factory: {
                if let Some(factory) = user_operation.factory {
                    factory
                } else {
                    Address::zero()
                }
            },
            factory_data: {
                if let Some(factory_data) = user_operation.factory_data {
                    factory_data
                } else {
                    Bytes::default()
                }
            },
            call_data: {
                if let Some(call_data) = user_operation.call_data {
                    call_data
                } else {
                    Bytes::default()
                }
            },
            call_gas_limit: {
                if let Some(call_gas_limit) = user_operation.call_gas_limit {
                    call_gas_limit
                } else {
                    U256::zero()
                }
            },
            verification_gas_limit: {
                if let Some(verification_gas_limit) = user_operation.verification_gas_limit {
                    verification_gas_limit
                } else {
                    U256::zero()
                }
            },
            pre_verification_gas: {
                if let Some(pre_verification_gas) = user_operation.pre_verification_gas {
                    pre_verification_gas
                } else {
                    U256::zero()
                }
            },
            max_fee_per_gas: {
                if let Some(max_fee_per_gas) = user_operation.max_fee_per_gas {
                    max_fee_per_gas
                } else {
                    U256::zero()
                }
            },
            max_priority_fee_per_gas: {
                if let Some(max_priority_fee_per_gas) = user_operation.max_priority_fee_per_gas {
                    max_priority_fee_per_gas
                } else {
                    U256::zero()
                }
            },
            paymaster: {
                if let Some(paymaster) = user_operation.paymaster {
                    paymaster
                } else {
                    "0x".to_string()
                }
            },
            paymaster_verification_gas_limit: {
                if let Some(paymaster_verification_gas_limit) = user_operation.paymaster_verification_gas_limit {
                    paymaster_verification_gas_limit
                } else {
                    U256::zero()
                }
            },
            paymaster_post_op_gas_limit: {
                if let Some(paymaster_post_op_gas_limit) = user_operation.paymaster_post_op_gas_limit {
                    paymaster_post_op_gas_limit
                } else {
                    U256::zero()
                }
            },
            paymaster_data: {
                if let Some(paymaster_data) = user_operation.paymaster_data {
                    paymaster_data
                } else {
                    Bytes::default()
                }
            },
            signature: {
                if let Some(signature) = user_operation.signature {
                    signature
                } else {
                    Bytes::default()
                }
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationGasEstimation {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
    pub paymaster_verification_gas_limit: U256,
    pub paymaster_post_op_gas_limit: U256
}
