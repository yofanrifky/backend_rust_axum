use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, Method},
    middleware::Next,
    response::Response,
};

use crate::{
    auth::{jwt::AuthenticatedUser, role::Role},
    error::ApiError,
    state::AppState,
};

pub async fn require_login(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = bearer_token(&request)?;
    let user = authenticate(&state, &token).await?;
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

pub async fn require_admin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    authorize(&state, request, next, |role| role == Role::Admin).await
}

pub async fn require_menu_access(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let method = request.method().clone();
    authorize(&state, request, next, move |role| {
        menu_access_allowed(role, &method)
    })
    .await
}

pub async fn require_order_access(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    authorize(&state, request, next, move |role| {
        order_access_allowed(role, &method, &path)
    })
    .await
}

pub async fn require_payment_access(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    authorize(&state, request, next, move |role| {
        payment_access_allowed(role, &method, &path)
    })
    .await
}

async fn authorize<F>(
    state: &AppState,
    mut request: Request,
    next: Next,
    allowed: F,
) -> Result<Response, ApiError>
where
    F: FnOnce(Role) -> bool,
{
    let token = bearer_token(&request)?;
    let user = authenticate(state, &token).await?;
    if !allowed(user.role) {
        return Err(ApiError::forbidden(
            "Role akun tidak memiliki akses ke endpoint ini",
        ));
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

async fn authenticate(state: &AppState, token: &str) -> Result<AuthenticatedUser, ApiError> {
    let user = state
        .jwt
        .verify(token)
        .map_err(|_| ApiError::unauthorized("Token tidak valid atau sudah kedaluwarsa"))?;

    let stored_role = sqlx::query_scalar::<_, String>(r#"SELECT role FROM "user" WHERE id = $1"#)
        .bind(user.user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| ApiError::internal("Gagal memvalidasi role akun", error))?
        .ok_or_else(|| ApiError::unauthorized("Akun tidak lagi tersedia"))?;

    let current_role = Role::from_database(&stored_role)
        .ok_or_else(|| ApiError::internal_message("Role akun tidak valid di database"))?;

    if current_role != user.role {
        return Err(ApiError::unauthorized(
            "Role akun berubah; silakan login kembali",
        ));
    }

    Ok(user)
}

fn bearer_token(request: &Request) -> Result<String, ApiError> {
    request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApiError::unauthorized("Bearer token diperlukan"))
}

fn is_order_detail_path(path: &str) -> bool {
    path.strip_prefix("/api/orders/")
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
}

fn menu_access_allowed(role: Role, method: &Method) -> bool {
    match *method {
        Method::GET | Method::HEAD => true,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE => {
            role == Role::Admin || role == Role::Kasir
        }
        _ => false,
    }
}

fn order_access_allowed(role: Role, method: &Method, path: &str) -> bool {
    role == Role::Admin
        || (role == Role::Pembeli
            && ((method == Method::GET && path == "/api/orders")
                || method == Method::POST
                || (method == Method::GET
                    && (is_order_detail_path(path)
                        || path
                            .strip_suffix("/receipt")
                            .is_some_and(is_order_detail_path)))))
}

fn payment_access_allowed(role: Role, method: &Method, path: &str) -> bool {
    role == Role::Admin
        || (role == Role::Pembeli
            && ((method == Method::POST && path.ends_with("/payment"))
                || (method == Method::GET && path.ends_with("/receipt"))))
}

#[cfg(test)]
mod tests {
    use axum::http::Method;

    use crate::auth::role::Role;

    use super::{menu_access_allowed, order_access_allowed, payment_access_allowed};

    #[test]
    fn menu_access_matches_roles() {
        for role in [Role::Admin, Role::Kasir, Role::Pembeli] {
            assert!(menu_access_allowed(role, &Method::GET));
        }
        assert!(menu_access_allowed(Role::Kasir, &Method::PATCH));
        assert!(menu_access_allowed(Role::Admin, &Method::DELETE));
        assert!(!menu_access_allowed(Role::Pembeli, &Method::POST));
    }

    #[test]
    fn order_access_hides_purchase_list_from_cashier_but_allows_buyer_history() {
        assert!(order_access_allowed(
            Role::Admin,
            &Method::GET,
            "/api/orders"
        ));
        assert!(!order_access_allowed(
            Role::Kasir,
            &Method::GET,
            "/api/orders"
        ));
        assert!(order_access_allowed(
            Role::Pembeli,
            &Method::GET,
            "/api/orders"
        ));
        assert!(order_access_allowed(
            Role::Pembeli,
            &Method::POST,
            "/api/orders"
        ));
        assert!(order_access_allowed(
            Role::Pembeli,
            &Method::GET,
            "/api/orders/15"
        ));
        assert!(!order_access_allowed(
            Role::Pembeli,
            &Method::DELETE,
            "/api/orders/15"
        ));
    }

    #[test]
    fn only_admin_or_buyer_can_request_payment_and_receipt() {
        assert!(payment_access_allowed(
            Role::Pembeli,
            &Method::POST,
            "/api/orders/15/payment"
        ));
        assert!(payment_access_allowed(
            Role::Pembeli,
            &Method::GET,
            "/api/orders/15/receipt"
        ));
        assert!(!payment_access_allowed(
            Role::Kasir,
            &Method::GET,
            "/api/orders/15/receipt"
        ));
        assert!(payment_access_allowed(
            Role::Admin,
            &Method::POST,
            "/api/orders/15/payment"
        ));
    }
}
