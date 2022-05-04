mod logbook;
mod logsetup;

use std::fs::File;
use std::io::{prelude::*, BufReader, BufWriter};

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use log::*;

use logbook::Logbook;
use logsetup::init_logger;

#[derive(Debug, Subcommand)]
enum Command {
    /// Read the given BMS logbook and print it as JSON
    Read {
        /// Pretty-print the JSON output
        #[clap(short, long)]
        pretty: bool,

        /// `*.lbk` to read
        logbook: Utf8PathBuf,
    },
    /// Read the given JSON and write it as a BMS logbook
    Write {
        /// JSON file to read, or `-` for stdin
        json: Utf8PathBuf,
    },
    /// Create a default logbook, commissioned today.
    WriteDefault {
        #[clap(short, long)]
        name: String,

        #[clap(short, long)]
        callsign: String,

        #[clap(short, long)]
        password: Option<String>,
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

    /// File to write to, or `-` for stdout
    #[clap(short, long)]
    output: Option<Utf8PathBuf>,

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

    let output = args.output.unwrap_or_else(|| Utf8PathBuf::from("-"));

    match args.command {
        Command::Read { pretty, logbook } => {
            let r = reader(&logbook)?;
            let book =
                Logbook::parse(r).with_context(|| format!("Couldn't parse logbook {logbook}"))?;

            let mut w = writer(&output)?;

            if pretty {
                writeln!(w, "{}", serde_json::to_string_pretty(&book)?)?;
            } else {
                writeln!(w, "{}", serde_json::to_string(&book)?)?;
            }

            w.flush()
                .with_context(|| format!("Couldn't flush JSON to {output}"))?;
        }
        Command::Write { json } => {
            let r = reader(&json)?;
            let book: Logbook =
                serde_json::from_reader(r).with_context(|| format!("Couldn't parse {json}"))?;

            let mut w = writer(&output)?;
            book.write(&mut w)?;

            w.flush()
                .with_context(|| format!("Couldn't flush logbook to {output}"))?;
        },
        Command::WriteDefault { name, callsign, password } => {
            let password = password.unwrap_or_default();
            let book = Logbook::new(name, callsign, password)?;

            let mut w = writer(&output)?;
            book.write(&mut w)?;

            w.flush()
                .with_context(|| format!("Couldn't flush logbook to {output}"))?;
        }
    }
    Ok(())
}

fn reader(path: &Utf8Path) -> Result<BufReader<Box<dyn Read>>> {
    let reader: Box<dyn Read> = match path.as_str() {
        "-" => Box::new(std::io::stdin()),
        p => {
            let f = File::open(p).with_context(|| format!("Couldn't read {p}"))?;
            Box::new(f)
        }
    };
    Ok(BufReader::new(reader))
}

fn writer(path: &Utf8Path) -> Result<BufWriter<Box<dyn Write>>> {
    let writer: Box<dyn Write> = match path.as_str() {
        "-" => Box::new(std::io::stdout()),
        p => {
            let f = File::create(p).with_context(|| format!("Couldn't write to {p}"))?;
            Box::new(f)
        }
    };

    Ok(BufWriter::new(writer))
}
