use crate::traits::{SmartWalletAccount, SmartWalletAccountFactory, MSABasicFactory};
use crate::consts::{GETH_SIMPLE_ACCOUNT_FACTORY, SIMPLE_ACCOUNT_FACTORY, MSA_FACTORY_SEPOLIA};
use ethers::middleware::transformer::ds_proxy::factory;
use ethers::signers::Wallet;
use ethers::{
    prelude::{NonceManagerMiddleware, SignerMiddleware},
    signers::LocalWallet,
    types::{Address, U256},
    providers::Middleware,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;
use hashbrown::HashMap;

use std::fmt::Debug;

// TODO: Figure out how to realize with alloy.rs.
// Seems this type is only used in bundler implementation. Hopefully we don't have to think this when we just want to have userOperation and send it to external bundler.
pub type SignerType<M> = NonceManagerMiddleware<SignerMiddleware<Arc<M>, LocalWallet>>;

pub type WalletMap = HashMap<Address, Arc<Mutex<Box<dyn SmartWalletAccount>>>>;
pub type MSABasicWalletMap = HashMap<Address, Arc<Mutex<Box<dyn SmartWalletAccount>>>>;

#[derive(Debug, Serialize)]
pub struct Request<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateResult {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
    pub paymaster_verification_gas_limit: U256,
    pub paymaster_post_op_gas_limit: U256,
}

#[derive(Debug, Deserialize)]
pub struct Response<R> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: R,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ErrorResponse {
    pub(crate) jsonrpc: String,
    pub(crate) id: u64,
    pub(crate) error: JsonRpcError,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

pub struct DeployedContract<C> {
    contract: C,
    pub address: Address,
}

impl<C> DeployedContract<C> {
    pub fn new(contract: C, addr: Address) -> Self {
        Self {
            contract,
            address: addr,
        }
    }

    pub fn contract(&self) -> &C {
        &self.contract
    }
}

pub enum WalletRegistry {
    SimpleAccount,
    MSABasicAccount,
}

#[allow(dead_code)]
impl WalletRegistry {
    pub fn from_str(s: &str) -> anyhow::Result<WalletRegistry> {

        match s {
            "simple-account" => Ok(WalletRegistry::SimpleAccount),
            "simple-account-test" => Ok(WalletRegistry::SimpleAccount),
            "msa-basic-account" => Ok(WalletRegistry::MSABasicAccount),
            _ => Err(anyhow::anyhow!("{} wallet currently not supported", s)),
        }

    }
}

pub enum WalletFactoryAddresses {
    SimpleAccountFactoryAddress(Address),
    MSABasicFactoryAddress(Address)
}


#[allow(dead_code)]
impl WalletFactoryAddresses {
    pub fn from_str(s: &str) -> anyhow::Result<WalletFactoryAddresses> {

        match s {
            "simple-account" => Ok(WalletFactoryAddresses::SimpleAccountFactoryAddress((
                SIMPLE_ACCOUNT_FACTORY.parse::<Address>().unwrap()),
            )),
            "simple-account-test" => Ok(WalletFactoryAddresses::SimpleAccountFactoryAddress((
                GETH_SIMPLE_ACCOUNT_FACTORY.parse::<Address>().unwrap()),
            )),
            "msa-account-sepolia" => Ok(WalletFactoryAddresses::MSABasicFactoryAddress((
                MSA_FACTORY_SEPOLIA.parse::<Address>().unwrap()),
            )),
            _ => Err(anyhow::anyhow!("{}'s factory not supported", s)),
        }

    }
}

#[derive(Debug)]
pub enum WalletFactoryRegistry<M: Middleware + 'static> {
    SimpleAccountFactory(Box<dyn SmartWalletAccountFactory<M>>),
    MSABasicFactory(Box<dyn MSABasicFactory<M>>),
}

impl<M: Middleware + 'static> Clone for WalletFactoryRegistry<M> {
    fn clone(&self) -> Self {
        match self {
            WalletFactoryRegistry::SimpleAccountFactory(factory) => {
                WalletFactoryRegistry::SimpleAccountFactory(factory.clone_box())
            }
            WalletFactoryRegistry::MSABasicFactory(factory) => {
                WalletFactoryRegistry::MSABasicFactory(factory.clone_box())
            }
        }
    }
}
