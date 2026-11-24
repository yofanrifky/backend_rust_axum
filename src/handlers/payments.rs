use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rust_decimal::Decimal;

use crate::{
    auth::{jwt::AuthenticatedUser, role::Role},
    error::ApiError,
    extractors::ApiPath,
    state::AppState,
};

pub async fn list_payments(State(state): State<AppState>) -> Result<Response, ApiError> {
    let payments = sqlx::query_as::<_, (i32, i32, Option<i32>, String, String, String)>(
        r#"SELECT p.id, p.order_id, p.user_id, p.method, p.status,
            to_char(p.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
            FROM payments p ORDER BY p.id DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal mengambil daftar pembayaran", error))?;

    let items = payments
        .into_iter()
        .map(|(id, order_id, user_id, method, status, created_at)| {
            serde_json::json!({
                "id": id,
                "order_id": order_id,
                "user_id": user_id,
                "method": method,
                "status": status,
                "created_at": created_at
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(items).into_response())
}

pub async fn get_receipt(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    ApiPath(order_id): ApiPath<i32>,
) -> Result<Response, ApiError> {
    let receipt = sqlx::query_as::<_, (i32, String, String, i32, Option<Decimal>, Option<String>)>(
        r#"SELECT o.id, o.customer_name, o.item_name, o.quantity,
            o.unit_price * o.quantity, to_char(p.paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
            FROM orders o
            JOIN payments p ON p.order_id = o.id
            WHERE o.id = $1 AND p.status = 'lunas' AND ($2 OR o.user_id = $3)"#,
    )
    .bind(order_id)
    .bind(user.role == Role::Admin)
    .bind(user.user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| ApiError::internal("Gagal mengambil struk", error))?
    .ok_or_else(|| {
        ApiError::not_found("Struk tidak ditemukan, belum dibayar, atau bukan milik akun ini")
    })?;

    let (order_id, customer_name, item_name, quantity, total, paid_at) = receipt;
    let total = total.ok_or_else(|| {
        ApiError::new(
            StatusCode::CONFLICT,
            "Struk tidak tersedia untuk pesanan lama tanpa snapshot harga",
        )
    })?;
    let paid_at = paid_at.ok_or_else(|| {
        ApiError::new(
            StatusCode::CONFLICT,
            "Waktu pembayaran tidak tersedia untuk struk ini",
        )
    })?;

    Ok(Json(serde_json::json!({
        "receipt_number": format!("RST-{order_id}"),
        "order_id": order_id,
        "customer_name": customer_name,
        "item_name": item_name,
        "quantity": quantity,
        "total": total,
        "currency": "IDR",
        "paid_at": paid_at
    }))
    .into_response())
}
