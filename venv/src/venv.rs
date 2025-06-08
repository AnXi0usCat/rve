use std::{
    fs::{self, File},
    io::Write,
    os::unix::{self, fs::PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub fn create_venv(
    dest: &Path,
    python: &Path,
    use_copies: bool,
    use_symlinks: bool,
    requirements: Option<PathBuf>,
    upgrade_deps: bool,
) -> Result<(), String> {
    let python_path = path_to_system_interpretor(python)?;
    let (major, minor, micro) = get_python_version(&python_path)?;

    if dest.exists() {
        let empty = dest.read_dir().unwrap().next().is_none();
        if !empty {
            return Err(format!(
                "Destination {:?} already exists and is not empty. Aborting.",
                dest
            ));
        }
    }

    create_directory_structure(dest, major, minor)?;
    copy_or_symlink_interpretor(dest, &python_path, use_symlinks, use_copies)?;
    write_pyvenv_cfg(dest, &python_path, major, minor, micro)?;
    write_activation_scripts(dest)?;
    bootstrap_pip(dest, upgrade_deps)?;

    if let Some(req) = requirements {
        install_requirements(dest, &req)?;
    }

    Ok(())
}

fn path_to_system_interpretor(python: &Path) -> Result<PathBuf, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {:?}", python))
        .output()
        .map_err(|_| format!("Failed to get path to the system interpretor {:?}", python))?;

    if !output.status.success() {
        return Err(format!(
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // deal with possible aliases
    let path = if stdout.starts_with("alias ") {
        stdout
            .splitn(2, '=')
            .nth(1)
            .map(|s| s.trim_matches('\'').trim_matches('"').trim().to_string())
            .ok_or_else(|| format!("Unexpected alias format: {}", stdout))?
    } else if stdout.contains(": aliased to ") {
        stdout
            .split(": aliased to")
            .nth(1)
            .map(|s| s.trim().to_string())
            .ok_or_else(|| format!("Unexpected aliased format: {}", stdout))?
    } else {
        stdout
    };

    Ok(PathBuf::from(path))
}

fn get_python_version(python_path: &Path) -> Result<(u8, u8, u8), String> {
    // TODO: reuse this and convert to struct maybe?
    // Run: python -c "import sys; print(sys.version_info[:2])"
    let command = Command::new(python_path)
        .arg("-c")
        .arg("import sys; v=sys.version_info; print(f\"{v.major}.{v.minor}.{v.micro}\")")
        .output()
        .map_err(|e| format!("Failed to execute {:?} error: {}", python_path, e))?;

    if !command.status.success() {
        let stderr = String::from_utf8_lossy(&command.stderr);
        return Err(format!(
            "Error querying Python version from {:?}: {}",
            python_path, stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&command.stdout).trim().to_string();
    let parts: Vec<&str> = stdout.split(".").collect();

    if parts.len() < 3 {
        return Err(format!("Unexpected version string: {}", stdout));
    }

    let major: u8 = parts[0]
        .parse()
        .map_err(|e| format!("Parsing major version from {}: {}", parts[0], e))?;
    let minor: u8 = parts[1]
        .parse()
        .map_err(|e| format!("Parsing minor version from {}: {}", parts[1], e))?;
    let micro: u8 = parts[2]
        .parse()
        .map_err(|e| format!("Parsing micro version from {}: {}", parts[2], e))?;

    Ok((major, minor, micro))
}

fn create_directory_structure(dest: &Path, major: u8, minor: u8) -> Result<(), String> {
    let bin_dir = dest.join("bin");
    let include = dest.join(format!("python{}.{}", major, minor));
    let site_packages = dest
        .join("lib")
        .join(format!("python{}.{}", major, minor))
        .join("site-packages");

    // TODO: remove created directories during error
    fs::create_dir_all(&bin_dir)
        .map_err(|_| format!("Failed to create bin/Scripts directory: {:?}", bin_dir))?;
    fs::create_dir_all(&include)
        .map_err(|_| format!("Failed to create include dir: {:?}", include))?;
    fs::create_dir_all(&site_packages).map_err(|_| {
        format!(
            "Failed to create site-packages directory: {:?}",
            site_packages
        )
    })?;

    Ok(())
}

fn copy_or_symlink_interpretor(
    dest: &Path,
    python_path: &Path,
    use_symlinks: bool,
    use_copies: bool,
) -> Result<(), String> {
    let symlink = {
        if use_symlinks {
            true
        } else if use_copies {
            false
        } else {
            true
        }
    };
    let target_python = dest.join("bin").join("python");
    if symlink {
        unix::fs::symlink(python_path, &target_python).map_err(|_| {
            format!(
                "Failed to symlink Python from {:?} to {:?}",
                python_path, target_python
            )
        })?;
    } else {
        fs::copy(python_path, &target_python).map_err(|_| {
            format!(
                "Failed to copy Python executable from {:?} to {:?}",
                python_path, target_python
            )
        })?;
    }
    Ok(())
}

fn write_pyvenv_cfg(
    dest: &Path,
    python_path: &Path,
    major: u8,
    minor: u8,
    micro: u8,
) -> Result<(), String> {
    let home = python_path
        .parent()
        .ok_or_else(|| format!("Failed to get parent of {:?}", python_path))?;
    let full_version = format!("{}.{}.{}", major, minor, micro);
    let cfg_contents = format!(
        "home = {}\ninclude-system-site-packages = false\nversion = {}\n",
        home.display(),
        full_version
    );

    let cfg_path = dest.join("pyvenv.cfg");
    std::fs::write(&cfg_path, cfg_contents)
        .map_err(|_| format!("Failed to write pyvenv.cfg to {:?}", cfg_path))?;

    Ok(())
}

fn write_activation_scripts(dest: &Path) -> Result<(), String> {
    let bash_template = r#"
# Save virtual environment path
__RVE_VIRTUAL_ENV="__VENV_PATH__"
export VIRTUAL_ENV="$__RVE_VIRTUAL_ENV"

# Save old PATH
__RVE_OLD_PATH="$PATH"
export PATH="$__RVE_VIRTUAL_ENV/bin:$PATH"

# Save old PS1 if it exists
if [ -z "${__RVE_OLD_PS1:-}" ] && [ -n "${PS1:-}" ]; then
    __RVE_OLD_PS1="$PS1"
fi

# Modify the prompt if not disabled
if [ -z "${VIRTUAL_ENV_DISABLE_PROMPT:-}" ]; then
    PS1="($(basename "$__RVE_VIRTUAL_ENV")) ${PS1:-}"
    export PS1
fi

# Unset PYTHONHOME if it's set (and save it)
if [ -n "${PYTHONHOME:-}" ]; then
    __RVE_OLD_PYTHONHOME="$PYTHONHOME"
    unset PYTHONHOME
fi

# Rehash shell PATH cache to pick up changes
hash -r 2>/dev/null

# Define deactivate function to restore environment
deactivate () {
    # Restore old PATH
    if [ -n "${__RVE_OLD_PATH:-}" ]; then
        export PATH="$__RVE_OLD_PATH"
        unset __RVE_OLD_PATH
    fi

    # Restore old PS1
    if [ -n "${__RVE_OLD_PS1:-}" ]; then
        export PS1="$__RVE_OLD_PS1"
        unset __RVE_OLD_PS1
    fi

    # Restore PYTHONHOME if previously set
    if [ -n "${__RVE_OLD_PYTHONHOME:-}" ]; then
        export PYTHONHOME="$__RVE_OLD_PYTHONHOME"
        unset __RVE_OLD_PYTHONHOME
    fi

    # Unset VIRTUAL_ENV
    unset VIRTUAL_ENV

    # Rehash shell PATH cache again after deactivation
    hash -r 2>/dev/null

    # Unset the deactivate function itself
    unset -f deactivate
}

# Export deactivate function
export -f deactivate > /dev/null
"#;

    let venv_path = dest
        .canonicalize()
        .unwrap_or_else(|_| dest.to_path_buf())
        .display()
        .to_string();

    let content = bash_template.replace("__VENV_PATH__", &venv_path);
    let activate_path = dest.join("bin").join("activate");
    let mut f = File::create(&activate_path)
        .map_err(|e| format!("Failed to create {:?}: {}", activate_path, e))?;
    f.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write content to file {:?}", e))?;
    // make it an executable
    let mut perms = fs::metadata(&activate_path)
        .map_err(|_| "Failed to get the current file permissions")?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&activate_path, perms)
        .map_err(|_| "Failed to set permissions on a file")?;
    Ok(())
}

fn bootstrap_pip(dest: &Path, upgrade_deps: bool) -> Result<(), String> {
    let python_exe = dest.join("bin").join("python");

    // python -m ensurepip --upgrade
    let mut status = Command::new(&python_exe)
        .arg("-m")
        .arg("ensurepip")
        .arg("--upgrade")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run {:?} -m ensurepip. error: {}", python_exe, e))?;

    if !status.success() {
        return Err(format!(
            "`ensurepip` failed with exit code {:?}",
            status.code()
        ));
    }

    // python -m pip install --upgrade pip setuptools wheel
    if upgrade_deps {
        status = Command::new(&python_exe)
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--upgrade")
            .arg("pip")
            .arg("setuptools")
            .arg("wheel")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|_| format!("Failed to upgrade pip/setuptools/wheel in venv"))?;

        if !status.success() {
            return Err(format!(
                "`pip install --upgrade pip setuptools wheel` failed with exit code {:?}",
                status.code()
            ));
        }
    }

    Ok(())
}

fn install_requirements(dest: &Path, requirements: &Path) -> Result<(), String> {
    if !requirements.exists() {
        return Err(format!("Requirements file {:?} not found", requirements));
    }
    let python = dest.join("bin").join("python");
    Command::new(&python)
        .arg("-m")
        .arg("pip")
        .arg("install")
        .arg("-r")
        .arg(requirements)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| {
            format!(
                "Failed to install requirements from {:?} using {:?}. error: {:?}",
                requirements, python, e
            )
        })?;

    Ok(())
}
