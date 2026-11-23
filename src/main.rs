use clap::{Parser, Subcommand, ValueEnum};
use precommitguard::config::{Config, DEFAULT_CONFIG};
use precommitguard::{FileReport, Kind, Severity, discover, lint_str, output};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Json,
    Sarif,
    Github,
}

#[derive(Clone, Copy, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

/// Fast offline supply-chain and hygiene linter for .pre-commit-config.yaml
/// and .pre-commit-hooks.yaml.
///
/// Exit codes: 0 = no findings at or above --fail-on, 1 = findings,
/// 2 = usage, config or I/O error.
#[derive(Parser)]
#[command(version, about, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    check: CheckArgs,
}

#[derive(Subcommand)]
enum Command {
    /// Lint files (the default when no subcommand is given).
    Check(CheckArgs),
    /// List all rules.
    Rules,
    /// Show the documentation of one rule, e.g. `explain PCG001`.
    Explain {
        /// Rule ID.
        id: String,
    },
}

#[derive(clap::Args, Clone)]
struct CheckArgs {
    /// Files or directories to lint. Directories are searched recursively for
    /// .pre-commit-config.yaml and .pre-commit-hooks.yaml. Defaults to ".".
    paths: Vec<PathBuf>,
    /// Output format.
    #[arg(short, long, value_enum, default_value = "human")]
    format: Format,
    /// Config file (default: .precommitguard.yaml if present).
    #[arg(short, long, conflicts_with = "no_config")]
    config: Option<PathBuf>,
    /// Do not load .precommitguard.yaml from the working directory.
    #[arg(long)]
    no_config: bool,
    /// Minimum severity that causes exit code 1: info, warning, error, never.
    #[arg(long)]
    fail_on: Option<String>,
    /// Disable a rule (repeatable).
    #[arg(long = "ignore", value_name = "RULE")]
    ignore: Vec<String>,
    /// Skip files matching a glob (repeatable), e.g. 'examples/**'.
    #[arg(long = "ignore-path", value_name = "GLOB")]
    ignore_path: Vec<String>,
    /// Treat every given file as a hook manifest (.pre-commit-hooks.yaml).
    #[arg(long)]
    manifest: bool,
    /// Colorize human output.
    #[arg(long, value_enum, default_value = "auto")]
    color: Color,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Rules) => {
            print!("{}", output::rules_table());
            ExitCode::SUCCESS
        }
        Some(Command::Explain { id }) => match output::explain(&id) {
            Some(t) => {
                print!("{t}");
                ExitCode::SUCCESS
            }
            None => fatal(&format!(
                "unknown rule id '{id}' (see `precommitguard rules`)"
            )),
        },
        Some(Command::Check(args)) => run(args),
        None => run(cli.check),
    }
}

fn run(cli: CheckArgs) -> ExitCode {
    let mut cfg = match &cli.config {
        Some(p) => Config::load(p),
        None if cli.no_config => Ok(Config::default()),
        None if Path::new(DEFAULT_CONFIG).exists() => Config::load(Path::new(DEFAULT_CONFIG)),
        None => Ok(Config::default()),
    }
    .unwrap_or_else(|e| fatal(&format!("config error: {e}")));
    for id in &cli.ignore {
        if precommitguard::rules::rule(id).is_none() {
            fatal(&format!("unknown rule id '{id}'"));
        }
    }
    cfg.ignore.extend(cli.ignore.iter().cloned());
    cfg.add_ignore_paths(cli.ignore_path.clone())
        .unwrap_or_else(|e| fatal(&e));

    let fail_on = cli
        .fail_on
        .clone()
        .or(cfg.fail_on.clone())
        .unwrap_or_else(|| "warning".into());
    let threshold: Option<Severity> = match fail_on.parse() {
        Ok(s) => Some(s),
        Err(e) if e == "never" => None,
        Err(e) => fatal(&e),
    };

    let inputs = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths.clone()
    };
    for p in &inputs {
        if !p.exists() {
            fatal(&format!("{}: no such file or directory", p.display()));
        }
    }
    let files: Vec<PathBuf> = discover(&inputs)
        .unwrap_or_else(|e| fatal(&e.to_string()))
        .into_iter()
        .filter(|p| !cfg.path_ignored(&p.display().to_string()))
        .collect();
    if files.is_empty() {
        fatal("no .pre-commit-config.yaml or .pre-commit-hooks.yaml found; pass a path");
    }

    let mut reports = Vec::new();
    let mut io_errors = false;
    for f in &files {
        let display = f.display().to_string();
        let display = display.strip_prefix("./").unwrap_or(&display).to_string();
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("precommitguard: {display}: {e}");
                io_errors = true;
                continue;
            }
        };
        let kind = if cli.manifest {
            Kind::Manifest
        } else {
            Kind::for_path(f)
        };
        reports.push(FileReport {
            path: display,
            findings: lint_str(&src, kind, &cfg),
        });
    }

    let color = match cli.color {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    };
    let text = match cli.format {
        Format::Human => output::human(&reports, color),
        Format::Json => output::json(&reports),
        Format::Sarif => output::sarif(&reports),
        Format::Github => output::github(&reports),
    };
    print!("{text}");
    if matches!(cli.format, Format::Json | Format::Sarif) {
        println!();
    }

    if io_errors {
        return ExitCode::from(2);
    }
    let failed = threshold.is_some_and(|t| {
        reports
            .iter()
            .flat_map(|r| &r.findings)
            .any(|f| f.severity >= t)
    });
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn fatal(msg: &str) -> ! {
    eprintln!("precommitguard: {msg}");
    std::process::exit(2)
}
