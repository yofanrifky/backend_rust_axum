# `src/main.rs`

File ini membaca konfigurasi, menyiapkan database dan JWT, lalu menjalankan server Axum.

Navigasi: [indeks](README.md) · [konfigurasi](env.md) · [state](state.md) · [routing](routes.md).

## Urutan startup

```text
.env → validasi konfigurasi → pool PostgreSQL → siapkan tabel
     → JwtService → AppState → router Axum → server
```

Server baru menerima request setelah konfigurasi dan persiapan database berhasil.

1. `dotenvy::dotenv().ok()` mencoba memuat `.env` ke environment proses.
2. `DATABASE_URL` dan `JWT_SECRET` wajib tersedia. Secret harus memiliki minimal 32 byte.
3. `HOST` dan `PORT` memakai nilai default `127.0.0.1` dan `8080` jika tidak diatur.
4. `PORT` diubah menjadi `u16`; nilai selain angka atau di luar rentang port menghasilkan error yang jelas.
5. `PgPoolOptions` membuat pool dengan maksimal lima koneksi dan waktu tunggu 10 detik.
6. `db::initialize_database` membuat tabel dan menambahkan kolom yang diperlukan.
7. `AppState` menyimpan pool dan layanan JWT, lalu `routes::configure` menyusun endpoint serta middleware.
8. `TcpListener` membuka alamat server dan `axum::serve` mulai melayani request.

Kesalahan koneksi atau persiapan database dicatat di terminal. Pesan teknis ditampilkan di log, sedangkan kesalahan API saat server berjalan memakai format JSON yang aman.

## Bagian penting Rust

- `#[tokio::main]` menyiapkan runtime asynchronous yang dibutuhkan Axum dan SQLx.
- `.await` menunggu operasi I/O seperti koneksi database tanpa memblokir thread saat menunggu.
- `?` meneruskan error startup ke pemanggil `main`.
- `pool.clone()` membuat handle pool tambahan yang menunjuk ke pool yang sama, bukan menyalin isi database.
- `spawn_blocking` dipakai pada proses hash password agar komputasi Argon2 tidak menahan thread async.

Nama variabel konfigurasi tetap sama: `DATABASE_URL`, `HOST`, `PORT`, dan `JWT_SECRET`.

Kembali ke [indeks catatan](README.md) atau lanjut ke [state aplikasi](state.md).
