pub mod service {
    tonic::include_proto!("proxy");
}

#[actix_web::main]
async fn main() {
    println!("Hello, world!");
}
