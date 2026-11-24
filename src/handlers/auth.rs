use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{
    auth::{jwt::AuthenticatedUser, password, role::Role},
    error::ApiError,
    extractors::{ApiJson, ApiPath},
    state::AppState,
};

#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    body: ApiJson<Credentials>,
) -> Result<Response, ApiError> {
    let username = body.username.trim();

    if username.is_empty() || body.password.len() < 8 {
        return Err(ApiError::bad_request(
            "Username wajib diisi dan password minimal 8 byte",
        ));
    }

    if body.password.len() > 128 {
        return Err(ApiError::bad_request("Password maksimal 128 byte"));
    }

    let password = body.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || password::hash_password(&password))
        .await
        .map_err(|error| ApiError::internal("Gagal memproses password", error))?
        .map_err(|error| ApiError::internal("Gagal mengamankan password", error))?;

    let result =
        sqlx::query(r#"INSERT INTO "user" (username, password, role) VALUES ($1, $2, $3)"#)
            .bind(username)
            .bind(password_hash)
            .bind(Role::Pembeli.as_str())
            .execute(&state.db)
            .await;

    match result {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({ "message": "Akun berhasil dibuat" })),
        )
            .into_response()),
        Err(sqlx::Error::Database(database_error)) if database_error.is_unique_violation() => {
            Err(ApiError::conflict("Username sudah digunakan"))
        }
        Err(error) => Err(ApiError::internal("Gagal membuat akun", error)),
    }
}

pub async fn login(
    State(state): State<AppState>,
    body: ApiJson<Credentials>,
) -> Result<Response, ApiError> {
    let user = sqlx::query_as::<_, (i32, String, String)>(
        r#"SELECT id, password, role FROM "user" WHERE username = $1"#,
    )
    .bind(body.username.trim())
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal memeriksa kredensial", error))?;

    let Some((user_id, stored_password, role_name)) = user else {
        return Err(ApiError::unauthorized("Username atau password salah"));
    };

    let role = Role::from_database(&role_name)
        .ok_or_else(|| ApiError::internal_message("Role akun tidak valid"))?;

    if body.password.len() > 128 {
        return Err(ApiError::unauthorized("Username atau password salah"));
    }

    let password = body.password.clone();
    let verification = tokio::task::spawn_blocking(move || {
        password::verify_and_upgrade(&password, &stored_password)
    })
    .await
    .map_err(|error| ApiError::internal("Gagal memproses password", error))?
    .map_err(|error| ApiError::internal("Gagal memverifikasi password", error))?;

    if !verification.valid {
        return Err(ApiError::unauthorized("Username atau password salah"));
    }

    if let Some(password_hash) = verification.upgraded_hash {
        sqlx::query(r#"UPDATE "user" SET password = $1 WHERE id = $2"#)
            .bind(password_hash)
            .bind(user_id)
            .execute(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal memperbarui keamanan akun", error))?;
    }

    let access_token = state
        .jwt
        .issue(user_id, role)
        .map_err(|error| ApiError::internal("Gagal membuat token login", error))?;

    Ok(Json(serde_json::json!({
        "message": "Login berhasil",
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": state.jwt.expiration_seconds(),
        "role": role
    }))
    .into_response())
}

pub async fn logout() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "Hapus access_token dari penyimpanan klien"
    }))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Response, ApiError> {
    let username = sqlx::query_scalar::<_, String>(r#"SELECT username FROM "user" WHERE id = $1"#)
        .bind(user.user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| ApiError::internal("Gagal mengambil profil akun", error))?;

    match username {
        Some(username) => Ok(Json(serde_json::json!({
            "id": user.user_id,
            "username": username,
            "role": user.role
        }))
        .into_response()),
        None => Err(ApiError::unauthorized("Akun tidak lagi tersedia")),
    }
}

#[derive(Deserialize)]
pub struct RoleInput {
    pub role: String,
}

pub async fn update_user_role(
    State(state): State<AppState>,
    ApiPath(user_id): ApiPath<i32>,
    body: ApiJson<RoleInput>,
) -> Result<Response, ApiError> {
    let role = Role::from_database(body.role.trim()).ok_or_else(|| {
        ApiError::bad_request("Role harus salah satu dari: admin, kasir, pembeli")
    })?;

    let updated = sqlx::query_as::<_, (i32, String, String)>(
        r#"UPDATE "user" SET role = $1 WHERE id = $2 RETURNING id, username, role"#,
    )
    .bind(role.as_str())
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal memperbarui role akun", error))?;

    match updated {
        Some((id, username, role)) => Ok(Json(serde_json::json!({
            "id": id,
            "username": username,
            "role": role,
            "message": "Role diperbarui; pengguna harus login kembali"
        }))
        .into_response()),
        None => Err(ApiError::not_found("Akun tidak ditemukan")),
    }
}
