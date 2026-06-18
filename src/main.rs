//! corpus-introspect — the multinode self-mirror.
//!
//! Synthesises attested membership, per-node activity, self-state convergence,
//! and held leases — plus link health — into a single human-readable picture
//! of the entire wintermute entity.

use std::process;

use clap::Parser;

mod facets;
mod model;
mod render;
mod runner;

use model::WholeSelf;

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    /// Human-readable self-portrait (default)
    Text,
    /// Machine-readable JSON (`WholeSelf` record)
    Json,
    /// Parseable block for the self-review playbook
    Selfreview,
}

/// corpus-introspect — synthesise all corpus facets into a single self-portrait.
#[derive(Debug, Parser)]
#[command(name = "corpus-introspect", version, about)]
struct Cli {
    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    format: OutputFormat,

    /// Equivalent to --format json
    #[arg(long, conflicts_with = "format")]
    json: bool,
}

fn main() {
    // SIGPIPE must be reset first — per bad-rust rules and PRD AC7.
    sigpipe::reset();

    let cli = Cli::parse();

    let format = if cli.json {
        OutputFormat::Json
    } else {
        cli.format
    };

    let whole = WholeSelf::collect();

    match format {
        OutputFormat::Json => {
            match serde_json::to_string_pretty(&whole) {
                Ok(s) => println!("{s}"),
                Err(e) => {
                    eprintln!("corpus-introspect: serialisation error: {e}");
                    process::exit(1);
                }
            }
        }
        OutputFormat::Text => {
            render::print_text(&whole);
        }
        OutputFormat::Selfreview => {
            render::print_selfreview(&whole);
        }
    }
}
