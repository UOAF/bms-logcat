mod logbook;
mod logsetup;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use log::*;

use logsetup::init_logger;

#[derive(Debug, Subcommand)]
enum Command {
    /// Read the given logbook and print it to the given file, or `-` for stdout
    Read {
        /// Pretty-print the JSON output
        #[clap(short, long)]
        pretty: bool,

        /// `*.lbk` to read
        logbook: Utf8PathBuf,
    },
    /// Read the given JSON and write it back out as a logbook
    Write {
        /// File to write to, or `-` for stdout
        #[clap(short, long)]
        output: Utf8PathBuf,

        /// JSON file to read, or `-` for stdin
        json: Utf8PathBuf,
    },
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
        Command::Read { pretty, logbook } => {
            let book = logbook::read(&logbook)?;
            if pretty {
                println!("{}", serde_json::to_string_pretty(&book)?);
            } else {
                println!("{}", serde_json::to_string(&book)?);
            }
        }
        Command::Write { output, json } => {
            let f: Box<dyn std::io::Read> = if json.as_str() == "-" {
                Box::new(std::io::stdin())
            } else {
                Box::new(
                    std::fs::File::open(&json).with_context(|| format!("Couldn't open {json}"))?,
                )
            };
            let book: logbook::Logbook = serde_json::from_reader(std::io::BufReader::new(f))
                .with_context(|| format!("Couldn't parse {json}"))?;
            logbook::write(&book, &output)?;
        }
    }
    Ok(())
}
