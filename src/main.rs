//! funclens — rank the functions in a project by line count.
//!
//! Walks a directory, finds every function definition across the supported
//! languages, and prints the longest `n` of them. The interesting work lives in
//! [`scan`] and [`language`]; this file is just the command-line shell around
//! them: parse arguments, call [`scan::scan`], rank, and print.

mod language;
mod scan;

use std::path::PathBuf;

use clap::Parser;

use scan::{CountMode, ExtFilter, Function};

/// Report the top functions by line count in a source tree.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Directory (or file) to scan.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// How many functions to show.
    #[arg(short, long, default_value_t = 20)]
    number: usize,

    /// Only scan these extensions (comma-separated, no dots), e.g. `rs,py`.
    #[arg(long, value_delimiter = ',')]
    include: Vec<String>,

    /// Never scan these extensions (comma-separated, no dots).
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// Count only lines of code, excluding blank and comment-only lines.
    #[arg(long)]
    logical: bool,
}

fn main() {
    let cli = Cli::parse();
    let filter = ExtFilter { include: cli.include, exclude: cli.exclude };
    let mode = if cli.logical { CountMode::Logical } else { CountMode::Physical };

    let mut functions = scan::scan(&cli.path, &filter, mode);
    // Longest first; ties broken by name then path for a stable, readable order.
    functions.sort_by(|a, b| {
        b.lines
            .cmp(&a.lines)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.path.cmp(&b.path))
    });
    functions.truncate(cli.number);

    print_table(&functions);
}

/// Print one function per line as `<lines> <name> <path>`, with the line counts
/// right-aligned in a column sized to the widest value present.
fn print_table(functions: &[Function]) {
    let width = functions
        .iter()
        .map(|f| f.lines)
        .max()
        .map_or(0, |max| max.to_string().len());

    for f in functions {
        println!("{:>width$} {} {}", f.lines, f.name, f.path.display());
    }
}
