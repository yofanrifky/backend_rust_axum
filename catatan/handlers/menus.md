# `src/handlers/menus.rs`

Handler menu membaca katalog dan menjalankan operasi tambah, ubah, serta hapus. Aturan role dipasang di router: semua role dapat membaca, sedangkan admin dan kasir dapat mengubah menu.

Navigasi: [indeks catatan](../README.md) · [routing](../routes.md) · [model data](../models.md).

## Membaca menu

`GET /api/menus` menerima query opsional `field` melalui `ApiQuery`.

| Query | Isi response |
|---|---|
| Tanpa `field` | Array dengan ID, nama, harga Rupiah, kategori, dan stok. |
| `field=id` | Daftar ID. |
| `field=name` | Daftar nama. |
| `field=price` | Daftar harga yang diformat sebagai Rupiah. |
| `field=category` | Daftar kategori. |
| `field=stock` | Daftar stok. |

Field lain menghasilkan HTTP 400. `GET /api/menus/{id}` mengembalikan satu menu sebagai array berisi lima nilai, sesuai bentuk response endpoint yang sudah ada.

## Membuat dan mengubah menu

`POST /api/menus` dan `PUT /api/menus/{id}` menerima `name`, `price`, `category`, serta `stock`. Stok boleh tidak dikirim saat membuat menu; nilai default-nya nol. Handler menolak nama/kategori kosong, harga nol atau negatif, dan stok negatif.

`PATCH /api/menus/{id}` menerima satu atau beberapa field. Query memakai `COALESCE` agar field yang tidak dikirim tetap menggunakan nilai lama. Body kosong atau nilai yang melanggar validasi menghasilkan HTTP 400.

`DELETE /api/menus/{id}` mengembalikan pesan berhasil, atau HTTP 404 jika ID tidak ditemukan.

## Error dan response

Handler memakai `State<AppState>` untuk pool SQLx, `ApiPath<i32>` untuk ID, dan `ApiJson<T>` untuk body. Kegagalan database dicatat di terminal melalui `ApiError::internal`; response klien berisi pesan yang jelas tanpa detail query atau koneksi.

Harga menggunakan `Decimal`, bukan floating-point. `format_rupiah` mengubah nilai untuk endpoint daftar/detail, sementara response create/update tetap menyertakan objek menu dengan nilai harga desimal.
