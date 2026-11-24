use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::{
    auth::{jwt::AuthenticatedUser, role::Role},
    error::ApiError,
    extractors::{ApiJson, ApiPath, ApiQuery},
    models::{CreateOrderInput, Order, OrderInput, OrderPatchInput},
    state::AppState,
};

pub async fn get_orders(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    ApiQuery(query): ApiQuery<HashMap<String, String>>,
) -> Result<Response, ApiError> {
    let field = query.get("field").map(String::as_str);
    let is_admin = user.role == Role::Admin;

    let values = match field {
        None => {
            let orders = sqlx::query_as::<_, Order>(
                "SELECT id, customer_name, item_name, quantity, status FROM orders WHERE $1 OR user_id = $2 ORDER BY id DESC",
            )
            .bind(is_admin)
            .bind(user.user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal mengambil daftar pesanan", error))?;

            serde_json::json!(orders
                .into_iter()
                .map(|order| [
                    serde_json::json!(order.id),
                    serde_json::json!(order.customer_name),
                    serde_json::json!(order.item_name),
                    serde_json::json!(order.quantity),
                    serde_json::json!(order.status),
                ])
                .collect::<Vec<_>>())
        }
        Some("id") => {
            let values = sqlx::query_scalar::<_, i32>(
                "SELECT id FROM orders WHERE $1 OR user_id = $2 ORDER BY id DESC",
            )
            .bind(is_admin)
            .bind(user.user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal mengambil daftar pesanan", error))?;
            serde_json::json!(values)
        }
        Some("customer_name") => {
            let values = sqlx::query_scalar::<_, String>(
                "SELECT customer_name FROM orders WHERE $1 OR user_id = $2 ORDER BY id DESC",
            )
            .bind(is_admin)
            .bind(user.user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal mengambil daftar pesanan", error))?;
            serde_json::json!(values)
        }
        Some("item_name") => {
            let values = sqlx::query_scalar::<_, String>(
                "SELECT item_name FROM orders WHERE $1 OR user_id = $2 ORDER BY id DESC",
            )
            .bind(is_admin)
            .bind(user.user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal mengambil daftar pesanan", error))?;
            serde_json::json!(values)
        }
        Some("quantity") => {
            let values = sqlx::query_scalar::<_, i32>(
                "SELECT quantity FROM orders WHERE $1 OR user_id = $2 ORDER BY id DESC",
            )
            .bind(is_admin)
            .bind(user.user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal mengambil daftar pesanan", error))?;
            serde_json::json!(values)
        }
        Some("status") => {
            let values = sqlx::query_scalar::<_, String>(
                "SELECT status FROM orders WHERE $1 OR user_id = $2 ORDER BY id DESC",
            )
            .bind(is_admin)
            .bind(user.user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|error| ApiError::internal("Gagal mengambil daftar pesanan", error))?;
            serde_json::json!(values)
        }
        Some(_) => {
            return Err(ApiError::bad_request(
                "Field harus salah satu dari: id, customer_name, item_name, quantity, status",
            ));
        }
    };

    Ok(Json(values).into_response())
}

pub async fn get_order(
    State(state): State<AppState>,
    ApiPath(order_id): ApiPath<i32>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Response, ApiError> {
    let order = if user.role == Role::Pembeli {
        sqlx::query_as::<_, Order>(
            "SELECT id, customer_name, item_name, quantity, status FROM orders WHERE id = $1 AND user_id = $2",
        )
        .bind(order_id)
        .bind(user.user_id)
        .fetch_optional(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Order>(
            "SELECT id, customer_name, item_name, quantity, status FROM orders WHERE id = $1",
        )
        .bind(order_id)
        .fetch_optional(&state.db)
        .await
    }
    .map_err(|error| ApiError::internal("Gagal mengambil pesanan", error))?
    .ok_or_else(|| ApiError::not_found("Pesanan tidak ditemukan"))?;

    Ok(Json(serde_json::json!([
        order.id,
        order.customer_name,
        order.item_name,
        order.quantity,
        order.status
    ]))
    .into_response())
}

pub async fn create_order(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    body: ApiJson<CreateOrderInput>,
) -> Result<Response, ApiError> {
    validate_new_order(&body)?;

    let mut transaction = state
        .db
        .begin()
        .await
        .map_err(|error| ApiError::internal("Gagal memulai pembelian", error))?;

    let menu = sqlx::query_as::<_, (String, Decimal)>(
        "UPDATE menus SET stock = stock - $1 WHERE id = $2 AND stock >= $1 RETURNING name, price",
    )
    .bind(body.quantity)
    .bind(body.menu_id)
    .fetch_optional(&mut *transaction)
    .await;

    let (item_name, unit_price) = match menu {
        Ok(Some(menu)) => menu,
        Ok(None) => {
            if let Err(error) = transaction.rollback().await {
                return Err(ApiError::internal(
                    "Gagal membatalkan transaksi pembelian",
                    error,
                ));
            }
            return Err(ApiError::conflict(
                "Menu tidak ditemukan atau stok tidak mencukupi",
            ));
        }
        Err(error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                eprintln!("[ERROR] Gagal membatalkan transaksi pembelian: {rollback_error}");
            }
            return Err(ApiError::internal(
                "Gagal memeriksa dan mengurangi stok",
                error,
            ));
        }
    };

    let order = sqlx::query_as::<_, Order>(
        "INSERT INTO orders (customer_name, item_name, quantity, status, user_id, menu_id, unit_price) VALUES ($1, $2, $3, 'lunas', $4, $5, $6) RETURNING id, customer_name, item_name, quantity, status",
    )
    .bind(body.customer_name.trim())
    .bind(item_name)
    .bind(body.quantity)
    .bind(user.user_id)
    .bind(body.menu_id)
    .bind(unit_price)
    .fetch_one(&mut *transaction)
    .await;

    let order = match order {
        Ok(order) => order,
        Err(error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                eprintln!("[ERROR] Gagal membatalkan transaksi pembelian: {rollback_error}");
            }
            return Err(ApiError::internal("Gagal membuat pesanan", error));
        }
    };

    let payment_id = sqlx::query_scalar::<_, i32>(
        "INSERT INTO payments (order_id, user_id, method, status, paid_at) VALUES ($1, $2, $3, 'lunas', NOW()) RETURNING id",
    )
    .bind(order.id)
    .bind(user.user_id)
    .bind(body.payment_method.trim())
    .fetch_one(&mut *transaction)
    .await;

    let payment_id = match payment_id {
        Ok(payment_id) => payment_id,
        Err(error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                eprintln!("[ERROR] Gagal membatalkan transaksi pembelian: {rollback_error}");
            }
            return Err(ApiError::internal("Gagal mencatat pembayaran", error));
        }
    };

    transaction
        .commit()
        .await
        .map_err(|error| ApiError::internal("Gagal menyelesaikan pembelian", error))?;

    let total = unit_price * Decimal::from(body.quantity);
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": order.id,
            "customer_name": order.customer_name,
            "item_name": order.item_name,
            "quantity": order.quantity,
            "status": order.status,
            "payment": {
                "id": payment_id,
                "method": body.payment_method.trim(),
                "status": "lunas",
                "total": total,
                "currency": "IDR"
            }
        })),
    )
        .into_response())
}

pub async fn update_order(
    State(state): State<AppState>,
    ApiPath(order_id): ApiPath<i32>,
    body: ApiJson<OrderInput>,
) -> Result<Response, ApiError> {
    validate_order(&body)?;

    let order = sqlx::query_as::<_, Order>(
        "UPDATE orders SET customer_name = $1, item_name = $2, quantity = $3, status = $4 WHERE id = $5 RETURNING id, customer_name, item_name, quantity, status",
    )
    .bind(body.customer_name.trim())
    .bind(body.item_name.trim())
    .bind(body.quantity)
    .bind(body.status.trim())
    .bind(order_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal memperbarui pesanan", error))?
    .ok_or_else(|| ApiError::not_found("Pesanan tidak ditemukan"))?;

    Ok(Json(order).into_response())
}

pub async fn patch_order(
    State(state): State<AppState>,
    ApiPath(order_id): ApiPath<i32>,
    body: ApiJson<OrderPatchInput>,
) -> Result<Response, ApiError> {
    if body.customer_name.is_none()
        && body.item_name.is_none()
        && body.quantity.is_none()
        && body.status.is_none()
    {
        return Err(ApiError::bad_request(
            "Minimal satu field pesanan harus diisi",
        ));
    }
    if body
        .customer_name
        .as_ref()
        .is_some_and(|name| name.trim().is_empty())
        || body
            .item_name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        || body
            .status
            .as_ref()
            .is_some_and(|status| status.trim().is_empty())
    {
        return Err(ApiError::bad_request(
            "Nama pelanggan, nama menu, dan status tidak boleh kosong",
        ));
    }
    if body.quantity.is_some_and(|quantity| quantity <= 0) {
        return Err(ApiError::bad_request(
            "Jumlah pesanan harus lebih besar dari 0",
        ));
    }
    if body.status.as_deref() == Some("paid") {
        return Err(ApiError::bad_request(
            "Status paid hanya dapat ditetapkan melalui konfirmasi pembayaran",
        ));
    }

    let order = sqlx::query_as::<_, Order>(
        "UPDATE orders SET customer_name = COALESCE($1, customer_name), item_name = COALESCE($2, item_name), quantity = COALESCE($3, quantity), status = COALESCE($4, status) WHERE id = $5 RETURNING id, customer_name, item_name, quantity, status",
    )
    .bind(body.customer_name.as_deref().map(str::trim))
    .bind(body.item_name.as_deref().map(str::trim))
    .bind(body.quantity)
    .bind(body.status.as_deref().map(str::trim))
    .bind(order_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal memperbarui pesanan", error))?
    .ok_or_else(|| ApiError::not_found("Pesanan tidak ditemukan"))?;

    Ok(Json(order).into_response())
}

pub async fn delete_order(
    State(state): State<AppState>,
    ApiPath(order_id): ApiPath<i32>,
) -> Result<Response, ApiError> {
    let result = sqlx::query("DELETE FROM orders WHERE id = $1")
        .bind(order_id)
        .execute(&state.db)
        .await
        .map_err(|error| ApiError::internal("Gagal menghapus pesanan", error))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Pesanan tidak ditemukan"));
    }

    Ok(Json(serde_json::json!({ "message": "Pesanan berhasil dihapus" })).into_response())
}

fn validate_order(order: &OrderInput) -> Result<(), ApiError> {
    if order.customer_name.trim().is_empty()
        || order.item_name.trim().is_empty()
        || order.status.trim().is_empty()
    {
        return Err(ApiError::bad_request(
            "Nama pelanggan, nama menu, dan status wajib diisi",
        ));
    }
    if order.quantity <= 0 {
        return Err(ApiError::bad_request(
            "Jumlah pesanan harus lebih besar dari 0",
        ));
    }
    if order.status == "paid" {
        return Err(ApiError::bad_request(
            "Status paid hanya dapat ditetapkan melalui konfirmasi pembayaran",
        ));
    }
    Ok(())
}

fn validate_new_order(order: &CreateOrderInput) -> Result<(), ApiError> {
    if order.customer_name.trim().is_empty() {
        return Err(ApiError::bad_request("Nama pelanggan wajib diisi"));
    }
    if order.menu_id <= 0 || order.quantity <= 0 {
        return Err(ApiError::bad_request(
            "ID menu dan jumlah pesanan harus lebih besar dari 0",
        ));
    }
    if !matches!(order.payment_method.trim(), "cash" | "bank_transfer") {
        return Err(ApiError::bad_request(
            "Metode pembayaran harus cash atau bank_transfer",
        ));
    }
    Ok(())
}
