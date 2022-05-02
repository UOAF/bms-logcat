mod logbook;
mod logsetup;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use log::*;

use logsetup::init_logger;

#[derive(Debug, Subcommand)]
enum Command {
    /// Read the given logbook and print it to stdout
    Read { file: Utf8PathBuf },
    /// Read the given logbook JSON and print it to stdout
    Write { file: Utf8PathBuf },
}

/// Read and write Falcon BMS logbooks
#[derive(Parser, Debug)]
struct Args {
    /// Verbosity (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    verbose: u8,

    #[clap(short, long, arg_enum, default_value = "auto")]
    color: logsetup::Color,

    #[clap(subcommand)]
    command: Command,
}

fn main() {
    run().unwrap_or_else(|e| {
        error!("{:?}", e);
        std::process::exit(1);
    });
}

fn run() -> Result<()> {
    let args = Args::parse();
    init_logger(args.verbose, args.color);

    match args.command {
        Command::Read { file } => {
            let book = logbook::read(&file)?;
            println!("{:#?}", book);
        }
        Command::Write { file: _ } => {}
    }
    Ok(())
}