use crate::{
    errors::{UserOpMiddlewareError}, gen::SimpleAccount, traits::SmartWalletAccount, types::{ErrorResponse, EstimateResult, Request, Response, WalletMap}, uo_builder::UserOperationBuilder
};
use async_trait::async_trait;
use ethers::{
    contract::abigen, providers::{Middleware, MiddlewareError}, signers::{LocalWallet as Wallet, Signer}, types::{transaction::eip2718::TypedTransaction, Address, Bytes, H256, U256}
};
use hashbrown::HashMap;
use parking_lot::Mutex;
use rand::Rng;
use regex::Regex;
use serde_json::json;
use crate::primitives::user_operation::{UserOperation, UserOperationHash, UserOperationPartial, UserOperationReceipt};
use std::fmt;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::error::Error;


#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

abigen!(EntryPoint, "src/abi/EntryPoint.json",);
abigen!(
    MSABasic, 
    "./src/abi/MSABasic.json"
);

#[derive(Clone)]
pub struct UserOpMiddleware<M> {
    pub inner: M,
    pub entry_point_address: Address,
    pub rpc_address: String,
    pub chain_id: u64,
    #[doc(hidden)]
    pub wallet: Wallet,
    pub wallet_map: WalletMap,
    pub sender: Address,
    pub validator: Address,
    pub factory: Address,
    pub bootstrap: Address,
}

impl<M: Middleware + 'static + fmt::Debug + Clone> fmt::Debug for UserOpMiddleware<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserOpMiddleware")
            .field("inner", &self.inner)
            .field("entry_point_address", &self.entry_point_address)
            .field("rpc_address", &self.rpc_address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}
impl<M: Middleware + 'static + fmt::Debug + Clone> MiddlewareError for UserOpMiddlewareError<M> {
    type Inner = M::Error;

    fn from_err(src: M::Error) -> Self {
        UserOpMiddlewareError::MiddlewareError(src)
    }

    fn as_inner(&self) -> Option<&Self::Inner> {
        match self {
            UserOpMiddlewareError::MiddlewareError(e) => Some(e),
            _ => None,
        }
    }
}

#[async_trait]
impl<M: Middleware + 'static + fmt::Debug + Clone> Middleware for UserOpMiddleware<M> {
    type Error = UserOpMiddlewareError<M>;
    type Provider = M::Provider;
    type Inner = M;

    fn inner(&self) -> &M {
        &self.inner
    }
}
impl<M: Middleware + 'static + fmt::Debug + Clone> UserOpMiddleware<M> {
    pub fn new(
        inner: M,
        entry_point_address: Address,
        rpc_address: impl Into<String>,
        wallet: Wallet,
        sender: Address,
        validator: Address,
        factory: Address,
        bootstrap: Address,
    ) -> Self {
        let chain_id = wallet.chain_id();

        let wallet_account = Box::new(SimpleAccount::new(Address::default(), inner.clone().into()));
        let wallet_contract: Box<dyn SmartWalletAccount> = wallet_account;
        let mut wallet_map = HashMap::new();
        wallet_map.insert(Address::default(), Arc::new(Mutex::new(wallet_contract)));

        Self {
            inner,
            entry_point_address,
            rpc_address: rpc_address.into(),
            chain_id,
            wallet,
            wallet_map,
            sender,
            validator,
            factory,
            bootstrap
        }
    }

    #[allow(dead_code)]
    fn entry_point_address(&self) -> &Address {
        &self.entry_point_address
    }

    #[allow(dead_code)]
    fn rpc_address(&self) -> &String {
        &self.rpc_address
    }

    #[allow(dead_code)]
    fn wallet(&self) -> &Wallet {
        &self.wallet
    }

    #[allow(dead_code)]
    fn wallet_map(&self) -> &WalletMap {
        &self.wallet_map
    }

    pub async fn estimate_user_operation_gas(
        &self,
        user_operation: &UserOperationPartial
    ) -> anyhow::Result<Response<EstimateResult>> {
        let params = vec![json!(user_operation), json!(self.entry_point_address)];
        let req_body = Request {
            jsonrpc: "2.0".to_string(),
            method: "eth_estimateUserOperationGas".to_string(),
            params: params.clone(),
            id: 1,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_address)
            .json(&req_body)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn send_user_operation(
        &self,
        user_operation: &UserOperationPartial
    ) -> anyhow::Result<Response<H256>> {
        let req_body = Request {
            jsonrpc: "2.0".to_string(),
            method: "eth_sendUserOperation".to_string(),
            params: vec![json!(user_operation), json!(self.entry_point_address)],
            id: 1,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_address)
            .json(&req_body)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn get_nonce(
        &self,
    ) -> anyhow::Result<U256> {

        let mut padded_bytes = [0u8; 32];
        padded_bytes[8..28].copy_from_slice(self.validator.as_bytes());
        let validator_for_input = U256::from_big_endian(&padded_bytes);

        let nonce = EntryPoint::new(self.entry_point_address, self.inner.clone().into())
                .get_nonce(self.sender, validator_for_input)
                .call()
                .await?;

        Ok(nonce)
    }

    pub fn calldata_gen_send_eth(
        &self,
        to_address: Address,
        value: U256,
    ) -> anyhow::Result<Bytes> {
        let mut mode_code_single = [0u8; 32];

        let mut execution_calldata = Vec::new();
        execution_calldata.extend_from_slice(&to_address.as_bytes());

        let mut value_bytes = [0u8; 32];
        value.to_big_endian(&mut value_bytes);
        execution_calldata.extend_from_slice(&value_bytes);

        execution_calldata.extend_from_slice(&[0u8]);

        let calldata_for_wallet = MSABasic::new(self.sender, self.inner.clone().into())
            .encode("execute", (mode_code_single, Bytes::from(execution_calldata)))?;

        Ok(calldata_for_wallet)
    }

    pub async fn uogen_send_eth(
        &self,
        to_address: Address,
        value: U256,
    ) -> anyhow::Result<UserOperationPartial> {
        let nonce = self.get_nonce().await?;
        let calldata = self.calldata_gen_send_eth(to_address, value).unwrap();
        let mut user_operation = UserOperationPartial {
            sender: Some(self.sender,),
            nonce: Some(U256::from(nonce), ),
            factory: None,
            factory_data: None,
            call_data: Some(calldata,),
            call_gas_limit: Some(U256::from(1_000_000_000u64),),
            verification_gas_limit: Some(U256::from(1_000_000_000u64),),
            pre_verification_gas: Some(U256::from(1_000_000_000u64),),
            max_fee_per_gas: Some(U256::from(1_000_000_000u64),),
            max_priority_fee_per_gas: Some(U256::from(1_000_000_000u64),),
            paymaster: None,
            paymaster_verification_gas_limit: None,
            paymaster_post_op_gas_limit: None,
            paymaster_data: None,
            signature: Some(Bytes::default(),),
        };

        let estimated_gas = self.estimate_user_operation_gas(&user_operation).await.unwrap();

        let avg_gas_price = self.get_gas_fee().await?;

        user_operation.call_gas_limit = Some(estimated_gas.result.call_gas_limit, );
        user_operation.verification_gas_limit = Some(estimated_gas.result.verification_gas_limit, );
        user_operation.pre_verification_gas = Some(estimated_gas.result.pre_verification_gas, );
        user_operation.max_fee_per_gas = Some(avg_gas_price.0, );
        user_operation.max_priority_fee_per_gas = Some(avg_gas_price.1, );

        Ok(user_operation)
    
    }

    // pub fn get_factory_data(
    //     &self,
    //     salt: U256,
    // ) -> anyhow::Result<Bytes> {
        // TODO: need to add a function which make calldata to create account
        // let bootstrap_contract = Bootstrap::new(bootstrap, contract_provider.clone());
        // let validators: Vec<BootstrapConfig> = vec![
        //     BootstrapConfig {
        //         module: validator,
        //         data: Bytes::default(),
        //     },
        // ];
        // let executors: Vec<BootstrapConfig> = vec![];
        // let hook = BootstrapConfig {
        //     module: Address::zero(), 
        //     data: Bytes::default(),      
        // };
        // let fallbacks: Vec<BootstrapConfig> = vec![];

        // let result: Bytes = bootstrap_contract
        //     .get_init_msa_calldata(validators, executors, hook, fallbacks)
        //     .call()
        //     .await?;

        // let factory_contract = MSAFactory::new(factory_address.clone(), contract_provider.clone());
        // let factory_data = factory_contract
        //     .method::<(H256, Bytes), Address>("createAccount", (salt.clone(), result.clone()))?
        //     .calldata()
        //     .unwrap();

        // let factory_contract = MSAFactory::new(factory_address.clone(), contract_provider.clone());
        // let factory_data = factory_contract
        //     .method::<(H256, Bytes), Address>("createAccount", (salt.clone(), result.clone()))?
        //     .calldata()
        //     .unwrap();
    // }

    pub fn supported_entry_point(&self) -> Address {
        self.entry_point_address
    }

    pub async fn ask_supported_entry_point(&self) -> Result<JsonRpcResponse<serde_json::Value>, Box<dyn Error>> {

        let params = json!([]);

        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "eth_supportedEntryPoints".to_string(),
            params: params,
            id: 1,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_address)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Box::from(format!("Failed Http request, status code: {}", response.status())));
        }

        let response_json: JsonRpcResponse<serde_json::Value> = response.json().await?;

        Ok(response_json)
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub async fn get_user_operation_receipt(
        &self,
        user_operation_hash: &UserOperationHash,
    ) -> anyhow::Result<UserOperationReceipt> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_address)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getUserOperationReceipt",
                "params": vec![json!(user_operation_hash)],
                "id": 1,
            }))
            .send()
            .await?
            .json::<Response<UserOperationReceipt>>()
            .await?;

        Ok(response.result)
    }

    pub async fn get_user_operation_by_hash(
        &self,
        user_operation_hash: &UserOperationHash,
    ) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.rpc_address)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getUserOperationByHash",
                "params": vec![json!(user_operation_hash)],
                "id": 1,
            }))
            .send()
            .await?
            .json::<Response<String>>()
            .await?;
        
        Ok(response.result)
    }

    async fn handle_response<R>(response: reqwest::Response) -> anyhow::Result<Response<R>>
    where
        R: std::fmt::Debug + serde::de::DeserializeOwned,
    {
        let str_response = response.text().await?;
        println!("str: {:?}", str_response.clone());
        let parsed_response: anyhow::Result<Response<R>> =
            serde_json::from_str(&str_response).map_err(anyhow::Error::from);
        println!("parsed response: {:?}", parsed_response);

        match parsed_response {
            Ok(success_response) => {
                log::info!("Success {:?}", success_response);
                Ok(success_response)
            }
            Err(_) => {
                let error_response: ErrorResponse = serde_json::from_str(&str_response)?;
                log::warn!("Error: {:?}", error_response);
                let error_message = &error_response.error.message;

                if let Some(captures) =
                    Regex::new(r"Call gas limit (\d+) is lower than call gas estimation (\d+)")
                        .unwrap()
                        .captures(error_message)
                {
                    let limit: u64 = captures[1].parse().unwrap();
                    let estimation: u64 = captures[2].parse().unwrap();
                    return Err(anyhow::anyhow!(
                        UserOpMiddlewareError::<M>::CallGasLimitError(limit, estimation,)
                    ));
                }

                if let Some(captures) = Regex::new(r"Pre-verification gas (\d+) is lower than calculated pre-verification gas (\d+)")
                    .unwrap()
                    .captures(error_message)
                {
                    let pre_verification_gas: u64 = captures[1].parse().unwrap();
                    let calculated_gas: u64 = captures[2].parse().unwrap();
                    return Err(anyhow::anyhow!(
                        UserOpMiddlewareError::<M>::PreVerificationGasError(pre_verification_gas, calculated_gas)
                    ));
                }

                if error_message.contains("AA40 over verificationGasLimit") {
                    return Err(anyhow::anyhow!(
                        UserOpMiddlewareError::<M>::VerificationGasLimitError
                    ));
                }
                println!("{}", error_message);
                Err(anyhow::anyhow!(UserOpMiddlewareError::<M>::UnknownError))
            }
        }
    }

    pub async fn uo_calldata_from_tx(
        &self,
        tx: TypedTransaction,
    ) -> anyhow::Result<(Bytes, Address, U256)> {
        let calldata: Bytes;
        let dest: Address;
        let value: U256;
        match tx {
            TypedTransaction::Eip1559(tx_req) => {
                calldata = tx_req.data.expect("No `data` in transaction request");
                dest = *tx_req
                    .to
                    .expect("No `to` address in transaction request")
                    .as_address()
                    .unwrap();
                value = tx_req.value.expect("No `value@ in transaction request");
            }
            TypedTransaction::Legacy(tx_req) => {
                calldata = tx_req.data.expect("No `data` in transaction request");
                dest = *tx_req
                    .to
                    .expect("No `to` address in transaction request")
                    .as_address()
                    .unwrap();
                value = tx_req.value.expect("No `value` in transaction request");
            }
            TypedTransaction::Eip2930(tx_req) => {
                calldata = tx_req.tx.data.expect("No `data` in transaction request");
                dest = *tx_req
                    .tx
                    .to
                    .expect("No `to` address in transaction request")
                    .as_address()
                    .unwrap();
                value = tx_req.tx.value.expect("No `value` in transaction request");
            }
        };

        Ok((calldata, dest, value))
    }

    pub fn build_random_uo_builder(
        &self,
        wallet_name: String,
    ) -> anyhow::Result<UserOperationBuilder<M>> {
        let sender_address = self.wallet.address();
        let salt = rand::thread_rng().gen::<u64>();

        UserOperationBuilder::new(
            sender_address,
            wallet_name,
            None,
            self.inner.clone().into(),
            Some(salt),
        )
    }

    pub async fn sign_uo(&self, uo: UserOperation) -> anyhow::Result<UserOperation> {
        let h = uo.hash(&self.entry_point_address, &U256::from(self.chain_id));
        let sig = self.wallet.sign_message(h.0.as_bytes()).await?;
        let res_uo = uo.clone().signature(sig.to_vec().into());
        Ok(res_uo)
    }

    pub async fn get_gas_fee(&self) -> anyhow::Result<(U256, U256)> {
        let latest_block_number = self.provider().get_block_number().await?;
        let latest_block = self.provider().get_block_with_txs(latest_block_number).await?;
        let transactions = latest_block.unwrap().transactions;

        let mut total_max_fee_per_gas = U256::from(0);
        let mut total_max_priority_fee_per_gas = U256::from(0);
        let mut tx_count = 0;

        for tx in transactions {
            if let (Some(max_fee_per_gas), Some(max_priority_fee_per_gas)) = (tx.max_fee_per_gas, tx.max_priority_fee_per_gas) {
                // u64 から BigInt に変換
                let max_fee_per_gas_value = U256::from(max_fee_per_gas);
                let max_priority_fee_per_gas_value = U256::from(max_priority_fee_per_gas);
    
                // 合計値を更新
                total_max_fee_per_gas += max_fee_per_gas_value;
                total_max_priority_fee_per_gas += max_priority_fee_per_gas_value;
                tx_count += 1;
            }
        }
    
        let avg_max_fee_per_gas = total_max_fee_per_gas / tx_count;
        let avg_max_priority_fee_per_gas = total_max_priority_fee_per_gas / tx_count;
    
        Ok((avg_max_fee_per_gas, avg_max_priority_fee_per_gas))

        
    }

}
