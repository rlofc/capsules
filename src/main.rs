//        ░░░░░░░░░░░░░░░░░
//    ░░░░░░░░░░░░░░░░░░░░░░░░░
//   ░░░░░░░░░▒▒░░░░░░░░░░░░░░░░
//   ░░░░░░░░░▒▒░░░░░░░░░░░░░░░░
//   ░░░░░░░░░▒▒▓░░░░░░░░░░░░░░░
//   ░░░░░░░░░▒▒▒░░░░░░░░░░░░░░░
//   ░░░░░░░░░░▒▓▒░░░░░░░░░░░░░░
//   ░░░░░░░░░░▒▒▒▒▒░░░░░░░░░░░░
//   ░░░░░░░░░░░░▒▒░░░▒░░░░░░░░░
//   ░░░░░░░░░░░░░▒▒░░░░░░░░░░░░
//   ░░░░░░░░░░░░░░▒▒░░▒░░░░░░░░
//   ░░░░░░░░░░░░░░░▒▒░░░░░░░░░░
//   ░░░░░░░░░░░░░░░▒▒░░░▒▒░▒░░░
//   ░░░░░░░░░░░░░░░▒▒▒░░░░░░░░░
//   ░░░░░░░░░░░░░░░▒▒▒░░░░░░░░░
//   ░░░░░░░░░░░░░░▒▒▒▒░▒▒▒░▒▒░░
//   ░░░░░░░░░░░░▒▒▒▒▒░░▒▒░░░░░░
//   ░░░░░░░░░▒▒▒▒▒▒░░░░░░░░░░░░
//   ░░░░░░░▒▒▒▒▒▒░░░▒▒▒░░░▒░░░░
//   ░░░░░▒▒▒▒▒▒▒▒░░▒▒▒░░░▒▒░░░░
//   ░░░░▒▒▒▒▒▒▒▓▓░░░░░░░░░░░░░░
//   ░░░░▒▒▒▒▒▒▒▒▒░░░▒▒▒░░░░░░░░
//   ░░░░▒▒▒▒▒▒▒▓▒░░░▒░░░░░░░░░░
//  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
// ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
//  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
//    ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
//            CAPSULES

mod capsules;
mod commands;
mod tools;
mod utils;
use crate::capsules::*;
use crate::utils::Config;
use anyhow::Result;

fn main() -> Result<()> {
    let cfg = Config::load();
    let matches = clap::Command::new("Capsules")
        .about("Secure-by-default containers for operating-system hygiene")
        .version(clap::crate_version!())
        .author("Ithai Levi")
        .subcommand(clap::Command::new("list").about("List all capsules"))
        .subcommand(
            clap::Command::new("console")
                .about("Start a console root session")
                .arg(clap::Arg::new("container_id").required(true))
                .arg(clap::Arg::new("command").required(false)),
        )
        .subcommand(
            clap::Command::new("run")
                .about("Executes a command in a running capsule")
                .arg(clap::Arg::new("container_id").required(true))
                .arg(clap::Arg::new("command").required(true))
                .arg(clap::Arg::new("args").required(false).num_args(1..)),
        )
        .subcommand(
            clap::Command::new("init")
                .about("Init container volume")
                .arg(clap::Arg::new("blueprint").required(true)),
        )
        .subcommand(
            clap::Command::new("create")
                .about("Spins up a new container")
                .arg(clap::Arg::new("container_id").required(true))
                .arg(
                    clap::Arg::new("volume")
                        .long("volume")
                        .short('v')
                        .num_args(1..)
                        .value_name("host_path:container_path")
                        .help("Bind mount a volume (can be used multiple times)"),
                )
                .arg(
                    clap::Arg::new("no-init")
                        .short('f')
                        .long("no-init")
                        .required(false)
                        .action(clap::ArgAction::SetFalse),
                )
                .arg(
                    clap::Arg::new("no-gpu")
                        .long("no-gpu")
                        .required(false)
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    clap::Arg::new("no-pulse")
                        .long("no-pulse")
                        .required(false)
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            clap::Command::new("start")
                .about("Starts a container")
                .arg(clap::Arg::new("container_id").required(true)),
        )
        .subcommand(
            clap::Command::new("stop")
                .about("Stops a container")
                .arg(clap::Arg::new("container_id").required(true)),
        )
        .subcommand(
            clap::Command::new("delete")
                .about("Deletes a container")
                .arg(clap::Arg::new("container_id").required(true)),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("list") {
        Capsules::new(cfg.clone()).list_capsules()?;
    }

    if let Some(matches) = matches.subcommand_matches("console") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        Capsules::new(cfg.clone())
            .capsule_console_as_root(container_id, matches.get_one::<String>("command").cloned())?;
    }

    if let Some(matches) = matches.subcommand_matches("run") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        let command = matches.get_one::<String>("command").unwrap();
        let args = match matches.get_many::<String>("args") {
            Some(args) => args.map(|v| v.to_owned()).collect(),
            None => Vec::new(),
        };
        Capsules::new(cfg.clone()).execute_in_capsule(container_id, command, args)?;
    }

    if let Some(matches) = matches.subcommand_matches("init") {
        let blueprint = matches.get_one::<String>("blueprint").unwrap();
        Capsules::new(cfg.clone()).init_container_volume(blueprint)?;
    }

    if let Some(matches) = matches.subcommand_matches("create") {
        let init = matches.get_flag("no-init");
        let options = CapsuleOptions {
            no_gpu: matches.get_flag("no-gpu"),
            no_pulse: matches.get_flag("no-pulse"),
        };
        let container_id = matches.get_one::<String>("container_id").unwrap();
        let volumes = match matches.get_many::<String>("volume") {
            Some(volumes) => volumes.map(|v| v.to_owned()).collect(),
            None => Vec::new(),
        };
        Capsules::new(cfg.clone()).create_a_new_capsule(container_id, volumes, init, &options)?;
    }

    if let Some(matches) = matches.subcommand_matches("delete") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        Capsules::new(cfg.clone()).delete_capsule(container_id)?;
    }

    if let Some(matches) = matches.subcommand_matches("stop") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        Capsules::new(cfg.clone()).stop_capsule(container_id)?;
    }

    if let Some(matches) = matches.subcommand_matches("start") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        Capsules::new(cfg.clone()).start_capsule(container_id)?;
    }

    Ok(())
}
