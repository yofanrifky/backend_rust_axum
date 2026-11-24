# `src/auth_middleware.rs`

Middleware memeriksa bearer token sebelum request mencapai handler yang dilindungi.

Navigasi: [indeks](README.md) · [routing](routes.md) · [handler autentikasi](handlers/auth.md).

## Urutan pemeriksaan

1. Middleware membaca header `Authorization: Bearer <token>`.
2. `JwtService` memvalidasi tanda tangan HS256 dan waktu kedaluwarsa.
3. Middleware membaca role terbaru pengguna dari database.
4. Role pada token dibandingkan dengan role di database. Jika berubah, pengguna perlu login kembali.
5. Aturan izin memeriksa apakah role boleh menjalankan metode dan route tersebut.
6. Identitas tervalidasi disimpan pada request extensions untuk dibaca handler.

Token hilang, rusak, kedaluwarsa, atau milik akun yang sudah tidak tersedia menghasilkan `401 Unauthorized`. Pengguna yang login tetapi tidak memiliki izin mendapat `403 Forbidden`. Kegagalan query role menghasilkan `500 Internal Server Error` dan rincian teknisnya hanya dicatat di server.

## Aturan akses

| Middleware | Aturan |
|---|---|
| `require_login` | Memerlukan token valid; tidak membatasi role tertentu. |
| `require_admin` | Hanya admin, untuk daftar pembayaran dan perubahan role. |
| `require_menu_access` | Semua role boleh membaca; admin dan kasir boleh menulis. |
| `require_order_access` | Admin boleh semua. Pembeli boleh membuat pesanan dan membaca daftar/detail pesanan. Kasir ditolak. |
| `require_payment_access` | Admin atau pembeli dapat masuk ke route struk; handler tetap memastikan kepemilikan pesanan. |

Pemeriksaan role tidak menggantikan pemeriksaan pemilik data. Query detail pesanan dan struk tetap membatasi pembeli pada `user_id` miliknya.

## Format error

Semua error API dikembalikan sebagai JSON dengan bentuk yang sama:

```json
{"error":"Bearer token diperlukan"}
```

Implementasi format respons ada di `src/error.rs`; helper `ApiError` memilih status HTTP dan menyimpan pesan yang aman untuk klien.

Kembali ke [indeks catatan](README.md) atau lanjut ke [handler autentikasi](handlers/auth.md).
