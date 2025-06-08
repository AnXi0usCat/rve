use std::path::PathBuf;

use clap::{Arg, ArgAction, Command};
use rve::venv::create_venv;

pub fn main() -> Result<(), String> {
    let args = Command::new("rve")
        .version("0.1.0")
        .author("AnxiousCat")
        .about("reate a Python virtual environment (rve = Rust venv)")
        .arg(
            Arg::new("dest")
                .help("Directory to create the virtual environment in")
                .required(true),
        )
        .arg(
            Arg::new("python")
                .short('p')
                .long("python")
                .help("Path to base python interpretor")
                .num_args(1)
                .value_name("PYTHON"),
        )
        .arg(
            Arg::new("copies")
                .short('c')
                .long("copies")
                .help("Copy the interpreter binary instead of symlinking")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("symlinks")
                .short('s')
                .long("symlinks")
                .help("Force symlink the interpreter binary (default on Unix)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("upgrade-deps")
                .short('u')
                .long("upgrade-deps")
                .help("Upgrade core dependencies (pip) to the latest version in PyPI")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("requirements")
                .short('r')
                .long("requirements")
                .help("install requirements from a requirements.txt file")
                .value_name("FILE")
                .num_args(1),
        )
        .get_matches();

    let dest = args.get_one::<String>("dest").map(PathBuf::from).unwrap();
    let python = args
        .get_one::<String>("python")
        .map(PathBuf::from)
        .unwrap_or_else(|| "python3".into());
    let use_copies = args.get_flag("copies");
    let use_symlinks = args.get_flag("symlinks");
    let upgrade_deps = args.get_flag("upgrade-deps");
    let requirements_path = args.get_one::<String>("requirements").map(PathBuf::from);

    if use_copies && use_symlinks {
        panic!("Cannot specify both --copies and --symlinks");
    }

    create_venv(
        &dest,
        &python,
        use_copies,
        use_symlinks,
        requirements_path,
        upgrade_deps,
    )?;
    Ok(())
}
