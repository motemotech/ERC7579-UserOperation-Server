use alloy::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder};

pub fn build_wallet(seed: &str) -> anyhow::Result<LocalWallet> {
    let wallet = MnemonicBuilder::<English>::defautl().phrase(seed).build()?;
    Ok(wallet)
}