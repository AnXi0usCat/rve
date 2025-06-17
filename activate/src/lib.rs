use std::{
    error::Error,
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio}, thread, time::Duration,
};
use log;
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

impl Drop for ModelProcess {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

pub fn load_config(config_path: PathBuf) -> Result<Vec<ModelConfig>, Box<dyn Error>> {
    let content = fs::read_to_string(config_path)?;
    let yamls: Vec<ModelConfig> = serde_yaml::from_str(&content)?;

    Ok(yamls)
}

pub fn start_model_process(model_config: &ModelConfig) -> Result<ModelProcess, Box<dyn Error>> {
    let python_path = model_config.path.join("venv/bin/python");
    if !python_path.exists() {
        return Err(format!("Python not found at {:?}", python_path).into());
    }

    let script_path = model_config.path.join("grpc_server.py");
    if !script_path.exists() {
        return Err(format!("Server entry point not found at {:?}", script_path).into());
    }

    log::info!("Spawning: {:?} {:?}", python_path, script_path);

    let process = Command::new(python_path)
        .arg(script_path)
        .arg("--port")
        .arg(model_config.port.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    Ok(ModelProcess {
        name: model_config.name.clone(),
        port: model_config.port.clone(),
        process,
    })
}
