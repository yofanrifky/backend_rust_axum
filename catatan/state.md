# `src/state.rs`

`AppState` menyimpan dependency yang dipakai bersama oleh handler dan middleware.

Navigasi: [indeks](README.md) · [startup](main.md) · [routing](routes.md).

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt: JwtService,
}
```

- `db` adalah pool SQLx untuk query PostgreSQL.
- `jwt` membuat dan memvalidasi token login.
- `Clone` memungkinkan Axum membagikan handle state ke route tanpa menyalin isi database atau secret ke string baru.
- Extractor `State<AppState>` mengambil state ini di handler dan middleware.

Alurnya:

```text
main.rs membuat pool dan JwtService
             ↓
      AppState { db, jwt }
             ↓
      Router Axum dan middleware
             ↓
 handler memakai state.db / state.jwt
```

Pemisahan state membuat koneksi dan konfigurasi bersama tidak perlu dibuat ulang di setiap handler.

Kembali ke [indeks catatan](README.md) atau lanjut ke [model data](models.md).
