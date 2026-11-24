use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::{
    error::ApiError,
    extractors::{ApiJson, ApiPath, ApiQuery},
    handlers::format::format_rupiah,
    models::{MenuInput, MenuItem, MenuPatchInput},
    state::AppState,
};

pub async fn get_menus(
    State(state): State<AppState>,
    ApiQuery(query): ApiQuery<HashMap<String, String>>,
) -> Result<Response, ApiError> {
    let values = match query.get("field").map(String::as_str) {
        None => {
            let rows = sqlx::query_as::<_, (i32, String, Decimal, String, i32)>(
                "SELECT id, name, price, category, stock FROM menus ORDER BY id ASC",
            )
            .fetch_all(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal mengambil daftar menu", error))?;

            serde_json::json!(rows
                .into_iter()
                .map(|(id, name, price, category, stock)| {
                    serde_json::json!([id, name, format_rupiah(price), category, stock])
                })
                .collect::<Vec<_>>())
        }
        Some("id") => {
            let values = sqlx::query_scalar::<_, i32>("SELECT id FROM menus ORDER BY id ASC")
                .fetch_all(&state.db)
                .await
                .map_err(|error| ApiError::internal("Gagal mengambil daftar menu", error))?;
            serde_json::json!(values)
        }
        Some("name") => {
            let values = sqlx::query_scalar::<_, String>("SELECT name FROM menus ORDER BY id ASC")
                .fetch_all(&state.db)
                .await
                .map_err(|error| ApiError::internal("Gagal mengambil daftar menu", error))?;
            serde_json::json!(values)
        }
        Some("price") => {
            let values =
                sqlx::query_scalar::<_, Decimal>("SELECT price FROM menus ORDER BY id ASC")
                    .fetch_all(&state.db)
                    .await
                    .map_err(|error| ApiError::internal("Gagal mengambil daftar menu", error))?;
            serde_json::json!(values.into_iter().map(format_rupiah).collect::<Vec<_>>())
        }
        Some("category") => {
            let values =
                sqlx::query_scalar::<_, String>("SELECT category FROM menus ORDER BY id ASC")
                    .fetch_all(&state.db)
                    .await
                    .map_err(|error| ApiError::internal("Gagal mengambil daftar menu", error))?;
            serde_json::json!(values)
        }
        Some("stock") => {
            let values = sqlx::query_scalar::<_, i32>("SELECT stock FROM menus ORDER BY id ASC")
                .fetch_all(&state.db)
                .await
                .map_err(|error| ApiError::internal("Gagal mengambil daftar stok", error))?;
            serde_json::json!(values)
        }
        Some(_) => {
            return Err(ApiError::bad_request(
                "Field harus salah satu dari: id, name, price, category, stock",
            ));
        }
    };

    Ok(Json(values).into_response())
}

pub async fn get_menu(
    State(state): State<AppState>,
    ApiPath(menu_id): ApiPath<i32>,
) -> Result<Response, ApiError> {
    let menu = sqlx::query_as::<_, MenuItem>(
        "SELECT id, name, price, category, stock FROM menus WHERE id = $1",
    )
    .bind(menu_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal mengambil menu", error))?
    .ok_or_else(|| ApiError::not_found("Menu tidak ditemukan"))?;

    Ok(Json(serde_json::json!([
        menu.id,
        menu.name,
        format_rupiah(menu.price),
        menu.category,
        menu.stock
    ]))
    .into_response())
}

pub async fn create_menu(
    State(state): State<AppState>,
    body: ApiJson<MenuInput>,
) -> Result<Response, ApiError> {
    validate_menu(&body)?;

    let menu = sqlx::query_as::<_, MenuItem>(
        "INSERT INTO menus (name, price, category, stock) VALUES ($1, $2, $3, $4) RETURNING id, name, price, category, stock",
    )
    .bind(body.name.trim())
    .bind(body.price)
    .bind(body.category.trim())
    .bind(body.stock)
    .fetch_one(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal membuat menu", error))?;

    Ok((StatusCode::CREATED, Json(menu)).into_response())
}

pub async fn update_menu(
    State(state): State<AppState>,
    ApiPath(menu_id): ApiPath<i32>,
    body: ApiJson<MenuInput>,
) -> Result<Response, ApiError> {
    validate_menu(&body)?;

    let menu = sqlx::query_as::<_, MenuItem>(
        "UPDATE menus SET name = $1, price = $2, category = $3, stock = $4 WHERE id = $5 RETURNING id, name, price, category, stock",
    )
    .bind(body.name.trim())
    .bind(body.price)
    .bind(body.category.trim())
    .bind(body.stock)
    .bind(menu_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal memperbarui menu", error))?
    .ok_or_else(|| ApiError::not_found("Menu tidak ditemukan"))?;

    Ok(Json(menu).into_response())
}

pub async fn patch_menu(
    State(state): State<AppState>,
    ApiPath(menu_id): ApiPath<i32>,
    body: ApiJson<MenuPatchInput>,
) -> Result<Response, ApiError> {
    if body.name.is_none()
        && body.price.is_none()
        && body.category.is_none()
        && body.stock.is_none()
    {
        return Err(ApiError::bad_request("Minimal satu field menu harus diisi"));
    }
    if body
        .name
        .as_ref()
        .is_some_and(|name| name.trim().is_empty())
        || body
            .category
            .as_ref()
            .is_some_and(|category| category.trim().is_empty())
    {
        return Err(ApiError::bad_request(
            "Nama dan kategori menu tidak boleh kosong",
        ));
    }
    if body.price.is_some_and(|price| price <= Decimal::ZERO) {
        return Err(ApiError::bad_request(
            "Harga harus berupa angka lebih besar dari 0",
        ));
    }
    if body.stock.is_some_and(|stock| stock < 0) {
        return Err(ApiError::bad_request("Stok tidak boleh negatif"));
    }

    let menu = sqlx::query_as::<_, MenuItem>(
        "UPDATE menus SET name = COALESCE($1, name), price = COALESCE($2, price), category = COALESCE($3, category), stock = COALESCE($4, stock) WHERE id = $5 RETURNING id, name, price, category, stock",
    )
    .bind(body.name.as_deref().map(str::trim))
    .bind(body.price)
    .bind(body.category.as_deref().map(str::trim))
    .bind(body.stock)
    .bind(menu_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal memperbarui menu", error))?
    .ok_or_else(|| ApiError::not_found("Menu tidak ditemukan"))?;

    Ok(Json(menu).into_response())
}

pub async fn delete_menu(
    State(state): State<AppState>,
    ApiPath(menu_id): ApiPath<i32>,
) -> Result<Response, ApiError> {
    let result = sqlx::query("DELETE FROM menus WHERE id = $1")
        .bind(menu_id)
        .execute(&state.db)
        .await
        .map_err(|error| ApiError::internal("Gagal menghapus menu", error))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Menu tidak ditemukan"));
    }

    Ok(Json(serde_json::json!({ "message": "Menu berhasil dihapus" })).into_response())
}

fn validate_menu(menu: &MenuInput) -> Result<(), ApiError> {
    if menu.name.trim().is_empty() || menu.category.trim().is_empty() {
        return Err(ApiError::bad_request("Nama dan kategori menu wajib diisi"));
    }
    if menu.price <= Decimal::ZERO {
        return Err(ApiError::bad_request(
            "Harga harus berupa angka lebih besar dari 0",
        ));
    }
    if menu.stock < 0 {
        return Err(ApiError::bad_request("Stok tidak boleh negatif"));
    }
    Ok(())
}
