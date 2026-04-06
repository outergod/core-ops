use clap::Parser;
use core_ops::cli::common as cli_common;
use core_ops::cli::verification::{
    generate, run, VerificationCommandResult, VerifyCli, VerifyCommands,
};

fn main() {
    let cli = VerifyCli::parse();
    if let Err(err) = run_and_exit(cli) {
        let report = cli_common::report_error(err);
        eprintln!("{:?}", report);
        std::process::exit(1);
    }
}

fn run_and_exit(cli: VerifyCli) -> Result<(), core_ops::core::errors::CoreError> {
    let result = match cli.command {
        VerifyCommands::Run(args) => VerificationCommandResult::Run(Box::new(run(&args)?)),
        VerifyCommands::Generate(args) => generate(&args)?,
    };

    match result {
        VerificationCommandResult::Run(output) => {
            if output.emit_json {
                println!("{}", output.machine_report);
            } else {
                println!("{}", output.human_report);
            }
            if output.exit_code != 0 {
                std::process::exit(output.exit_code);
            }
        }
        VerificationCommandResult::Generate {
            human_report,
            exit_code,
        } => {
            print!("{human_report}");
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
    }
    Ok(())
}
