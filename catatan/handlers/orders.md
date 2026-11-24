# `src/handlers/orders.rs`

Handler pesanan menyediakan daftar/detail pesanan, pembelian menu, perubahan data, dan penghapusan. Role diperiksa middleware, sedangkan query detail tetap memeriksa kepemilikan pembeli.

Navigasi: [indeks catatan](../README.md) · [routing](../routes.md) · [middleware](../auth-middleware.md) · [pembayaran](payments.md).

## Daftar dan detail pesanan

`GET /api/orders` menampilkan pesanan milik pembeli yang login. Admin menerima semua pesanan. Parameter `field` dapat memilih `id`, `customer_name`, `item_name`, `quantity`, atau `status`; field lain menghasilkan HTTP 400.

`GET /api/orders/{id}` membatasi pembeli pada pesanan dengan `user_id` miliknya. Admin dapat melihat seluruh pesanan. Jika tidak ditemukan atau bukan milik pembeli, response-nya HTTP 404.

## Pembelian dalam satu transaksi

`POST /api/orders` menerima `customer_name`, `menu_id`, `quantity`, dan `payment_method`. Metode pembayaran harus `cash` atau `bank_transfer`; ID menu dan jumlah harus positif.

Handler menjalankan langkah berikut dalam satu transaksi PostgreSQL:

1. Mengurangi stok hanya jika menu ada dan stok mencukupi.
2. Menyimpan pesanan beserta snapshot harga saat pembelian.
3. Mencatat pembayaran berstatus `lunas`.
4. Commit transaksi setelah seluruh langkah berhasil.

Jika menu tidak ada atau stok tidak cukup, pembelian dibatalkan dan server mengembalikan HTTP 409. Kegagalan menyimpan pesanan atau pembayaran juga membatalkan transaksi sehingga stok tidak berubah setengah jalan.

Response sukses berisi informasi pesanan dan objek `payment`, termasuk total dalam mata uang `IDR`. Total dihitung dari harga menu saat dibeli dikali jumlah.

## Mengubah dan menghapus

Hanya admin yang dapat memakai `PUT`, `PATCH`, dan `DELETE` pada `/api/orders/{id}`. `PUT` memerlukan semua field. `PATCH` menerima satu atau beberapa field dan menolak body kosong, teks wajib yang kosong, jumlah tidak positif, serta status `paid`.

## Penanganan error

Handler menggunakan `ApiJson`, `ApiPath`, `State`, dan `Extension<AuthenticatedUser>`. Error validasi diberi status 400; ID yang tidak ditemukan diberi 404; stok yang tidak cukup diberi 409. Error SQL dicatat di server dan dikirim ke klien sebagai JSON aman dengan status 500.
