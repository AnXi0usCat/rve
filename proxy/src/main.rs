use std::{collections::HashMap, env, error::Error, path::PathBuf};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, error::ErrorBadRequest, web};
use serde_json::Value;
use tonic::{Request, transport::Channel};
pub mod service {
    tonic::include_proto!("proxy");
}
use activate::{ModelConfig, ModelProcess, load_config, start_model_process};
use service::{PredictRequest, proxy_service_client::ProxyServiceClient};
use tokio::sync::Mutex;
struct AppState {
    clients: HashMap<String, Mutex<ProxyServiceClient<Channel>>>,
}

async fn predict_handler(
    req: HttpRequest,
    data: web::Data<AppState>,
    json: web::Json<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    let resource_name = req
        .match_name()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("missing route name"))?;

    let payload = serde_json::to_string(&json.into_inner()).map_err(ErrorBadRequest)?;
    let mut client = data.clients.get(resource_name).unwrap().lock().await;

    // gRPC request
    let grpc_response = client
        .predict(Request::new(PredictRequest {
            json_request: payload,
        }))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .into_inner();

    drop(client);

    // Deserialize arbitrary response JSON
    let response_value: Value = serde_json::from_str(&grpc_response.json_response)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let response = serde_json::json!({
        "response": response_value,
    });

    Ok(HttpResponse::Ok().json(response))
}

fn start_servers(configs: &Vec<ModelConfig>) -> Result<Vec<ModelProcess>, Box<dyn Error>> {
    let mut handles: Vec<ModelProcess> = Vec::new();

    for config in configs.iter() {
        log::info!("Creating a gRPC server for {}", &config.name);
        handles.push(start_model_process(config)?);
    }
    Ok(handles)
}

async fn create_clients(
    yamls: &Vec<ModelConfig>,
) -> Result<HashMap<String, Mutex<ProxyServiceClient<Channel>>>, Box<dyn Error>> {
    let mut clients: HashMap<String, Mutex<ProxyServiceClient<Channel>>> = HashMap::new();

    for yaml in yamls.iter() {
        let client = ProxyServiceClient::connect(format!("http://[::1]:{}", yaml.port))
            .await
            .expect("Could not connect to gRPC service");

        let mut client_name = yaml.name.clone();
        if let Some(sub_route) = yaml.sub_route.clone() {
               client_name = format!("{}-{}", yaml.name.clone(), sub_route);
        }
        log::info!("Creating a gGRPC client: {}", &client_name);
        clients.insert(client_name, Mutex::new(client));
    }
    Ok(clients)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let config_path =
        PathBuf::from(env::var("MODEL_YAML").expect("MODEL_YAML variable is not set."));

    let yamls = load_config(config_path)?;
    let _servers = start_servers(&yamls)?;
    let clients = create_clients(&yamls).await?;
    let state = web::Data::new(AppState { clients });

    HttpServer::new(move || {
        let mut app = App::new().app_data(state.clone());
        for yaml in yamls.iter() {
            let mut name = yaml.name.clone();
            let mut route = format!("{}/predict", name);

            if let Some(sub_route) = yaml.sub_route.clone() {
                route = format!("{}-{}", route, sub_route);
                name = format!("{}-{}", name, sub_route);
            }
            log::info!("Creating new route {}, with the name {}", &route, &name);
            
            app = app.service(
                web::resource(route)
                    .name(&name)
                    .route(web::post().to(predict_handler)),
            );
        }

        app
    }).workers(1)
    .bind("0.0.0.0:8000")?
    .run()
    .await?;

    Ok(())
}
