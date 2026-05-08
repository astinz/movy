use clap::Args;
use color_eyre::eyre::eyre;
use movy_types::error::MovyError;

#[derive(Args)]
pub struct AptosArgs {}

impl AptosArgs {
    pub async fn run(self) -> Result<(), MovyError> {
        let _ = self;
        Err(eyre!("Aptos support is feature-gated but not implemented yet").into())
    }
}
