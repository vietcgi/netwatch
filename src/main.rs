use anyhow::Result;
use clap::Parser;
use netwatch_rs::{cli::Args, run};

fn main() -> Result<()> {
    let args = Args::parse();
    run(args)
}
