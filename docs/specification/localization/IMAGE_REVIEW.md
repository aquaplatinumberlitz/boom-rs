# Duyệt ảnh Việt hóa KR–VN

Danh mục chuỗi/ảnh dùng riêng cho luồng V1 (đăng nhập → sảnh → phòng → trận → kết quả) nằm ở
[UI_STRINGS_V1](UI_STRINGS_V1.md); trang dưới đây dùng để chốt trạng thái từng ảnh trước khi dùng.

## Mở để xác nhận

Mở bằng Chromium hoặc Firefox:

`/home/admin/boom-re-l10n/l10n/.cache/image-review/index.html`

Có thể mở trực tiếp file HTML, không cần server hay kết nối mạng. Mở terminal và chạy:

```sh
xdg-open /home/admin/boom-re-l10n/l10n/.cache/image-review/index.html
```

Dữ liệu và ảnh đều ở trong thư mục `image-review/`. Nếu chép sang máy khác, chép cả
thư mục, không chỉ riêng `index.html`. Ảnh giải mã là dữ liệu sinh cục bộ, nằm trong
`l10n/.cache/` đã được Git bỏ qua; không đưa tài nguyên game vào commit.

## Cách duyệt

1. Có thể chọn **Khác bản VN** để ưu tiên các tài nguyên có ảnh VN thay thế.
2. Lọc pack hoặc tìm tên entry. Mỗi thẻ đặt ảnh KR và VN cạnh nhau.
3. Bấm **Coi đủ ảnh / phóng lớn**. Mỗi bên có danh sách ảnh con riêng vì KR/VN có thể
   khác số ảnh hoặc cách sắp xếp. Một ảnh con cũng có thể chứa cả dải trạng thái nút.
4. Bỏ **Vừa khung** để coi kích thước gốc, chọn 2×/4× nếu chữ nhỏ; có thể mở PNG gốc.
5. Chọn trạng thái và ghi chú. Xác nhận theo toàn bộ tài nguyên, không chỉ ảnh con đầu.

| Trạng thái | Khi dùng |
|---|---|
| Chưa duyệt (`pending`) | Mặc định, chưa có xác nhận của người duyệt |
| Đã có tiếng Việt (`translated`) | Đã coi trực tiếp và xác nhận có bản Việt dùng được; ghi chú chọn bản VN nếu cần |
| Cần dịch (`needs_translation`) | Có chữ cần Việt hóa, chưa có bản Việt phù hợp |
| Không có chữ (`no_text`) | Đã kiểm tra ảnh con và không thấy chữ cần dịch |
| Cần kiểm tra thêm (`check`) | Không đọc rõ, chưa chắc ý nghĩa hoặc cần kiểm tra khi chạy game |

**Khác bản VN không đồng nghĩa đã dịch.** Tên file và khác biệt byte chỉ là gợi ý.
Không tự đánh dấu bất kỳ tài nguyên nào là đã Việt hóa.

## Giữ kết quả

Lựa chọn và ghi chú được lưu vào bộ nhớ của trình duyệt. Nếu trình duyệt không cho
lưu với file cục bộ, trang báo rõ và vẫn cho xuất CSV. Nên bấm **Xuất kết quả CSV**
sau mỗi lượt; file tải xuống tên `images_review.csv` chứa đủ tài nguyên và quyết định.

**Nhập kết quả CSV** khôi phục toàn bộ lựa chọn, kể cả khi đổi trình duyệt hoặc máy.
Trang kiểm tra cột, trạng thái, ID, mục trùng và số tài nguyên trước khi thay dữ liệu.
Bản lưu trong trình duyệt chưa tự ghi vào repo.

Để đưa CSV đã duyệt vào dự án:

```sh
cd /home/admin/boom-re-l10n
python3 tools/l10n_images_review.py --import-csv /duong/dan/images_review.csv
```

Lệnh kiểm tra toàn bộ trước khi ghi nguyên tử vào `l10n/images_review.csv`.
File này chỉ được tạo khi nhập kết quả, không dùng bảng `pending` để giả làm kết quả
đã duyệt. Có thể nhập lại CSV đã chỉnh để cập nhật quyết định.

## Thống kê đã kiểm tra

Cập nhật 2026-10-19:

| Loại | Số tài nguyên |
|---|---:|
| Tổng ứng viên ảnh | 3.038 |
| I16/sprite | 2.933 |
| JPEG | 105 |
| Khác bản VN cùng tên | 681 |
| Giống hệt bản VN cùng tên | 1.113 |
| Không có bản VN cùng tên | 1.244 |

Trang xuất đủ **8.750 ảnh con** tính cả hai phía KR/VN; các ảnh giống nhau dùng chung
file PNG. Đã kiểm tra toàn bộ 12.150 file PNG đầy đủ/thu nhỏ được tham chiếu, không lỗi.
Đây là ảnh con I16, không phải khẳng định đã dựng mọi frame chuyển động của engine;
bộ giải mã hiện không ghép các frame record 48 byte thành hoạt cảnh.

Danh sách cũ có 3.042 mục. Bốn mục sau là dữ liệu bản đồ bị nhận nhầm thành I16 do có
số 2 ở word thứ hai, đã loại khỏi `images_todo.csv`:

- `FxMap:Map/OkongOfLegend01`
- `FxMap:Map/OkongOfLegend02`
- `FxMap:Map/OkongOfLegend03`
- `FxMap:Map/WinningStreak_Colosseum01`

Bộ nhận diện mới kiểm tra đủ dung lượng cho bảng kích thước, frame record và độ dài
ảnh trước khi nhận I16. Việc này không xóa hay sửa bản đồ gốc.

## Tạo lại trang hoặc chạy server

```sh
cd /home/admin/boom-re-l10n
python3 tools/l10n_images_review.py
```

Mặc định đọc KR ở `/home/admin/boom-re/boomKR` và VN ở `/home/admin/Downloads/Boom`.
Có thể đổi bằng `--kr`, `--vn`, `--out`, `--workers`. Chương trình chỉ đọc pack,
giải mã ảnh và tạo trang; không dịch, vẽ lại hay patch dữ liệu game.

Nếu muốn dùng địa chỉ localhost, chạy ngoài môi trường đang chặn socket:

```sh
python3 tools/l10n_images_review.py --serve 8765
```

Sau đó mở `http://127.0.0.1:8765`. Đổi số cổng nếu đã có chương trình khác dùng.
Server chỉ phục vụ thư mục xuất; việc duyệt vẫn dùng lưu tại trình duyệt và xuất CSV.
Phiên sandbox tạo bộ ảnh này không mở được socket TCP, vì vậy không ghi nhận server
đang chạy. Trang mở trực tiếp từ file không có phụ thuộc đó.

## File liên quan và kiểm thử

- `l10n/images_todo.csv`: danh mục ứng viên, chưa có kết luận đã dịch.
- `tools/l10n_images_review.py`: xuất ảnh, tạo trang, nhập CSV và server tùy chọn.
- `tools/l10n_images_review.html`: giao diện nguồn, nhúng danh mục khi xuất.
- `l10n/.cache/image-review/manifest.json`: tên nguồn, đối chiếu, ảnh con và kích thước.
- `l10n/.cache/image-review/decode_errors.json`: danh sách lỗi xuất; lần kiểm cuối là `[]`.
- `l10n/.cache/image-review/images_review.csv`: mẫu CSV, toàn bộ chưa duyệt.
- `l10n/images_review.csv`: kết quả thực tế sau khi nhập CSV do người duyệt xuất.

Đã kiểm tra xuất đủ danh mục, đường dẫn ảnh tồn tại, PNG hợp lệ; chạy JavaScript để
kiểm tra hiển thị danh mục, lọc tìm kiếm/đối chiếu/trạng thái, lưu lựa chọn và vòng
xuất–đọc CSV có dấu phẩy, dấu nháy và xuống dòng. Đã coi trực tiếp một cặp nút KR/VN
để xác nhận ảnh tiếng Việt được giải mã. Chưa thực hiện kiểm thử tương tác trong
trình duyệt thật hoặc kiểm tra bố cục trong game. Phần text và 23 mục review text
được để riêng, không thay đổi bởi công cụ này.
