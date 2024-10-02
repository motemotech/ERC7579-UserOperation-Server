use alloy::{
    primitives::{Address as a_Address, U256 as a_U256},
    sol,
    core::sol_types::SolCall,
};
use ethers::{
    prelude::FunctionCall,
    providers::Middleware,
    types::{Address, Bytes, H160, U256, H256},
};
use std::sync::Arc;
use std::fmt::Debug;

pub trait SmartWalletAccountFactory<M: Middleware>: Debug {
    fn create_account(&self, creator_address: Address, salt: U256)
        -> FunctionCall<Arc<M>, M, H160>;

    fn generate_address(
        &self,
        creator_address: Address,
        salt: U256
    ) -> FunctionCall<Arc<M>, M, H160>;

    fn clone_box(&self) -> Box<dyn SmartWalletAccountFactory<M>>;
}
pub trait MSABasicFactory<M: Middleware>: Debug {
    fn create_account(&self, salt: H256, init_code: Bytes)
        -> FunctionCall<Arc<M>, M, H160>;
    
    fn get_address(
        &self,
        salt: H256,
        init_code: Bytes,
    ) -> FunctionCall<Arc<M>, M, H160>;

    fn clone_box(&self) -> Box<dyn MSABasicFactory<M>>;
}

sol! {function execute(address dest, uint256 value, bytes calldata func);}
pub trait SmartWalletAccount: Debug + Send {
    fn execute(&self, dest: Address, value: U256, func: Bytes) -> Vec<u8> {
        let call = executeCall {
            dest: a_Address::from(dest.0),
            value: a_U256::from_limbs(value.0),
            func: func.to_vec().into(),
        };
        call.abi_encode()
    }

    fn clone_box(&self) -> Box<dyn SmartWalletAccount>;
}
