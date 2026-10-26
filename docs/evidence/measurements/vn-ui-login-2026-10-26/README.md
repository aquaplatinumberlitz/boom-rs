# VN UI login 2026-10-26 — F-11a

Local evidence only (repo PUBLIC; no game art committed).

## Provenance / command

- Date: 2026-10-26; owner dsh2; issue #354; baseline main `d979af2f3e7bf5fc3e4347c6285d549ee7a6fc0f`.
- VN client copy: `/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240/game/ca_fixed_patched.exe`
  (`bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`).
- Private Xvfb 1024×768x24, `-displayfd`; Wine prefix `vn-grace-240/prefix`; local mock server
  `/home/admin/boom-team/tmp/vn149-factory-server.py` (no lobby/room/match actions).
- `wine Z:\home\admin\boom-rs-dsh\re-kr\.re-work\vn-grace-240\game\ca_fixed_patched.exe`;
  screenshots `import -window root`, no `wineserver -k`/pkill. Own process groups cleaned.
- First launch captured only the splash; one allowed retry waited 30 s. Raw files are local:
  `/home/admin/boom-team/shots/fidelity/F-11/login/`.
- Native canvas is **800×600** content scaled **1.28×** to the 1024×768 Xvfb screenshot
  (1024/800 = 768/600 = 1.28). Coordinates below are screenshot px / 1.28, rounded.

## Screenshots

| file | SHA-256 | observation |
|---|---|---|
| `01-login-initial.png` | `e5c2e9ac27088ffecb4a0cfde4e338bc397e9a748d7f03e1152a49740b08922a` | login screen; account already persisted `tester`; password empty; submit disabled/gray |
| `02-account-focus.png` | `1b174a1bd9d8e60c88832abb82ee3e9da6aa0296023be3df1da32de7aa7d7952` | account click/caret; no distinct focus ring visible |
| `03-account-typed.png` | `f7567cccd15efeeb533cf5a9a316f081c9710927d38e6721f2f5f08ca0d8fea2` | account text `tester` |
| `04-password-focus.png` | `555e3c588d199cfe657ffb79e6e3b5359ebae175f134751bddcb587128c3c867` | password caret/focus |
| `05-password-typed.png` | `2f0e76ac9d423c1c56f39c1cac65d00bcd79d57dcb793473ff60932035d63541` | masked password; submit enabled blue |
| `06-submit-hover.png` | `2fed9e9012d3666a059c0daefa5bd00c259c24a28069d7311cb1bc937cb70f70` | cursor on submit; no clear hover color change |

No submit was clicked; no lobby/room/match/result was opened.

## Coordinates (logical 800×600; screenshot/1.28)

| element | logical rect (x,y,w,h) approx. | evidence |
|---|---:|---|
| login panel | 284,414,243,130 | 01; orange panel `NGƯỜI CHƠI 1` |
| account field | 396,462,107,18 | 01/02; label `NHÂN VẬT` |
| password field | 396,484,107,17 | 04; label `MẬT KHẨU` |
| 1P | 195,429,78,80 | 01; left blue orb |
| 2P | 527,429,78,80 | 01; right green orb |
| TÌM MK | 184,552,86,35 | 01; bottom row |
| ĐĂNG KÝ | 301,552,85,35 | 01; bottom row |
| ĐĂNG NHẬP | 402,552,90,35 | 01; gray disabled initially, blue after password |
| THOÁT RA | 516,552,86,35 | 01; bottom row |

Coordinates are visual bounds, ±2 screenshot px (±1.6 logical px) due border/antialias. Font family/size and exact glyph metrics were not measured; VN uses a bitmap UI font, labels uppercase.

## boom-rs current (for contrast, not committed VN art)

- Source `crates/boom-client/src/ui/layout.rs::login_layout` logical 800×600:
  - title `24,12,752,36`; account `24,130,400,44`; password `24,206,400,44`;
  - buttons `y=496`, `140×40`, gap8: `login.submit`, `login.register`, `quit`.
  - background element at side `636,104,156` (D-04 uses color/frame, not JPEG).
- `boom-rs-login-current-800x600.png` SHA `df11a055280613faa1f0ff1dc5b73de21e02da4ef86b0857a6ab75ac0e83657a`
  (local, 800×600).
- Differences: VN has centered panel/1P/2P, field x≈396/y≈462, four bottom buttons including
  `TÌM MK`; boom-rs fields at x24/y130 and three buttons at y496, no 1P/2P/TÌM MK. Submit VN is
  disabled until password; boom-rs submit is enabled unconditionally. These are layout/behaviour
  gaps, not the accepted D-04/D-05 background/text-asset differences.

## Limits / unknown

- One VN capture on one map/prefix; account field was pre-populated `tester` by the client/profile.
- Focus/hover are visually weak: no distinct focus ring or submit hover color observed; only
  caret/typed and disabled→enabled transitions. Cannot distinguish "no style" from lost pixel detail.
- No KR comparison, no lobby/room/match, no image files committed; D-04/D-05 remain accepted.
