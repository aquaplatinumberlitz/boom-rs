# F-10b — trợ góc VN: Down có biên, L/R/U chưa đo (metadata #369)

Baseline bàn giao `e6e3d239532e42c3ea72b59007a6f6dfeb0aff98`; main fetch `86cee6f`.
Build/fixture VN `bc875ba5…`; patch offline; không sửa binary/pack/IDA DB.

## Bảng hướng
| Hướng | Đo runtime | Geometry / kết quả | Native quantity | Nguồn raw |
|---|---|---|---|---|
| Down | R-177, actor449280160 | offset30 → x530→560/y139; offset35 → x525/y39 blocked; aligned x560 Down y164 | native XY, step5/6; max∈(30,35] | R-177 raw/`movement146` (xem R-177) |
| Left/Right/Up | **chưa đo trong F-10b** | static chỉ có 4 nhánh rect; không có raw đối chứng | — | — |

## Static 59DA6F
- Collision path `59DABE`; cell/rect `59DB22–59DB8E`; correction `59DF31/59DF52`; tile passable
  `59DD49→7293B4`, `59DD5E→5C1027→5C0E98`.
- Không có hằng ngưỡng 34/12; các so sánh 20/24/54 theo cell/rect.
- Static có nhánh cho 4 hướng nhưng chưa có runtime L/R/U; không suy ngưỡng px cố định.

## Kết luận
Chỉ Down có biên `(30,35]`; L/R/U **unknown**. Đây là điều kiện cell/rect + bước 5/6, không phải
một hằng `34`/`12`; V1 12 px vẫn lệch ít nhất với Down. Không đổi code/profile.
