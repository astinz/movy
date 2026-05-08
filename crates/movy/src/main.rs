use clap::{Parser, Subcommand};

#[cfg(feature = "analysis")]
use crate::analysis::AnlaysisArgs;
#[cfg(feature = "aptos")]
use crate::aptos::AptosArgs;
#[cfg(any(
    feature = "sui-trace",
    feature = "sui-fuzz",
    feature = "sui-replay",
    feature = "sui-static-analysis"
))]
use crate::sui::SuiArgs;

#[cfg(feature = "analysis")]
mod analysis;
#[cfg(feature = "aptos")]
mod aptos;
#[cfg(any(
    feature = "sui-trace",
    feature = "sui-fuzz",
    feature = "sui-replay",
    feature = "sui-static-analysis"
))]
mod sui;

#[derive(Subcommand)]
pub enum MovySubcommand {
    #[cfg(any(
        feature = "sui-trace",
        feature = "sui-fuzz",
        feature = "sui-replay",
        feature = "sui-static-analysis"
    ))]
    Sui(SuiArgs),
    #[cfg(feature = "analysis")]
    Analysis(AnlaysisArgs),
    #[cfg(feature = "aptos")]
    Aptos(AptosArgs),
    #[cfg(not(any(
        feature = "sui-trace",
        feature = "sui-fuzz",
        feature = "sui-replay",
        feature = "sui-static-analysis",
        feature = "analysis",
        feature = "aptos"
    )))]
    #[command(hide = true)]
    NoFeatures,
}

#[derive(Parser)]
pub struct MovyCommand {
    #[clap(subcommand)]
    pub cmd: MovySubcommand,
}

async fn main_entry() {
    let args = MovyCommand::parse();
    match args.cmd {
        #[cfg(any(
            feature = "sui-trace",
            feature = "sui-fuzz",
            feature = "sui-replay",
            feature = "sui-static-analysis"
        ))]
        MovySubcommand::Sui(args) => args.run().await.expect("sui command failed"),
        #[cfg(feature = "analysis")]
        MovySubcommand::Analysis(args) => args.run().await.expect("analysis failed"),
        #[cfg(feature = "aptos")]
        MovySubcommand::Aptos(args) => args.run().await.expect("aptos command failed"),
        #[cfg(not(any(
            feature = "sui-trace",
            feature = "sui-fuzz",
            feature = "sui-replay",
            feature = "sui-static-analysis",
            feature = "analysis",
            feature = "aptos"
        )))]
        MovySubcommand::NoFeatures => {
            eprintln!("No movy command features are enabled for this build")
        }
    }
}

fn main() {
    color_eyre::install().expect("Fail to install color_eyre");
    if let Ok(dot_file) = std::env::var("DOT") {
        dotenvy::from_path(dot_file).expect("fail to import");
    } else {
        // Allows failure
        let _ = dotenvy::dotenv();
    }
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("can not build runtime")
        .block_on(main_entry())
}
