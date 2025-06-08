use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
pub mod service {
    tonic::include_proto!("proxy");
}
use service::{PredictRequest, proxy_service_client::ProxyServiceClient};

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

async fn predict_handler(json: web::Json<HttpPredictRequest>) -> impl Responder {
        HttpResponse::Ok().json(HttpPredictResponse {
        status:  String::from("200"),
        message: String::from("hello"),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/predict", web::get().to(predict_handler)))
        .bind("0.0.0.0:8000")?
        .run()
        .await
}
