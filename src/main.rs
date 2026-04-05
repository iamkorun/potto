mod checker;
mod discovery;
mod parser;
mod sync;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(
    name = "potto",
    version,
    about = "Audit .env files for sync problems",
    long_about = "potto compares your .env and .env.example files to catch keys that are \
                  out of sync — before they break your teammates or your CI pipeline."
)]
struct Cli {
    /// Suppress all output (exit code only)
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check .env against .env.example for missing keys (default)
    Check {
        /// Path to .env file (auto-discovered if omitted)
        #[arg(long)]
        env: Option<PathBuf>,

        /// Path to .env.example file (auto-discovered if omitted)
        #[arg(long)]
        example: Option<PathBuf>,
    },

    /// Add missing keys from .env to .env.example (values stripped)
    Sync {
        /// Path to .env file (auto-discovered if omitted)
        #[arg(long)]
        env: Option<PathBuf>,

        /// Path to .env.example file (auto-discovered if omitted)
        #[arg(long)]
        example: Option<PathBuf>,
    },

    /// Compare any two env files
    Compare {
        /// First env file
        file_a: PathBuf,

        /// Second env file
        file_b: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let quiet = cli.quiet;

    let exit_code = match cli.command {
        Some(Commands::Check { env, example }) => run_check(env, example, quiet),
        Some(Commands::Sync { env, example }) => run_sync(env, example, quiet),
        Some(Commands::Compare { file_a, file_b }) => run_compare(&file_a, &file_b, quiet),
        // Default: run check
        None => run_check(None, None, quiet),
    };

    process::exit(exit_code);
}

// ─── Check ───────────────────────────────────────────────────────────────────

fn run_check(env_arg: Option<PathBuf>, example_arg: Option<PathBuf>, quiet: bool) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let (env_path, example_path) = resolve_env_paths(env_arg, example_arg, &cwd);

    let env_path = match env_path {
        Some(p) => p,
        None => {
            eprintln!("{}", "Error: .env file not found.".red().bold());
            eprintln!("Run potto in a directory that contains a .env file, or use --env <path>");
            return 2;
        }
    };

    let example_path = match example_path {
        Some(p) => p,
        None => {
            eprintln!("{}", "Error: .env.example file not found.".red().bold());
            eprintln!("Run `potto sync` to create one, or use --example <path>");
            return 2;
        }
    };

    if !quiet {
        print_checking(&env_path, &example_path);
    }

    let env_map = match parser::parse_env_file(&env_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{} {}: {}", "Error reading".red().bold(), env_path.display(), e);
            return 2;
        }
    };

    let example_map = match parser::parse_env_file(&example_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{} {}: {}", "Error reading".red().bold(), example_path.display(), e);
            return 2;
        }
    };

    let result = checker::compare_maps(&env_map, &example_map);
    if !quiet {
        print_check_result(&result);
    }

    if result.is_in_sync() { 0 } else { 1 }
}

// ─── Sync ─────────────────────────────────────────────────────────────────────

fn run_sync(env_arg: Option<PathBuf>, example_arg: Option<PathBuf>, quiet: bool) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let (env_path, example_path) = resolve_env_paths(env_arg, example_arg, &cwd);

    let env_path = match env_path {
        Some(p) => p,
        None => {
            eprintln!("{}", "Error: .env file not found.".red().bold());
            return 2;
        }
    };

    let env_map = match parser::parse_env_file(&env_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{} {}: {}", "Error reading".red().bold(), env_path.display(), e);
            return 2;
        }
    };

    // If example doesn't exist yet, create a fresh one from env
    let example_path = example_path.unwrap_or_else(|| {
        env_path.parent().unwrap_or(Path::new(".")).join(".env.example")
    });

    let example_map = if example_path.exists() {
        match parser::parse_env_file(&example_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{} {}: {}", "Error reading".red().bold(), example_path.display(), e);
                return 2;
            }
        }
    } else {
        std::collections::HashMap::new()
    };

    let check_result = checker::compare_maps(&env_map, &example_map);

    if check_result.missing_from_example.is_empty() {
        if !quiet {
            println!("{}", "Already in sync — nothing to add.".green().bold());
        }
        return 0;
    }

    if !quiet {
        println!(
            "{} Adding {} key(s) to {}",
            "->".cyan().bold(),
            check_result.missing_from_example.len(),
            example_path.display()
        );
    }

    match sync::sync_example(
        &env_map,
        &example_map,
        &example_path,
        &check_result.missing_from_example,
    ) {
        Ok(added) => {
            if !quiet {
                for key in &added {
                    println!("  {} {}=", "+".green().bold(), key.green());
                }
                println!(
                    "\n{} {} key(s) added to {}",
                    "OK".green().bold(),
                    added.len(),
                    example_path.display()
                );
            }
            0
        }
        Err(e) => {
            eprintln!(
                "{} Failed to write {}: {}",
                "Error:".red().bold(),
                example_path.display(),
                e
            );
            2
        }
    }
}

// ─── Compare ─────────────────────────────────────────────────────────────────

fn run_compare(file_a: &Path, file_b: &Path, quiet: bool) -> i32 {
    let map_a = match parser::parse_env_file(file_a) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{} {}: {}", "Error reading".red().bold(), file_a.display(), e);
            return 2;
        }
    };

    let map_b = match parser::parse_env_file(file_b) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{} {}: {}", "Error reading".red().bold(), file_b.display(), e);
            return 2;
        }
    };

    if !quiet {
        println!(
            "Comparing {} vs {}",
            file_a.display().to_string().cyan(),
            file_b.display().to_string().cyan()
        );
        println!();
    }

    let result = checker::compare_maps(&map_a, &map_b);
    if !quiet {
        print_compare_result(&result, file_a, file_b);
    }

    if result.is_in_sync() { 0 } else { 1 }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn resolve_env_paths(
    env_arg: Option<PathBuf>,
    example_arg: Option<PathBuf>,
    cwd: &Path,
) -> (Option<PathBuf>, Option<PathBuf>) {
    match (env_arg, example_arg) {
        (Some(e), Some(ex)) => (Some(e), Some(ex)),
        (Some(e), None) => {
            let dir = e.parent().unwrap_or(cwd);
            let (_, example) = discovery::find_env_files(dir);
            (Some(e), example)
        }
        (None, Some(ex)) => {
            let dir = ex.parent().unwrap_or(cwd);
            let (env, _) = discovery::find_env_files(dir);
            (env, Some(ex))
        }
        (None, None) => {
            let (env, example) = discovery::find_env_files(cwd);
            (env, example)
        }
    }
}

fn print_checking(env_path: &Path, example_path: &Path) {
    println!(
        "-> Checking {} against {}",
        env_path.display().to_string().cyan(),
        example_path.display().to_string().cyan()
    );
    println!();
}

fn print_check_result(result: &checker::CheckResult) {
    if result.is_in_sync() {
        println!(
            "{} All {} key(s) are in sync.",
            "OK".green().bold(),
            result.in_sync_count
        );
        return;
    }

    if !result.missing_from_example.is_empty() {
        println!(
            "{} {} key(s) in .env are MISSING from .env.example  {}",
            "FAIL".red().bold(),
            result.missing_from_example.len(),
            "(teammates can't run the app)".dimmed()
        );
        for key in &result.missing_from_example {
            println!("  {} {}", "-".red().bold(), key.red());
        }
        println!();
    }

    if !result.missing_from_env.is_empty() {
        println!(
            "{} {} key(s) in .env.example are MISSING from .env  {}",
            "WARN".yellow().bold(),
            result.missing_from_env.len(),
            "(you may need to set these)".dimmed()
        );
        for key in &result.missing_from_env {
            println!("  {} {}", "-".yellow().bold(), key.yellow());
        }
        println!();
    }

    let hint = if !result.missing_from_example.is_empty() {
        format!("Run {} to fix .env.example automatically.", "potto sync".bold())
    } else {
        "Check your .env for any missing values.".to_string()
    };
    println!("{}", hint.dimmed());
}

fn print_compare_result(result: &checker::CheckResult, file_a: &Path, file_b: &Path) {
    if result.is_in_sync() {
        println!(
            "{} Both files have the same {} key(s).",
            "OK".green().bold(),
            result.in_sync_count
        );
        return;
    }

    if !result.missing_from_example.is_empty() {
        println!(
            "{} {} key(s) only in {}:",
            "+".green().bold(),
            result.missing_from_example.len(),
            file_a.display().to_string().cyan()
        );
        for key in &result.missing_from_example {
            println!("  {} {}", "+".green().bold(), key.green());
        }
        println!();
    }

    if !result.missing_from_env.is_empty() {
        println!(
            "{} {} key(s) only in {}:",
            "+".yellow().bold(),
            result.missing_from_env.len(),
            file_b.display().to_string().cyan()
        );
        for key in &result.missing_from_env {
            println!("  {} {}", "+".yellow().bold(), key.yellow());
        }
    }
}
