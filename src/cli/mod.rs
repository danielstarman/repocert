mod app;
mod authorize;
mod certify;
mod check;
mod fix;
mod install_hooks;
mod json;
mod status;
mod validate;

use std::process::ExitCode;

use clap::Parser;

use app::{Cli, Commands};

pub fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(error.exit_code() as u8);
        }
    };

    match cli.command {
        Commands::Authorize(args) => authorize::run(args),
        Commands::Certify(args) => certify::run(args),
        Commands::Check(args) => check::run(args),
        Commands::Fix(args) => fix::run(args),
        Commands::InstallHooks(args) => install_hooks::run(args),
        Commands::Status(args) => status::run(args),
        Commands::Validate(args) => validate::run(args),
    }
}
