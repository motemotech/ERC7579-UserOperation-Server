use crate::errors::UserOpBuilderError;
use crate::gen::{SimpleAccount, MSABasic, SimpleAccountFactory, MSAFactory};
use crate::traits::{SmartWalletAccount, SmartWalletAccountFactory, MSABasicFactory};

use crate::types::{WalletRegistry, WalletFactoryRegistry, WalletFactoryAddresses};

use crate::primitives::user_operation::{UserOperation, UserOperationHash, UserOperationPartial};

use ethers::{
    providers::Middleware,
    types::{Address, Bytes, U256, H256},
    utils::keccak256,
};
use std::sync::Arc;
use anyhow::Ok;

#[derive(Debug)]
pub struct UserOperationBuilder<M: Middleware + 'static> {
    provider: Arc<M>,
    factory_contract: WalletFactoryRegistry<M>,
    factory_address: Address,
    wallet_contract: Box<dyn SmartWalletAccount>,
    scw_address: Option<Address>,
    signer_address: Address,
    salt: Option<u64>,
    uo: UserOperationPartial,
    uo_hash: Option<UserOperationHash>,
}

impl<M: Middleware> Clone for UserOperationBuilder<M> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            factory_contract: self.factory_contract.clone(),
            factory_address: self.factory_address,
            wallet_contract: self.wallet_contract.clone_box(),
            scw_address: self.scw_address,
            signer_address: self.signer_address,
            salt: self.salt,
            uo: self.uo.clone(),
            uo_hash: self.uo_hash,
        }
    }
}

impl<M: Middleware + 'static> UserOperationBuilder<M> {

    pub fn new(
        eoa_wallet_address: Address,
        wallet_name: impl Into<String>,
        scw_address: Option<Address>,
        provider: Arc<M>,
        salt: Option<u64>,
    ) -> anyhow::Result<Self> {
        let (wallet_contract, factory_contract, factory_address) =
            Self::match_wallet(wallet_name.into(), provider.clone())?;

        let uo = UserOperationPartial {
            sender: None,
            nonce: None,
            factory: None,
            factory_data: None,
            call_data: None,
            call_gas_limit: None,
            verification_gas_limit: None,
            pre_verification_gas: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            paymaster: None,
            paymaster_verification_gas_limit: None,
            paymaster_post_op_gas_limit: None,
            paymaster_data: None,
            signature: None
        };

        Ok(Self {
            provider,
            factory_contract,
            factory_address,
            wallet_contract,
            scw_address,
            signer_address: eoa_wallet_address,
            salt,
            uo,
            uo_hash: None,
        })
    }

    pub fn from_uo(
        uo: UserOperationPartial,
        provider: Arc<M>,
        wallet_name: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let mut uo_builder = Self::new(Address::zero(), wallet_name, None, provider, None)?;
        uo_builder.set_uo(uo);
        Ok(uo_builder)
    }

    #[allow(clippy::type_complexity)]
    fn match_wallet(
        wallet_name: String,
        provider: Arc<M>,
    ) -> anyhow::Result<(
        Box<dyn SmartWalletAccount>,
        WalletFactoryRegistry<M>,
        Address,
    )> {
        let (factory_contract, factory_address): (WalletFactoryRegistry<M>, Address) =
            match WalletFactoryAddresses::from_str(&wallet_name)? {
                WalletFactoryAddresses::SimpleAccountFactoryAddress(addr) => {
                    let wf = WalletFactoryRegistry::SimpleAccountFactory(Box::new(SimpleAccountFactory::new(addr, provider.clone())));
                    (wf, addr)
                },
                WalletFactoryAddresses::MSABasicFactoryAddress(addr) => {
                    let wf = WalletFactoryRegistry::MSABasicFactory(Box::new(MSAFactory::new(addr, provider.clone())));
                    (wf, addr)
                }
            };
        let wallet_contract: Box<dyn SmartWalletAccount> =
            match WalletRegistry::from_str(&wallet_name)? {
                WalletRegistry::SimpleAccount => {
                    Box::new(SimpleAccount::new(factory_address, provider.clone())) as _
                },
                WalletRegistry::MSABasicAccount => {
                    Box::new(MSABasic::new(factory_address, provider.clone())) as _
                }
            };
        Ok((wallet_contract, factory_contract, factory_address))
    }

    pub fn factory_contract_address(&self) -> Address {
        self.factory_address
    }

    pub fn factory_contract(&self) -> WalletFactoryRegistry<M> {
        self.factory_contract.clone()
    }

    pub fn wallet_contract(&self) -> Box<dyn SmartWalletAccount> {
        self.wallet_contract.clone_box()
    }

    pub fn signer_address(&self) -> Address {
        self.signer_address
    }

    pub fn scw_address(&self) -> Option<Address> {
        self.scw_address
    }

    pub fn salt(&self) -> Option<u64> {
        self.salt
    }

    pub fn uo(&self) -> &UserOperationPartial {
        &self.uo
    }

    pub fn uo_hash(&self) -> &Option<UserOperationHash> {
        &self.uo_hash
    }

    pub async fn set_scw_address(&mut self) -> anyhow::Result<Address> {
        let scw_address;
        match &self.factory_contract {
            WalletFactoryRegistry::SimpleAccountFactory(factory) => {
                let creator_address = self.factory_address;
                let salt = U256::from(self.salt.expect("salt is none"));
                scw_address = factory.generate_address(creator_address, salt).call().await?;
            },
            WalletFactoryRegistry::MSABasicFactory(factory) => {
                let hashed_salt = keccak256(self.salt.unwrap().to_be_bytes());
                let salt = H256::from(hashed_salt);
                let init_code = Bytes::new();
                scw_address = factory.get_address(salt, init_code).call().await?;
            },
        }
        self.scw_address = Some(scw_address);
        Ok(scw_address)
    }

    pub fn set_uo(&mut self, uo: UserOperationPartial) -> &mut Self {
        self.uo = uo;
        self
    }

    pub fn set_wallet(&mut self, wallet_name: String) -> anyhow::Result<&mut Self> {
        let (wallet_contract, factory_contract, factory_address) =
            Self::match_wallet(wallet_name, self.provider.clone())?;
        self.wallet_contract = wallet_contract;
        self.factory_contract = factory_contract;
        self.factory_address = factory_address;
        Ok(self)
    }

    pub fn set_uo_sender(&mut self, sender: Address) -> &mut Self {
        self.uo.sender = Some(sender);
        self
    }

    pub fn set_uo_nonce(&mut self, nonce: U256) -> &mut Self {
        self.uo.nonce = Some(nonce);
        self
    }

    pub fn set_uo_factory(&mut self, factory: Address) -> &mut Self {
        self.uo.factory = Some(factory);
        self
    }

    pub fn set_uo_factory_data(&mut self, factory_data: Bytes) -> &mut Self {
        self.uo.factory_data = Some(factory_data);
        self
    }

    pub fn set_uo_call_data(&mut self, call_data: Bytes) -> &mut Self {
        self.uo.call_data = Some(call_data);
        self
    }

    pub fn set_uo_call_gas_limit(&mut self, call_gas_limit: U256) -> &mut Self {
        self.uo.call_gas_limit = Some(call_gas_limit);
        self
    }

    pub fn set_uo_verification_gas_limit(&mut self, verification_gas_limit: U256) -> &mut Self {
        self.uo.verification_gas_limit = Some(verification_gas_limit);
        self
    }

    pub fn set_uo_pre_verification_gas(&mut self, pre_verification_gas: U256) -> &mut Self {
        self.uo.pre_verification_gas = Some(pre_verification_gas);
        self
    }

    pub fn set_uo_max_fee_per_gas(&mut self, max_fee_per_gas: U256) -> &mut Self {
        self.uo.max_fee_per_gas = Some(max_fee_per_gas);
        self
    }

    pub fn set_uo_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: U256) -> &mut Self {
        self.uo.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
        self
    }

    pub fn set_uo_paymaster(&mut self, paymaster: String) -> &mut Self {
        self.uo.paymaster = Some(paymaster);
        self
    }

    pub fn set_uo_paymaster_verification_gas_limit(&mut self, paymaster_verification_gas_limit: U256) -> &mut Self {
        self.uo.paymaster_verification_gas_limit = Some(paymaster_verification_gas_limit);
        self
    }

    pub fn set_uo_paymaster_post_op_gas_limit(&mut self, paymaster_post_op_gas_limit: U256) -> &mut Self {
        self.uo.paymaster_post_op_gas_limit = Some(paymaster_post_op_gas_limit);
        self
    }

    pub fn set_uo_paymaster_data(&mut self, paymaster_data: Bytes) -> &mut Self {
        self.uo.paymaster_data = Some(paymaster_data);
        self
    }

    pub fn set_uo_signature(&mut self, signature: Bytes) -> &mut Self {
        self.uo.signature = Some(signature);
        self
    }

    pub(crate) fn set_uo_hash(&mut self, uo_hash: UserOperationHash) -> &mut Self {
        self.uo_hash = Some(uo_hash);
        self
    }

    pub fn build_uo(&self) -> anyhow::Result<UserOperation> {

        if self.uo.sender.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("sender".to_string())
            ));
        };

        if self.uo.nonce.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("nonce".to_string())
            ));
        };

        if self.uo.factory.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("factory".to_string())
            ));
        };

        if self.uo.factory_data.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("factory_data".to_string())
            ));
        };

        if self.uo.call_data.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("call_data".to_string())
            ))
        };

        if self.uo.call_gas_limit.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("call_gas_limit".to_string())
            ));
        };

        if self.uo.verification_gas_limit.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("verification_gas_limit".to_string())
            ));
        };

        if self.uo.pre_verification_gas.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("pre_verification_gas".to_string())
            ));
        };

        if self.uo.max_fee_per_gas.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("max_fee_per_gas".to_string())
            ));
        };

        if self.uo.max_priority_fee_per_gas.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("max_priority_fee_per_gas".to_string())
            ));
        };

        if self.uo.paymaster.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("paymaster".to_string())
            ));
        };

        if self.uo.paymaster_verification_gas_limit.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("paymaster_verification_gas_limit".to_string())
            ));
        };

        if self.uo.paymaster_post_op_gas_limit.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("paymaster_post_op_gas_limit".to_string())
            ));
        };

        if self.uo.paymaster_data.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("paymaster_data".to_string())
            ));
        };

        if self.uo.signature.is_none() {
            return Err(anyhow::anyhow!(
                UserOpBuilderError::<M>::MissingUserOperationField("signature".to_string())
            ))
        };

        let uo = UserOperation::from(self.uo.clone());

        Ok(uo)
    }
}
