# Font client V1

Font dùng cho chữ Việt của client (V1-035). **Không phải asset game**: đây là font
mở (SIL OFL 1.1) nên được commit cùng license.

| File | SHA-256 | Nguồn |
|---|---|---|
| `BeVietnamPro-Regular.ttf` | `cd1ef6e9d7db28ad5cdb88a65ccbe693870e60d340b791f349d248342b4fe4c3` | google/fonts `ofl/bevietnampro/BeVietnamPro-Regular.ttf` @ `1cfe4cb468a703a247ca2d02410f80841b496218` |
| `BeVietnamPro-Bold.ttf` | `7f738fe5c43c8872807b20e2d30d42163618de8a4daf7f48a939adac32c16847` | google/fonts `ofl/bevietnampro/BeVietnamPro-Bold.ttf` @ `1cfe4cb468a703a247ca2d02410f80841b496218` |
| `OFL.txt` | `6b7f8f73609a25ea78c891e34cf37b06f8a676b7ea986e941e43b009110f2a85` | google/fonts `ofl/bevietnampro/OFL.txt` @ `1cfe4cb468a703a247ca2d02410f80841b496218` |

- **Họ font:** Be Vietnam Pro, **phiên bản 1.002** (`name` table: `Version 1.002;
  ttfautohint (v1.8.3)`), thiết kế riêng cho tiếng Việt.
- **Nguồn:** <https://github.com/google/fonts/tree/1cfe4cb468a703a247ca2d02410f80841b496218/ofl/bevietnampro>
  (commit `1cfe4cb468a703a247ca2d02410f80841b496218`, 2021-08-27 — "Be Vietnam Pro:
  Version 1.002; ttfautohint (v1.8.3) added (#3771)"). Nguồn gốc của dự án font:
  <https://github.com/bettergui/BeVietnamPro>.
- **License:** SIL Open Font License 1.1 — bản đầy đủ ở [`OFL.txt`](OFL.txt).
- **Vì sao chọn:** ứng viên đã nêu ở UI_STRINGS_V1 §5.1 (font thiết kế cho tiếng Việt,
  phủ Latin Extended Additional cho `ă â đ ê ô ơ ư` và đủ tổ hợp `ắ ộ ữ ợ ỹ`).

Quyết định dùng font (nhãn `design`) và giới hạn còn lại nằm ở
[UI_STRINGS_V1 §5.1](../../docs/specification/localization/UI_STRINGS_V1.md). Client nạp
font bằng `include_bytes!` nên không cần file lúc chạy.
