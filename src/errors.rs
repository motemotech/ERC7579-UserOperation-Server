use thiserror::Error;
use ethers::providers::Middleware;
// In the implementation of example ethers-userop, they import ethers but I can not confim Middleware trait in alloy.rs, so here I will skip to use middleware but maybe we need that in the future

#[derive(Debug, Clone, Error)]
pub enum UserOpMiddlewareError<M: Middleware> {
    #[error("Middleware error: {0}")]
    MiddlewareError(M::Error),

    #[error("Error occured durig smart contract wallet deployment")]
    SmartContractWalletDeploymentError,

    // #[error(transparent)]
    #[error(transparent)]
    UserOpBuilderError(UserOpBuilderError<M>),

    #[error("Pre-verification gas not enough: calculated: {0}, provided: {1}")]
    PreVerificationGasError(u64, u64),

    #[error("Call gas limit not enough: calculated: {0}, provided: {1}")]
    CallGasLimitError(u64, u64),

    #[error("Verification gas limit not enough")]
    VerificationGasLimitError,

    #[error("Unknown error")]
    UnknownError,
}

#[derive(Error, Clone, Debug)]
pub enum UserOpBuilderError<M: Middleware> {
    
    #[error("Middleware error: {0}")]
    MiddlewareError(M::Error),
   
    #[error("Smart contract wallet address has not been set")]
    SmartContractWalletAddressNotSet,

    #[error("Smart contract wallet has been deployed for the given counter-factual address")]
    SmartContractWalletHasBeenDeployed,

    #[error("Smart contract wallet has not been deployed for the given address. Cannot perform the current function call")]
    SmartContractWalletHasNotBeenDeployed,

    #[error("The field in the UserOperation is not set. Call the set_uo_{0} function to set")]
    MissingUserOperationField(String),

    #[error("Unknown error")]
    UnknownError,
}