# Item effects VN — Bomb/Fire raw metadata (2026-10-24)

Task/root [#149](https://github.com/aquaplatinumberlitz/boom-rs/issues/149).
New independent metadata only; no RULES/profile/index change. Raw media, screenshots, decompile and full
binary are **not** published.

## Provenance

- Fixture: VN VBNB_116 patched offline, SHA
  `bc875ba5765cc112e924f46e75d0ace112904c7801126e46bc0abb1c9cb576b1`, base `0x400000`, i386.
- Shared private fake server: `/home/admin/boom-team/tmp/vn240-flame-server.py`, SHA
  `fa65dc31bb8937e0a798c157bf5f4c8a95f99de4db8492402497973005ab5a45`; `FLOOR_ITEMS`, `ITEM_ACK`,
  `BOMB_PLACE`, `BOMB_ECHO`.
- Raw private root: `/home/admin/boom-rs-dsh/re-kr/.re-work/vn-grace-240`.
- `harness-used.py` per case is copied from the exact root supervisor script and hashed in
  [manifest.json](manifest.json). `packets.log`/`server.log` are the same private stream.
- Manifest has size/SHA-256 for every raw file, including PNG; only metadata is here (no raw/media content).

## Cases

| Case | Raw run | Status | Result |
|---|---|---|---|
| Bomb kind0 | `host-item-bomb-149-r1` | positive | first 111 x0 `r18=1`; at target40/actual45 Space **no111**; pickup 117/158 kind0/index0 at x80; second 111 x2 serial20 `r18=1` id2; third target40 **no111**; no151. |
| Fire kind1 | `host-item-fire-149-r2` | positive | first 111 x0 `r18=1` id1; control target40 **actual45** (raw samples x45; supervisor DONE45) **no111**; pickup 117/158 kind1/index0 at 18:01:00.966; Space before first151 **no second111** (capacity unchanged); first151 18:01:07; second 111 x2 serial20 **`r18=2`** id2 at 18:01:08; second151 18:01:16 traps own bomb and is outside comparison. |
| Fire kind1 failed control | `host-item-fire-149-r1` | failed, excluded | `MOVE_TO_X target80 origin80` still selected `Left`; delayed keys drifted x80→40 before first151; no second111/`r18` after pickup; trap at x40. |

### Important parser caveat

The fake-server parser field named **`power` is `1` in both Fire 111 records**. Do **not** report
`power 1→2`. The observed field is **`r18`** in the raw 111 record/source serializer (legacy docs/source
name it range/vf31): first1, second2. The server's S152 snapshot is serialized from the same record
(`r18` first1/second2); this is metadata provenance, not a new native getter.

## Controls / scope

- Bomb control: before Bomb pickup, second placement at actual45 produced **no111**; after pickup,
  second produced111 while first stayed alive (`BOMB_ECHO=0`); third attempt produced no111. This
  establishes **capacity 1→2 in this case**, not max cap.
- Fire control: before first151, after pickup, the attempted second bomb produced no second111
  (capacity unchanged); after first151, second111 was accepted with `r18=2`. The failed Fire1 run is
  retained only as evidence of a helper/control failure, not Fire behavior.
- Scope is **item effect/protocol observation only**: no R032/range rule, cap max, Roller level,
  C00/C01 stats, native getter/writer, profile value, or public media/decompile claim.
- Omissions: raw PNG/winedbg/xvfb content not published; no full map/binary analysis.

## Next edge

Next edge (prep only, no public plan): root uses the existing `boom-assets` map decoder to extract
the authentic Ocean01 collision/empty-floor path and select ≥10 distinct reachable pickup cells with
an escape path; if that path is unavailable, one bounded runtime receiver/getter probe at known call
site `0x5B1FB5` is the fallback. Do not extrapolate cap max from one Bomb pickup or the third rejection.
