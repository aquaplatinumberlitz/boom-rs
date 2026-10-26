# VN R-192 — BOMB_ARG 1/2/3, motion ordinal lửa (metadata #330)

Parent thật của head: `e39abce145da72064e1eb8c761df4c6644b48356` (baseline tiếp quản ghi trong issue:
`dc5241b4fb8c0203bf89142b033e0c26a82926d3`). Không sửa code/profile/binary/pack/IDA DB; không launch VN lại.

## Vị trí raw (local, không commit)
`/home/admin/boom-team/shots/fidelity/F-05/r192-bomb-arg/r{1,2,3}/` — đã chuyển từ
`.re-work/fidelity330-probe` sau cleanup; ảnh/video chỉ ở đây, không đưa raw game art vào repo.

## Provenance
- Executable VN: `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`
  (`/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/game/ca_fixed_patched.exe`).
- Server: `/home/admin/boom-team/tmp/vn240-flame-server.py`
  SHA `fa65dc31bb8937e0a798c157bf5f4c8a95f99de4db8492402497973005ab5a45`.
- Probe: `r1/probe.gdb` `0a04fab9f02b7bb541223a51e1e497fe8e56b34cbebe4abb466311117825af5b`,
  `r2/probe.gdb` `6b6c91935e6f65f36ef94141518945d0cef2549d95d8deda9cf1db2b82a32338`,
  `r3/probe.gdb` `bf4e1283404d4e4150d4378470df2d41f5208940b04554fa43b9ce99d27e4d0f`.
- Harness `r1/harness.py` `18419d88b85d876b908f91fdce825d9c6b7614f78755a9bfe2c18f68f76b99e6`;
  r2/r3 cùng nguồn, khác output path.
- Prefix/game sao chép `vn-grace-240`, Xvfb `:142`, Mesa llvmpipe; S->C 151 sau C->S 111:
  `BOMB_ECHO=8`, `BOMB_PLACE=1`, `BOMB_ARG` 1/2/3.

## Packet thật gửi + mẫu
| Run | packet 151 | observations | samples |
|---|---|---|---|
| arg1 | `... (probe single id 1, arg 1)` | `r1/flame-render-observations.jsonl` | **7** |
| arg2 | `... id 1, arg 2` | `r2/flame-render-observations.jsonl` | **6** |
| arg3 | `... id 1, arg 3` | `r3/flame-render-observations.jsonl` | **7** |

Observations SHA-256: r1 `6a4b9ebc51c519e497fc0db5e4cf84085b35bfc831f77cff59d131227bf20efb`,
r2 `701803a33007b2d08829b4ec7c7e02acd0d0d163fa479a5d071fa21c5e8de943`,
r3 `0bc368aaeca1836a3d907fcd6f2d727cbf07eb781286e3cd091e2fbbf1f653ec`.
Packets.log SHA-256: r1 `c5187c003ec61c0d4a29d93334eb547a55b6cf0464a0cdf9810ae98d24bd399e`,
r2 `8a75f198e1b0c2b35c913e3648306700a0b39d3fc5c9f8066e641f3afbd0d786`,
r3 `83d729243eb26c24b96b1a59099a81ea6ad135968ed4eb50fe913cc25ef7143c`.
Raw absolute locator: `/home/admin/boom-team/shots/fidelity/F-05/r192-bomb-arg/r{1,2,3}/{flame-render-observations.jsonl,packets.log}`.

## Ordinal thực đọc
- arg1: L15, C0, R45, D60 (counter1); thiếu U.
- arg2: L15, R45, D60 (counter15); thiếu C/U.
- arg3: L15, C0, R45, D60 (counter1); thiếu U.
- **Không** thấy power2 1/16/31/46/61 hay power3 2/17/32/47/62.
- `BOMB_ARG` một mình không chứng minh power/state selector; hậu tố `*02/*03` vẫn unknown.
  Counter15 arg2 không đủ gán power (counter khác motion; R-164 đã tách).

## Cleanup
Ba harness tự kết thúc; kiểm không còn `winedbg`, `vn240-flame-server`, `Xvfb :142`,
`ca_fixed_patched`/wineserver/wine64 hay launcher con của task. Không dùng `wineserver -k`/`pkill`;
prefix đã nhả. Không chạy Fire+ack; không chạy map/F-09.
