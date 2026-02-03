use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// revoke and clear tokens stored in keyrings
    Clear {
        /// forcefully clear tokens from keyrings even if revocation fails
        #[arg(short, long)]
        force: bool,
    },
}
