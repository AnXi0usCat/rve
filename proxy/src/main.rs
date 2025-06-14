use std::sync::Mutex;

use actix_web::{App, HttpResponse, HttpServer, error::ErrorBadRequest, web};
pub mod service {
    tonic::include_proto!("proxy");
}
use serde_json::Value;
use service::{PredictRequest, proxy_service_client::ProxyServiceClient};
use tonic::{Request, transport::Channel};

struct AppState {
    client: Mutex<ProxyServiceClient<Channel>>,
}

async fn predict_handler(
    data: web::Data<AppState>,
    json: web::Json<Value>,
) -> Result<HttpResponse, actix_web::Error> {
    let payload = serde_json::to_string(&json.into_inner()).map_err(ErrorBadRequest)?;

    let mut client = data.client.lock().unwrap();
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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let client = ProxyServiceClient::connect("http://[::1]:50051")
        .await
        .expect("Could not connect to gRPC service");

    HttpServer::new(move || {
        App::new()
            .app_data(AppState {
                client: Mutex::new(client.clone()),
            })
            .route("/predict", web::get().to(predict_handler))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
