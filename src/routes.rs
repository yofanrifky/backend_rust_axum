use axum::{
    http::StatusCode,
    middleware,
    routing::{get, post, put},
    Router,
};

use crate::{
    auth_middleware::{
        require_admin, require_login, require_menu_access, require_order_access,
        require_payment_access,
    },
    error::ApiError,
    handlers::{auth, menus, orders, payments},
    state::AppState,
};

pub fn configure(state: AppState) -> Router {
    let menu_routes = Router::new()
        .route("/api/menus", get(menus::get_menus).post(menus::create_menu))
        .route(
            "/api/menus/{id}",
            get(menus::get_menu)
                .put(menus::update_menu)
                .patch(menus::patch_menu)
                .delete(menus::delete_menu),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_menu_access,
        ));

    let receipt_route = Router::new()
        .route("/api/orders/{id}/receipt", get(payments::get_receipt))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_payment_access,
        ));

    let order_routes = Router::new()
        .route(
            "/api/orders",
            get(orders::get_orders).post(orders::create_order),
        )
        .route(
            "/api/orders/{id}",
            get(orders::get_order)
                .put(orders::update_order)
                .patch(orders::patch_order)
                .delete(orders::delete_order),
        )
        .merge(receipt_route)
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_order_access,
        ));

    let payment_routes = Router::new()
        .route("/api/payments", get(payments::list_payments))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));

    let protected_profile = Router::new()
        .route("/api/auth/me", get(auth::me))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_login));

    let admin_role_route = Router::new()
        .route("/api/auth/users/{id}/role", put(auth::update_user_role))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));

    Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .merge(menu_routes)
        .merge(order_routes)
        .merge(payment_routes)
        .merge(protected_profile)
        .merge(admin_role_route)
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(route_not_found)
        .with_state(state)
}

async fn route_not_found() -> ApiError {
    ApiError::not_found("Endpoint tidak ditemukan")
}

async fn method_not_allowed() -> ApiError {
    ApiError::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "Metode HTTP tidak didukung untuk endpoint ini",
    )
}
