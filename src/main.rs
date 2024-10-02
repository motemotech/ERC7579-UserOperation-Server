use ethers::{
    signers::{LocalWallet, Signer},
    providers::{Provider, Http},
    types::{Bytes, U256, Address, H256},
    prelude::{abigen},
};
use std::{
    env,
    sync::Arc
};
use anyhow::Result;

mod uo_builder;
mod gen;
mod errors;
mod types;
mod consts;
mod traits;
mod primitives;
mod userop_middleware;
// mod ERC7579Calldata;
use primitives::user_operation::{UserOperation, UserOperationPartial};
use userop_middleware::UserOpMiddleware;
use dotenv::dotenv;
use crate::consts::{ENTRY_POINT_SEPOLIA_V7,};
use serde::{Deserialize, Serialize};
use std::error::Error;
use serde_json::Value;

use crate::userop_middleware::JsonRpcResponse;

abigen!(
    Bootstrap,
    "./src/abi/Bootstrap.json"
);
abigen!(
    WalletContract, 
    "./src/abi/MSABasic.json"
);

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY not found");
    let wallet: LocalWallet = private_key.parse().expect("Invalid private key");

    let rpc_url = env::var("SEPOLIA_RPC_ENDPOINT").expect("SEPOLIA_RPC_ENDPOINT not found");
    let provider =  Provider::try_from(rpc_url.clone())?;
    let bundler_rpc_url = env::var("PIMLICO_SEPOLIA_ENDPOINT").expect("SEPOLIA_RPC_ENDPOINT not found");

    let sender:Address = env::var("SENDER_ADDRESS").expect("SENDER_ADDRESS not found").parse()?;
    let validator:Address = env::var("VALIDATOR_ADDRESS").expect("VALIDATOR_ADDRESS not found").parse()?;
    let factory :Address = env::var("FACTORY_ADDRESS").expect("FACTORY_ADDRESS not found").parse()?;
    let bootstrap:Address= env::var("BOOTSTRAP_ADDRESS").expect("BOOTSTRAP_ADDRESS not found").parse()?;

    let mut uo_middleware: UserOpMiddleware<Provider<Http>> = UserOpMiddleware::new(
        provider.clone(),
        ENTRY_POINT_SEPOLIA_V7.parse::<Address>().unwrap(),
        bundler_rpc_url,
        wallet.clone(),
        sender,
        validator,
        factory,
        bootstrap
    );

    let to_address: Address = "0xc0c374f049f2e0036B48D93346038f0133B8f00F".parse()?;
    let value = U256::from(1000000000000000u64);

    let user_operation = uo_middleware.uogen_send_eth(to_address, value).await?;

    // send user operation
    let sent = uo_middleware.send_user_operation(&user_operation).await?;

    println!("send! : {:?}", sent);
    Ok(())
}
