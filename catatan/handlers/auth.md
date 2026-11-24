# `src/handlers/auth.rs`

Handler autentikasi menangani register, login, logout, profil akun, dan perubahan role.

Navigasi: [indeks catatan](../README.md) · [middleware](../auth-middleware.md) · [layanan password](../auth/password.md) · [layanan JWT](../auth/jwt.md).

## Register

`POST /api/auth/register` menerima `username` dan `password` melalui `ApiJson<Credentials>`. Username harus berisi teks, password minimal 8 byte dan maksimal 128 byte. Role akun baru selalu `pembeli`.

Password di-hash dengan Argon2id melalui `tokio::task::spawn_blocking`. Ini memindahkan pekerjaan komputasi dari thread async. Jika username sudah dipakai, server mengembalikan HTTP 409; kesalahan database lainnya dicatat di server dan klien mendapat error JSON umum.

## Login

`POST /api/auth/login` mencari akun, membandingkan password, lalu menerbitkan access token HS256 yang berlaku satu jam. Jika database berisi password plaintext lama, verifikasi berhasil akan menggantinya dengan hash Argon2id.

Response berhasil berisi `access_token`, `token_type`, `expires_in`, dan `role`. Klien mengirim token berikutnya melalui header `Authorization: Bearer <token>`.

Username atau password yang salah menghasilkan HTTP 401 dengan pesan yang sama agar server tidak mengungkap apakah username terdaftar. Kegagalan hash, database, atau penerbitan token dilaporkan sebagai error server.

## Logout, profil, dan role

- `POST /api/auth/logout` memberi tahu klien untuk menghapus token. JWT stateless tetap berlaku sampai kedaluwarsa.
- `GET /api/auth/me` membaca `AuthenticatedUser` dari extractor `Extension` yang dipasang middleware login, lalu mengambil username dari database.
- `PUT /api/auth/users/{id}/role` hanya untuk admin. Role yang diterima adalah `admin`, `kasir`, dan `pembeli`; pengguna perlu login kembali setelah role berubah.

## Cara membaca handler Axum

- `State<AppState>` mengambil dependency bersama seperti pool database.
- `ApiJson<T>` membaca request JSON dan menghasilkan pesan error konsisten bila formatnya salah.
- `ApiPath<i32>` mengambil ID dari path URL.
- `Result<Response, ApiError>` mengembalikan response normal atau error dengan status HTTP yang sesuai.

Semua error API memiliki bentuk `{"error":"pesan"}`. Implementasinya ada di `src/error.rs`.
