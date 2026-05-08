use clap::{Args, Subcommand};
use movy_types::error::MovyError;

#[cfg(feature = "sui-fuzz")]
use crate::sui::fuzz::SuiFuzzArgs;
#[cfg(feature = "sui-replay")]
use crate::sui::replay::SuiReplaySeedArgs;
#[cfg(feature = "sui-static-analysis")]
use crate::sui::static_analysis::SuiStaticAnalysisArgs;
#[cfg(feature = "sui-trace")]
use crate::sui::trace::SuiTraceArgs;

#[cfg(feature = "sui-trace")]
pub mod env;
#[cfg(feature = "sui-fuzz")]
pub mod fuzz;
#[cfg(feature = "sui-replay")]
pub mod replay;
#[cfg(feature = "sui-static-analysis")]
pub mod static_analysis;
#[cfg(feature = "sui-trace")]
pub mod trace;
#[cfg(any(
    feature = "sui-fuzz",
    feature = "sui-replay",
    feature = "sui-static-analysis"
))]
pub mod utils;

#[derive(Subcommand)]
pub enum SuiSubcommand {
    #[cfg(feature = "sui-trace")]
    TraceTx(SuiTraceArgs),
    #[cfg(feature = "sui-fuzz")]
    Fuzz(SuiFuzzArgs),
    #[cfg(feature = "sui-replay")]
    ReplaySeed(SuiReplaySeedArgs),
    #[cfg(feature = "sui-static-analysis")]
    StaticAnalysis(SuiStaticAnalysisArgs),
}

#[derive(Args)]
pub struct SuiArgs {
    #[clap(subcommand)]
    pub cmd: SuiSubcommand,
}

impl SuiArgs {
    pub async fn run(self) -> Result<(), MovyError> {
        match self.cmd {
            #[cfg(feature = "sui-trace")]
            SuiSubcommand::TraceTx(args) => args.run().await?,
            #[cfg(feature = "sui-fuzz")]
            SuiSubcommand::Fuzz(args) => args.run().await?,
            #[cfg(feature = "sui-static-analysis")]
            SuiSubcommand::StaticAnalysis(args) => args.run().await?,
            #[cfg(feature = "sui-replay")]
            SuiSubcommand::ReplaySeed(args) => args.run().await?,
        }
        Ok(())
    }
}
