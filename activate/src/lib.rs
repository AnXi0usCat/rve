use std::{
    error::Error,
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub port: u16,
    pub path: PathBuf,
    pub sub_route: Option<String>,
}

#[derive(Debug)]
pub struct ModelProcess {
    name: String,
    port: u16,
    process: Child,
}

impl ModelProcess {
    pub fn terminate(&mut self) {
        let _ = self.process.kill();
    }
}

pub fn load_config(config_path: PathBuf) -> Result<Vec<ModelConfig>, Box<dyn Error>> {
    let content = fs::read_to_string(config_path)?;
    let yamls: Vec<ModelConfig> = serde_yaml::from_str(&content)?;

    Ok(yamls)
}

pub fn start_model_process(model_config: &ModelConfig) -> Result<ModelProcess, Box<dyn Error>> {
    let activate_script = model_config.path.join("venv/bin/activate");
    let start_command = format!(
        "source {} && python3 grpc_server --port {}",
        activate_script.display(),
        model_config.port
    );

    let process = Command::new("sh")
        .arg("-c")
        .arg(&start_command)
        .current_dir(model_config.path.clone())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    Ok(ModelProcess {
        name: model_config.name.clone(),
        port: model_config.port.clone(),
        process,
    })
}
