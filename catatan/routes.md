# `src/routes.rs`

File ini menghubungkan metode dan URL dengan fungsi handler Axum. Middleware dipasang pada kelompok route yang membutuhkan autentikasi.

Navigasi: [indeks](README.md) · [middleware](auth-middleware.md) · [handler autentikasi](handlers/auth.md) · [contoh `curl`](akses_curl.md).

## Pola route

```rust
Router::new()
    .route("/api/menus", get(menus::get_menus).post(menus::create_menu))
    .route("/api/menus/{id}", get(menus::get_menu).put(menus::update_menu))
```

`Router` menyimpan daftar URL. `.route(...)` menghubungkan path dengan metode HTTP dan handler. `get(...)`, `post(...)`, `put(...)`, `patch(...)`, dan `delete(...)` memilih metode; `{id}` adalah bagian URL dinamis yang dibaca oleh `ApiPath<i32>`.

## Endpoint dan middleware

| Kelompok | Endpoint | Akses |
|---|---|---|
| Autentikasi | Register, login, logout | Terbuka |
| Profil | `GET /api/auth/me` | Token valid |
| Role akun | `PUT /api/auth/users/{id}/role` | Admin |
| Menu | `GET/POST /api/menus`, operasi `/{id}` | Semua role membaca; admin/kasir mengubah |
| Pesanan | `GET/POST /api/orders`, operasi `/{id}` | Admin semua; pembeli membuat dan membaca miliknya |
| Struk | `GET /api/orders/{id}/receipt` | Admin atau pemilik pesanan |
| Pembayaran | `GET /api/payments` | Admin |

Route receipt memakai pemeriksaan akses pesanan dan pembayaran. Untuk endpoint yang memerlukan identitas pengguna, middleware memeriksa token dan menaruh `AuthenticatedUser` pada extensions request. Handler mengambilnya melalui extractor `Extension`.

## Extractor request

- `State<AppState>` memberi handler akses ke pool PostgreSQL dan layanan JWT.
- `ApiPath<i32>` membaca ID dari URL serta mengirim error JSON Indonesia bila nilainya tidak valid.
- `ApiQuery<HashMap<String, String>>` membaca parameter query seperti `?field=name`.
- `ApiJson<T>` membaca JSON ke model `T` dan menyeragamkan pesan kesalahan format request.

Pengecekan role dilakukan sebelum handler dijalankan. Router juga memiliki fallback JSON untuk URL yang tidak terdaftar (404) dan metode yang tidak didukung (405). Aturan akses rinci ada di [catatan middleware](auth-middleware.md). Validasi isi seperti harga positif dan stok tidak negatif tetap ada di handler.

Kembali ke [indeks catatan](README.md) atau lanjut ke [middleware autentikasi](auth-middleware.md).
