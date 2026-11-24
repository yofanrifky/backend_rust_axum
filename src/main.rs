mod auth;
mod auth_middleware;
mod db;
mod error;
mod extractors;
mod handlers;
mod models;
mod routes;
mod state;

use auth::jwt::JwtService;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use std::{env, io, time::Duration};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "DATABASE_URL belum disetel"))?;
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "PORT harus berupa angka 0–65535",
            )
        })?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .map_err(|error| {
            eprintln!("Koneksi PostgreSQL gagal: {error}");
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "Gagal terhubung ke PostgreSQL. Periksa DATABASE_URL dan status database.",
            )
        })?;

    db::initialize_database(&pool).await.map_err(|error| {
        eprintln!("Persiapan database gagal: {error}");
        io::Error::new(io::ErrorKind::Other, "Gagal menyiapkan struktur database")
    })?;

    let jwt_secret = env::var("JWT_SECRET")
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "JWT_SECRET belum disetel"))?;
    let jwt = JwtService::new(jwt_secret)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let app_state = AppState {
        db: pool.clone(),
        jwt: jwt.clone(),
    };

    let listener = TcpListener::bind((host.as_str(), port))
        .await
        .map_err(|error| {
            eprintln!("Gagal membuka listener HTTP: {error}");
            io::Error::new(
                error.kind(),
                format!("Gagal menjalankan server di http://{host}:{port}"),
            )
        })?;
    println!("API berjalan di http://{host}:{port}");
    let app: Router = routes::configure(app_state);
    axum::serve(listener, app).await?;
    Ok(())
}
