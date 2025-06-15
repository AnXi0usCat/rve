use std::{collections::HashMap, env, error::Error, path::PathBuf, sync::Mutex};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, error::ErrorBadRequest, web};
use serde_json::Value;
use tonic::{Request, transport::Channel};
pub mod service {
    tonic::include_proto!("proxy");
}
use activate::{load_config, start_model_process, ModelConfig, ModelProcess};
use service::{PredictRequest, proxy_service_client::ProxyServiceClient};

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
    let mut client = data.clients.get(resource_name).unwrap().lock().unwrap();

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
        if let Some(sub_route) = yaml.sub_route.clone() {
            clients.insert(
                format!("{}-{}", yaml.name.clone(), sub_route),
                Mutex::new(client),
            );
        } else {
            clients.insert(yaml.name.clone(), Mutex::new(client));
        }
    }
    Ok(clients)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
            app = app.service(
                web::resource(route)
                    .name(&name)
                    .route(web::get().to(predict_handler)),
            );
        }

        app
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await?;

    Ok(())
}
