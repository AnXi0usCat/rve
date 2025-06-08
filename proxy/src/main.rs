use actix_web::{App, HttpResponse, HttpServer, web};
use serde::{Deserialize, Serialize};
pub mod service {
    tonic::include_proto!("proxy");
}
use service::{PredictRequest, proxy_service_client::ProxyServiceClient};
use tonic::Request;

#[derive(Deserialize)]
struct HttpPredictRequest {
    user_id: String,
    action: String,
}

#[derive(Serialize)]
struct HttpPredictResponse {
    status: String,
    message: String,
}

async fn predict_handler(
    json: web::Json<HttpPredictRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut client = ProxyServiceClient::connect("http://[::1]:50051")
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let request = Request::new(PredictRequest {
        user_id: json.user_id.clone(),
        action: json.action.clone(),
    });

    let response = client
        .predict(request)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .into_inner();

    Ok(HttpResponse::Ok().json(HttpPredictResponse {
        status: response.status,
        message: response.message,
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/predict", web::get().to(predict_handler)))
        .bind("0.0.0.0:8000")?
        .run()
        .await
}
