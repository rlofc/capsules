mod handlers;
mod utils;

use anyhow::Result;
use crate::handlers::*;
use crate::utils::Config;

fn main() -> Result<()> {
    let cfg = Config::load();
    let matches = clap::Command::new("Capsules")
        .about("Secure-by-default containers for operating-system hygene")
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
            clap::Command::new("exec")
                .about("Executes a command in a running container")
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
            clap::Command::new("spin")
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
        list_capsules()?;
    }

    if let Some(matches) = matches.subcommand_matches("console") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        capsule_console_as_root(container_id, matches.get_one::<String>("command").cloned())?;
    }

    if let Some(matches) = matches.subcommand_matches("exec") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        let command = matches.get_one::<String>("command").unwrap();
        let args = match matches.get_many::<String>("args") {
            Some(args) => args.map(|v| v.to_owned()).collect(),
            None => Vec::new(),
        };
        execute_in_capsule(container_id, command, args)?;
    }

    if let Some(matches) = matches.subcommand_matches("init") {
        let blueprint = matches.get_one::<String>("blueprint").unwrap();
        init_container_volume(&cfg, blueprint)?;
    }

    if let Some(matches) = matches.subcommand_matches("spin") {
        let init = matches.get_flag("no-init");
        let container_id = matches.get_one::<String>("container_id").unwrap();
        let volumes = match matches.get_many::<String>("volume") {
            Some(volumes) => volumes.map(|v| v.to_owned()).collect(),
            None => Vec::new(),
        };
        spin_a_new_capsule(&cfg, container_id, volumes, init)?;
    }

    if let Some(matches) = matches.subcommand_matches("delete") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        delete_capsule(container_id)?;
    }

    if let Some(matches) = matches.subcommand_matches("stop") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        stop_capsule(container_id)?;
    }

    if let Some(matches) = matches.subcommand_matches("start") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        start_capsule(container_id)?;
    }

    Ok(())
}
