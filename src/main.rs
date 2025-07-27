use anyhow::Result;
use clap::Parser;
use netwatch::{cli::Args, run};

fn main() -> Result<()> {
    let args = Args::parse();
    run(args)
}
