# VN C00 — idle sau hướng, bomb/bubble (metadata #320)

Nguồn runtime local: `/home/admin/boom-team/shots/320/claude-r1/capture.mkv` (ffv1,
1024x768, 30 s), không commit. Fixture VN `bc875…`/prefix `vn-grace-240`, fake server local
`vn149-factory-server.py`, Xvfb/Mesa llvmpipe. Video SHA:
`e3610325129d9ee3bb0425a98cf2af8a7d454fde6c1da01c3e9f2002b5a329a5`; stream khai 60 fps nhưng
PTS thực tế **29,65 fps** (890 frame / 29,983 s; phổ biến 33/34 ms, có 50/67 ms).
Actions local `actions.jsonl`; walk Left 2,00–2,64; Right 4,14–4,77; Up 6,27–6,91; Down 8,41–9,04;
bomb 10,54.

## R-198 — idle giữ hướng

Mỗi hướng được giữ 0,6 s, nghỉ 1,5 s. Ảnh crop local:
`/home/admin/boom-team/shots/320/dsh1/idle-directions-tight.png`.
- Left idle 3,40 s khớp **frame cuối Walk-left** (so khớp crop 0,2/255; frame 36–38 của đoạn đã resample 60 fps từ video gốc (không phải mẫu gốc độc lập)).
- Right/up/down idle 5,50/7,70/9,80 s giữ tương ứng mặt phải/lưng-up/mặt down, **không** trả về
  `idle.blink`.
- `idle.blink` chỉ thấy ở idle ban đầu/spawn (1,0 s); trong cửa sổ 1,5 s sau dừng không thấy Blink.
Chưa đo ngưỡng chuyển sang Blink sau >1,5 s; chưa đo BlinkSmall.

## R-199 — bomb/bubble overlap

- Bomb 11,0 s: bomb cyan nằm **dưới** nhân vật (nhân vật vẽ đè bomb) — khớp `ORDER_BOMB <
  ORDER_CHARACTER`.
- Bubble 13,5 s: bong bóng **phủ trên** nhân vật — khớp `ORDER_BUBBLE > ORDER_CHARACTER`.
- Chưa có item trên sàn/high object/fire riêng trong capture; không kết luận các lớp đó.
- Ảnh local zoom `bomb-11-zoom.png`, `trap-13.5-zoom.png`.

## Giới hạn

Một actor/map/mode; stream khai 60 fps nhưng PTS thực tế29,65 fps (ảnh phân tích đã resample60); không native locator/counter. A2 (thứ tự frame/đổi hướng)
không chốt: kịch bản dừng 1,5 s giữa các hướng, không quay đổi hướng giữa lúc đi; frame walk chỉ quan
sát được 5-pose lặp nhưng chưa đủ phân biệt 0..4/ping-pong. Không code/asset commit.
