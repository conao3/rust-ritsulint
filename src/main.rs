mod generated;
mod law;
mod read_as;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use law::Law;
use read_as::{Clause, Counts, Warning};

const EXIT_USAGE: u8 = 252;
const EXIT_INPUT: u8 = 254;
const EXIT_FOUND: u8 = 1;

#[derive(Clone, Copy, ValueEnum)]
enum Output {
    Json,
    Text,
    Table,
}

#[derive(Parser)]
#[command(
    name = "ritsulint",
    version,
    about = "A linter for Japanese statutes (e-Gov law XML)"
)]
struct Cli {
    #[arg(long, global = true, value_enum, default_value = "json")]
    output: Output,
    #[arg(long, global = true)]
    debug: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// A single law
    Law {
        #[command(subcommand)]
        command: LawCommand,
    },
    /// A directory of laws, as the e-Gov bulk download unpacks it
    Corpus {
        #[command(subcommand)]
        command: CorpusCommand,
    },
}

#[derive(Subcommand)]
enum LawCommand {
    /// Check one law XML file
    CheckLaw {
        #[arg(long)]
        xml_file: PathBuf,
        #[arg(long)]
        fail_on_warnings: bool,
    },
}

#[derive(Subcommand)]
enum CorpusCommand {
    /// Check the revision of every law in force on a date
    CheckCorpus {
        #[arg(long)]
        xml_dir: PathBuf,
        /// The date as YYYYMMDD; of each law, the latest revision enforced on or before it
        #[arg(long)]
        as_of: String,
        #[arg(long)]
        fail_on_warnings: bool,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Report {
    laws: u64,
    unreadable: Vec<String>,
    counts: Counts,
    warnings: Vec<Warning>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    clauses: Vec<Clause>,
}

fn revision_files(xml_dir: &Path, as_of: &str) -> Result<Vec<PathBuf>, String> {
    let mut latest: BTreeMap<String, String> = BTreeMap::new();
    let entries = fs::read_dir(xml_dir).map_err(|e| format!("{}: {e}", xml_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let mut parts = name.split('_');
        let (Some(law), Some(date), Some(_)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if date > as_of {
            continue;
        }
        let slot = latest.entry(law.to_string()).or_default();
        if *slot < name {
            *slot = name.clone();
        }
    }
    Ok(latest
        .into_values()
        .map(|rev| xml_dir.join(&rev).join(format!("{rev}.xml")))
        .collect())
}

fn check_files(files: &[PathBuf], debug: bool) -> Report {
    let mut report = Report {
        laws: 0,
        unreadable: Vec::new(),
        counts: Counts::default(),
        warnings: Vec::new(),
        clauses: Vec::new(),
    };
    for file in files {
        match Law::from_file(file) {
            Ok(law) => {
                report.laws += 1;
                let (counts, warnings, clauses) = read_as::check(&law);
                if debug {
                    report.clauses.extend(clauses);
                }
                report.counts.add(&counts);
                report.warnings.extend(warnings);
            }
            Err(e) => report.unreadable.push(e.to_string()),
        }
    }
    report
}

fn print_report(report: &Report, output: Output) {
    match output {
        Output::Json => println!(
            "{}",
            serde_json::to_string_pretty(report).expect("the report serializes")
        ),
        Output::Text => {
            for w in &report.warnings {
                println!(
                    "{}\t{}\t{}\t{}\t{}/{}\t{}\t{}\t{}",
                    w.id,
                    w.rule,
                    w.status,
                    w.law_id,
                    w.source.article,
                    w.source.paragraph,
                    w.scope,
                    w.quoted,
                    w.nearest
                );
            }
        }
        Output::Table => {
            println!("| Id | Status | Law | Source | Scope | Quoted | Nearest |");
            println!("|---|---|---|---|---|---|---|");
            for w in &report.warnings {
                println!(
                    "| {} | {} | {} ({}) | {}/{} | {} | {} | {} |",
                    w.id,
                    w.status,
                    w.law_title,
                    w.law_id,
                    w.source.article,
                    w.source.paragraph,
                    w.scope,
                    w.quoted,
                    w.nearest
                );
            }
        }
    }
}

fn run(cli: Cli) -> Result<u8, (u8, String)> {
    let (files, fail_on_warnings) = match cli.command {
        Command::Law {
            command:
                LawCommand::CheckLaw {
                    xml_file,
                    fail_on_warnings,
                },
        } => (vec![xml_file], fail_on_warnings),
        Command::Corpus {
            command:
                CorpusCommand::CheckCorpus {
                    xml_dir,
                    as_of,
                    fail_on_warnings,
                },
        } => {
            if as_of.len() != 8 || !as_of.chars().all(|c| c.is_ascii_digit()) {
                return Err((
                    EXIT_USAGE,
                    format!("--as-of takes a date as YYYYMMDD ({as_of} is not one)"),
                ));
            }
            (
                revision_files(&xml_dir, &as_of).map_err(|e| (EXIT_INPUT, e))?,
                fail_on_warnings,
            )
        }
    };
    let report = check_files(&files, cli.debug);
    if report.laws == 0 {
        let detail = report
            .unreadable
            .first()
            .cloned()
            .unwrap_or_else(|| "no law XML found".to_string());
        return Err((EXIT_INPUT, detail));
    }
    print_report(&report, cli.output);
    Ok(if fail_on_warnings && !report.warnings.is_empty() {
        EXIT_FOUND
    } else {
        0
    })
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(if e.use_stderr() { EXIT_USAGE } else { 0 });
        }
    };
    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err((code, message)) => {
            eprintln!("ritsulint: {message}");
            ExitCode::from(code)
        }
    }
}
