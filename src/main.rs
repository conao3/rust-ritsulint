mod external;
mod generated;
mod history;
mod law;
mod read_as;
mod relative_ref;
mod structure;

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
    /// The revisions of laws, as the e-Gov API serves them
    History {
        #[command(subcommand)]
        command: HistoryCommand,
    },
}

#[derive(Subcommand)]
enum HistoryCommand {
    /// For each warning of the latest revision of every law, the revision it is given since
    ListOrigins {
        /// A directory with one `<revision>/<revision>.xml` for every revision of the laws
        #[arg(long)]
        xml_dir: PathBuf,
        /// The date as YYYYMMDD; revisions enforced after it are left out
        #[arg(long)]
        as_of: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Origins {
    origins: Vec<history::Origin>,
}

fn all_revisions(xml_dir: &Path, as_of: &str) -> Result<BTreeMap<String, Vec<PathBuf>>, String> {
    let mut names: Vec<String> = Vec::new();
    let entries = fs::read_dir(xml_dir).map_err(|e| format!("{}: {e}", xml_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        names.push(entry.file_name().to_string_lossy().to_string());
    }
    let mut keyed: Vec<(String, String, String)> = names
        .into_iter()
        .filter_map(|name| {
            let mut parts = name.split('_');
            let (law, date) = (parts.next()?.to_string(), parts.next()?.to_string());
            parts.next()?;
            (date.as_str() <= as_of).then_some((law, date, name))
        })
        .collect();
    keyed.sort();
    let mut out: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for (law, _, name) in keyed {
        out.entry(law)
            .or_default()
            .push(xml_dir.join(&name).join(format!("{name}.xml")));
    }
    Ok(out)
}

fn valid_date(as_of: &str) -> Result<(), (u8, String)> {
    if as_of.len() != 8 || !as_of.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            EXIT_USAGE,
            format!("--as-of takes a date as YYYYMMDD ({as_of} is not one)"),
        ));
    }
    Ok(())
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
    /// The read-as clauses that name another law and were looked up there, and how many matched.
    external_checked: u64,
    external_matched: u64,
    /// Warnings by rule.
    rules: BTreeMap<String, u64>,
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

fn check_files(files: &[PathBuf], debug: bool, corpus: bool) -> Report {
    let mut report = Report {
        laws: 0,
        unreadable: Vec::new(),
        counts: Counts::default(),
        external_checked: 0,
        external_matched: 0,
        rules: BTreeMap::new(),
        warnings: Vec::new(),
        clauses: Vec::new(),
    };
    let mut index = external::Index::default();
    let mut pending = Vec::new();
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
                report.warnings.extend(relative_ref::check(&law));
                report.warnings.extend(structure::check(&law));
                if corpus {
                    pending.extend(external::collect(&law));
                    index.add(&law, file.clone());
                }
            }
            Err(e) => report.unreadable.push(e.to_string()),
        }
    }
    if corpus {
        let (counts, warnings) = external::check(&pending, &index, |id| {
            index.files.get(id).and_then(|f| Law::from_file(f).ok())
        });
        report.external_checked = counts.checked;
        report.external_matched = counts.matched;
        report.warnings.extend(warnings);
    }
    for w in &report.warnings {
        *report.rules.entry(w.rule.clone()).or_default() += 1;
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
    if let Command::History {
        command: HistoryCommand::ListOrigins { xml_dir, as_of },
    } = &cli.command
    {
        valid_date(as_of)?;
        let files = all_revisions(xml_dir, as_of).map_err(|e| (EXIT_INPUT, e))?;
        let origins = Origins {
            origins: history::origins(&files),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&origins).expect("the origins serialize")
        );
        return Ok(0);
    }
    let corpus = matches!(cli.command, Command::Corpus { .. });
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
            valid_date(&as_of)?;
            (
                revision_files(&xml_dir, &as_of).map_err(|e| (EXIT_INPUT, e))?,
                fail_on_warnings,
            )
        }
        Command::History { .. } => unreachable!("handled above"),
    };
    let report = check_files(&files, cli.debug, corpus);
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
