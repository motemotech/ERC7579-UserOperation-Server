use crate::traits::{SmartWalletAccount, SmartWalletAccountFactory, MSABasicFactory};
use alloy::{
    primitives::{Address as a_Address, U256 as a_U256},
    sol,
    core::sol_types::SolCall,
};
use ethers::{
    contract::abigen,
    prelude::FunctionCall,
    providers::Middleware,
    types::{Address, Bytes, U256, H256},
};
use std::sync::Arc;

abigen!(SimpleAccountFactory, "src/abi/SimpleAccountFactory.json",);
abigen!(MSAFactory, "src/abi/MSAFactory.json",);
abigen!(SimpleAccount, "src/abi/SimpleAccount.json",);
abigen!(MSABasic, "src/abi/MSABasic.json",);
abigen!(EntryPoint, "src/abi/EntryPoint.json",);

sol! {function execute(address dest, uint256 value, bytes calldata func);}
pub struct SimpleAccountExecute(executeCall);
impl SimpleAccountExecute {
    pub fn new(address: Address, value: U256, func: Bytes) -> Self {
        Self(executeCall {
            dest: a_Address::from(address.0),
            value: a_U256::from_limbs(value.0),
            func: func.to_vec().into(),
        })
    }

    /// Encodes the calldata
    pub fn encode(&self) -> Vec<u8> {
        self.0.abi_encode()
    }
}

impl<M: Middleware + 'static> SmartWalletAccountFactory<M> for SimpleAccountFactory<M> {

    fn create_account(
        &self,
        creator_address: Address,
        salt: U256,
    ) -> FunctionCall<Arc<M>, M, Address> {
        self.create_account(creator_address, salt)
    }

    fn generate_address(
        &self,
        creator_address: Address,
        salt: U256,
    ) -> FunctionCall<Arc<M>, M, Address> {
        self.get_address(creator_address, salt)
    }

    fn clone_box(&self) -> Box<dyn SmartWalletAccountFactory<M>> {
        Box::new(self.clone())
    }
}

impl<M: Middleware + 'static> MSABasicFactory<M> for MSAFactory<M> {

    fn create_account(
        &self,
        salt: H256,
        init_code: Bytes,
    ) -> FunctionCall<Arc<M>, M, Address> {
        self.create_account(salt.into(), init_code)
    }

    fn get_address(
        &self,
        salt: H256,
        init_code: Bytes,
    ) -> FunctionCall<Arc<M>, M, Address> {
        self.get_address(salt.into(), init_code)
    }

    fn clone_box(&self) -> Box<dyn MSABasicFactory<M>> {
        Box::new(self.clone())
    }
}

impl<M: Middleware + 'static> SmartWalletAccount for SimpleAccount<M> {

    fn execute(&self, dest: Address, value: U256, func: Bytes) -> Vec<u8> {
        let sae = SimpleAccountExecute::new(dest, value, func);
        sae.0.abi_encode()
    }

    fn clone_box(&self) -> Box<dyn SmartWalletAccount> {
        Box::new(self.clone())
    }
}

impl<M: Middleware + 'static> SmartWalletAccount for MSABasic<M> {
    
    fn execute(&self, dest: Address, value: U256, func: Bytes) -> Vec<u8> {
        let exec = SimpleAccountExecute::new(dest, value, func);
        exec.0.abi_encode()
    }

    fn clone_box(&self) -> Box<dyn SmartWalletAccount> {
        Box::new(self.clone())
    }

}
