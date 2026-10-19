> **Tài liệu lịch sử, không phải kế hoạch điều hành hiện tại.** Đọc [docs/README.md](../../README.md) và nguồn chuẩn được dẫn ở đó.

# Boom Online — Master Plan: RE, Cross-Platform Port & Remaster

> **KR product scope update (2026-10-19):** [KR_RELEASE1.md](../../planning/SCOPE_V1.md) and
> [KR_FEATURE_CATALOG.md](../../specification/FEATURE_CATALOG.md) define baseline `KR-00967-V1-scope-1`.
> KR is now the rewrite target; VN remains a reference knowledge base. Old KR scope gates and
> pack-key unknowns below are historical tracking entries, not blockers to the approved V1 scope.

> Status: **APPROVED FOR TRACKING (2026-10-18).** Planning document; producing it modified no
> binaries, ran no game executables, and made no connections to game services.
> Evidence is cited by relative path. Each claim is tagged **[VERIFIED]** (observed in-repo
> while writing this plan), **[DOC]** (asserted by an existing doc, not re-verified),
> **[HYPOTHESIS]**, or **[UNKNOWN]**.

## 1. Executive summary

Boom Online is the VNG Vietnamese build of Nexon *Crazy Arcade / BnB* (source branch
`VBNB_116`). The original server is dead and source is lost; only clients remain. Prior
work (mostly in Vietnamese, under `docs/`) has, for the **VN build**, unpacked the packed
client, rebuilt a runnable dump, reverse-engineered the TCP protocol and asset pack format,
and driven the dump through a full match (login → channel → lobby → room → in-match bomb
placement) against a Python spec server (`tools/fakeserver.py`) and a Rust reimplementation
(`server/`). A **Korean build** (Nexon patch `00967`, 2026-06-10) sits at `boomKR/`
(git-ignored); only a *plan* exists for it (`docs/archive/plans/KR_RE_PLAN_2026-10-18.md`), plus one unexplained `.i64`.

Strategic recommendations:

1. **Treat the VN work as the canonical knowledge base.** It is the only build driven to
   gameplay. The durable asset is the **protocol + asset-format + server spec**, not the
   patched binary.
2. **Do not deepen the KR investigation yet.** KR adds an embedded web (CEF) login path and a
   different online/anti-cheat stack. For an *offline preservation* goal, online auth and
   anti-cheat components are out of scope by design.
3. **The remaster is a clean-room reimplementation** of game rules + a documented protocol +
   a converted asset pipeline — **not** a patched legacy binary and **not** shipping any
   legacy anti-cheat component.
4. **Single best next action: PRES-001..004 (preserve & stabilize the repo).** The working
   directory holds ~8 GB of costly-to-regenerate, partly copyrighted artifacts and an IDA
   database whose component files changed recently. Everything else depends on them.

## 2. Verified repository state & evidence references

### 2.1 Layout (observed 2026-10-18)

| Area | Path | State |
|---|---|---|
| Docs | `README.md` (243 lines), `docs/` (~19 `.md`, ~21 `.png`) | **[VERIFIED]** present, detailed, mostly Vietnamese |
| Tracked files | ~205 files; `main` pushed to `origin` | **[VERIFIED]** `git ls-files`, `git status -sb` |
| Ignore rules | binaries, `assets/`, `logs/`, `wine/`, `thirdparty/`, `boomKR/` ignored | **[VERIFIED]** `.gitignore` (boomKR added `8f25060`) |
| Working tree | `server/src/{packets,session}.rs`, `server/tests/bomb.rs`, `tools/fakeserver.py` modified; `tools/gdb/spawnitems.gdb` untracked | **[VERIFIED]** — owned by a parallel session, not part of this plan |
| Analysis output | `decomp/` (51 files: `ida_names.json`, `iat_resolved.json`, `handlers/`, `net/`, `vm_*.json`) | **[VERIFIED]** listing |
| IDA DB (VN) | `unpacked_ca.exe.i64` (312 MB) + unpacked `.id0/.id1/.id2/.nam/.til` | **[VERIFIED]**; unpacked parts newer than `.i64` → **DB likely open / unsaved** |
| Binaries (VN) | `unpacked_ca.exe`, `ca_fixed.exe`, `ca_fixed_patched.exe`, `ca_patched.exe`, `ca_devirt.exe` | **[VERIFIED]** present (ignored) |
| Assets | `assets/` 110,902 files, 665 MB | **[VERIFIED]** |
| Logs | `logs/` 883 files, 2.9 GB (`snap` 1.8 GB, `dumps` 800 MB) | **[VERIFIED]** |
| Server (Rust) | `server/` src (6), tests (`golden.rs`, `bomb.rs`, `golden.txt`), `target/` 289 MB | **[VERIFIED]** present; tests **not run** for this plan |
| Tools | `tools/` 86 files (Python / PowerShell / sh / gdb / C) | **[VERIFIED]** |
| Third-party | `thirdparty/` 803 MB (software Mesa / D3D9) | **[VERIFIED]** |
| VN run copy | `wine/Boom/` 1.1 GB (= `~/Downloads/Boom` + 7 runtime files) | **[VERIFIED]** `diff -rq` |
| Pristine VN | `~/Downloads/Boom` (98 files) | **[VERIFIED]** |
| KR build | `boomKR/` 1.8 GB: 169 patch files (match `files.txt`) + `CA.exe.i64`, `files.txt`, `files.nfo2`, `release_date.txt` | **[VERIFIED]** |

### 2.2 Evidence index

Read in full: `README.md`, `docs/archive/re/VN_PROGRESS_LOG.md`, `docs/archive/plans/KR_RE_PLAN_2026-10-18.md`.
Sized but not read in full: `docs/specification/assets/PACKS.md`, `docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `docs/research/findings/VN_DEVIRT_STATUS.md`,
`docs/archive/re/VN_LOBBY_LOG.md`, `docs/oc_*.md`, `docs/vm*.md`, `docs/specification/assets/MAPS.md`, `docs/research/findings/VN_FEATURES_REFERENCE.md`,
`docs/research/findings/VN_RUNTIME_REFERENCE.md` — their status below is **[DOC]** unless re-verified.

### 2.3 Inspection limitations

- Decompiled C, JSON analysis files, and both `.i64` databases were not opened.
- `server/` tests were not compiled or run; "byte-exact vs fakeserver" is **[DOC]**.
- No `tools/*` script was executed; status reflects docs + file presence.
- KR `CA.exe.i64` provenance is **[UNKNOWN]**.

## 3. Current VN reverse-engineering status

Legend: ✅ driven to a working result · 🟡 partial · ⬜ not started.

| ID | Subsystem | Status | Evidence | Limitation / open question |
|---|---|---|---|---|
| VN-NET | TCP protocol (frame, XOR, seq, CRC8) | ✅ **[DOC]** verified vs real client | `docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `tools/boomproto.py`, `tools/verify_emu.py` | 2nd header byte, 6×u32 in opcode 0 **[UNKNOWN]** |
| VN-DISP | Opcode → handler map | ✅ **[DOC]** 520 opcodes / 59 dispatchers | `decomp/opcode_dispatch.json`, `decomp/handlers/` | handler pseudocode not all audited |
| VN-LOGIN | Hello + account login | ✅ **[DOC]** | `docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `docs/archive/re/OC_D-LOGIN.md` | — |
| VN-LOBBY | Channel → square → room → start | ✅ **[DOC]** | `docs/archive/re/VN_LOBBY_LOG.md` | map-select dialog lists 0 maps; room/user lists sent empty |
| VN-MATCH | In-match (stage 13) | 🟡 **[DOC]** sim runs, walk, bomb → explode, death → respawn, end → room, item pickup | `docs/archive/re/VN_PROGRESS_LOG.md`, `docs/archive/re/OC_E-GAME.md` | single-player only; multi-player relay, real fuse timing incomplete |
| VN-ASSET | `.idx/.idd` + sprite → PNG | ✅ **[DOC]** 108,726 PNG, 965 maps, 645 ogg | `docs/specification/assets/PACKS.md`, `tools/idf.py`, `tools/i16.py` | — |
| VN-MAP | `.map` / `Attr/*` / animation | ⬜/🟡 **[DOC]** | `docs/specification/assets/MAPS.md`, `decomp/CGamePlayMap__Load.c` | format not fully decoded |
| VN-UNPACK | Unpacked dump `unpacked_ca.exe` | ✅ **[DOC]** | README §1 | IAT boot-dependent, frozen in `decomp/iat_resolved.json` |
| VN-DUMPFIX | Protected blocks in runnable dump | 🟡 **[DOC]** 15 blocks handled | `docs/research/findings/VN_DEVIRT_STATUS.md`, `tools/devirt.py` | some sites still skipped |
| VN-IDA | RTTI naming | ✅ **[DOC]** 2558 classes, ~20.7k names | `rtti.txt`, `decomp/ida_names.json` | DB likely open/unsaved |
| VN-SRV | Rust server | 🟡 **[DOC]** runs a full match vs dump | `server/`, `docs/archive/re/VN_PROGRESS_LOG.md` §12 | structured C→S parsing, multi-player incomplete |

**Durable vs fragile:** VN-NET / VN-DISP / VN-ASSET / VN-SRV spec are **durable** (they carry
into the remaster). VN-UNPACK / VN-DUMPFIX / patched exes are **observation scaffolding** —
useful to watch original behavior, never shipped.

## 4. KR build assessment & unresolved questions

Per `docs/archive/plans/KR_RE_PLAN_2026-10-18.md` (**[DOC]**, measured on-file; plan not executed):

| Aspect | VN | KR | Reuse category |
|---|---|---|---|
| Main exe | `ca.exe` 7.08 MB, PE subsystem 5.01 | `CA.exe` 10.1 MB, subsystem 6.00, packed | C |
| Login path | plain socket | launcher + embedded CEF web (`Dwarf/`) | C |
| Anti-cheat stack | HackShield + BlackCipher | GameGuard + BlackCipher (+ BlackMon, eTracer) | C (policy-gated) |
| Server config | `cavn.ini` readable | `CAKR.ini` binary | C |
| Asset packs | 4 | 13 (9 new `Fx*`) | B |
| Pack format/keys | known | 4/13 readable with VN keys | A |

**Shared-file hash check [VERIFIED]:** 14 byte-identical (`binkw32.dll`, `cawh.sys`,
`procobsrv.sys`, `d3dx9_30.dll`, `dtm.dll`, `dwk.dll`, `ijl15.dll`, `mss32.dll`, `mssogg.asi`,
`nda.dll`, `npa.dll`, `cxdefault.fft`, `fxasset.idd/.idx`). Differing: `CA.exe`, `charte`,
`emote`, `fx`, `gameguard.des`, `ngclient.aes`, `nm.bik`, `dbghelp.dll`, `nmcosrv.exe`,
`nmco3.dll`, `BlackCipher/config.bc`. *Identical hash proves byte identity only, not identical
in-game behavior.*

**Open KR questions [UNKNOWN]:** provenance of `boomKR/CA.exe.i64`; meaning of the 8-hex codes
in `files.txt` and the `files.nfo2` format; keys for the 9 new `Fx*` packs; whether KR framing
matches VN; whether KR is runnable offline at all; availability of a legitimate KR account.

## 5. VN → KR reuse matrix

| Capability | Category | Action |
|---|---|---|
| Pack reader `tools/idf.py` | **A** (4 packs) / **B** (9 `Fx*`) | KR-ASSET-001 |
| Sprite decode `tools/i16.py`, `docs/specification/assets/PACKS.md` | B | reuse; re-verify on KR sprites |
| Pack keys `95551996` / `1906092848` | A (proven on 4 KR packs) | — |
| Protocol spec `docs/specification/networking/LEGACY_VN_PROTOCOL.md`, `tools/boomproto.py` | C | comparison baseline only |
| Static analysis methods (`docs/research/findings/DEVIRT_METHOD.md`) | B | gated by KR-002 |
| Rust server `server/` | C for KR | VN-only until KR protocol is shown to match |
| `.gitignore` discipline | A | done (`boomKR/`) |

Rule: **A** = use directly; **B** = reuse tool, re-verify finding; **C** = independent investigation.

## 6. Subsystem work breakdown

Effort S/M/L/XL (relative, not calendar); priority P0–P3.

### 6.1 Preservation & repo (P0)

| ID | Objective | Deliverable | Verification / acceptance | Eff | Pri | Parallel |
|---|---|---|---|---|---|---|
| PRES-001 | Inventory all >10 MB / copyrighted artifacts with regen source; harden ignore rules | `docs/archive/re/ARTIFACTS_HISTORY.md` | `git status` can never stage a binary; every artifact listed | S | P0 | yes |
| PRES-002 | SHA-256 manifests of `~/Downloads/Boom` and `boomKR/` | `docs/inventory/*.sha256` | re-hash matches | S | P0 | yes |
| PRES-003 | Resolve IDA DB open/unsaved state; back up `.i64` | backup + note | `.i64` authoritative & backed up | S | P0 | no |
| PRES-004 | Reconcile parallel-session working-tree changes | owner decision | no lost or clobbered work | S | P0 | no |

### 6.2 VN consolidation (P1)

| ID | Objective | Deliverable | Verification | Eff | Pri | Parallel |
|---|---|---|---|---|---|---|
| VN-001 | Re-run spec checks (`boomproto` roundtrip, `idf.py` listing, `verify_emu`, `cargo test`) | `docs/verification.md` | commands exit 0; `[DOC]` → `[VERIFIED]` | M | P1 | yes |
| VN-002 | Audit remaining OpenCode docs (`oc_c-square`, `oc_a-room`, `oc_e-game`) against IDA | audit notes in each doc | offsets match IDA | M | P1 | yes |
| VN-003 | Complete `.map` / `Attr` / animation formats | `docs/specification/assets/MAPS.md` + loader | round-trip a real map | L | P1 | yes |
| VN-004 | Structured C→S parsing in Rust (77/97/139/140/111/112/145) | `server/src` parsers + tests | golden tests | L | P1 | partly |
| VN-005 | Multi-player: lobby/room lists, teams, relay 111/152/151/120 | server + spec | 2-client match (needs harness) | XL | P2 | no |

### 6.3 KR (gated — only if KR is an approved goal)

| ID | Objective | Gate | Deliverable | Eff | Pri |
|---|---|---|---|---|---|
| KR-002 | **Decide KR scope & legal posture** (online auth, anti-cheat, account) | — | written decision | — | P2 |
| KR-ASSET-001 | Extract the 4 VN-key packs; add KR stems to `KEYS` | — | KR asset subset | M | P2 |
| KR-001 | Decode patch manifest (`files.nfo2`, 8-hex codes) | — | manifest schema | M | P3 |
| KR-ASSET-002 | Recover 9 `Fx*` pack keys | KR-002 | key table | L | P3 |
| KR-003 | Static PE/structure comparison of `CA.exe` (no running/unpacking) | KR-002 | `docs/kr.md` | M | P3 |

> KR runtime work is intentionally not broken down here pending KR-002. An offline
> preservation remaster does not need it: it reimplements rules and protocol instead of
> running the protected client.

### 6.4 Port & remaster

> Detailed Rust rewrite plan (fidelity spec, Feel Oracle, crate layout, R0–R7 roadmap): `docs/archive/plans/REWRITE_PLAN_2026-10-19.md`.

| ID | Objective | Deliverable | Eff | Pri |
|---|---|---|---|---|
| PORT-001 | Canonical English protocol spec merging `protocol.md`, `oc_*`, `server/` | `docs/spec/protocol.md` + machine-readable opcode table | L | P1 |
| PORT-002 | Asset-format spec + converter to open formats (PNG / JSON / OGG) | `docs/spec/assets.md`, `tools/convert/` | L | P2 |
| PORT-003 | Engine-agnostic game-rules model (bomb, clock, map, turn state machine) | `docs/spec/rules.md` | L | P1 |
| PORT-004 | Engine decision after PORT-001/003 requirements | decision record | S | P2 |
| PORT-005 | Vertical slice: new client + `server/` play one offline match from converted assets | runnable slice | XL | P2 |
| PORT-006 | Regression harness: replay recorded original-client traces against new server | `tests/replay/` | M | P2 |

## 7. Team responsibilities & handoff rules

| Workstream | Owns | Hands off | Reviews |
|---|---|---|---|
| WS-LEAD (tech lead) | roadmap, this plan, PORT-004 | decision records | merges to `main` |
| WS-VN (baseline & docs) | VN-001/002 | audited specs → WS-PORT | runtime findings |
| WS-KR (gated) | KR-* | KR specs | — |
| WS-RUNTIME (protocol) | VN-004/005, PORT-006 | traces, golden files | WS-SERVER |
| WS-ASSET | VN-003, PORT-002, KR-ASSET | format spec + converted assets | — |
| WS-SERVER | `server/`, PORT-001 | protocol spec + server | WS-RUNTIME |
| WS-CLIENT | PORT-004/005 | prototype | WS-LEAD |
| WS-QA | PORT-006, CI | regression reports | all |

One owner per investigation; no duplicate ownership without a stated reason.

**Shared evidence format (mandatory for humans and AI agents):** every claim carries
(1) a tag `[VERIFIED|DOC|HYPOTHESIS|UNKNOWN]`; (2) evidence by relative path + address/line or
command; (3) author, date, model. Findings go in `docs/`, never only in chat or logs. Addresses
name their binary (`unpacked_ca.exe`); note decimal vs hex.

**Integration checkpoints:** end of each roadmap phase; WS-LEAD signs off exit criteria before
the next phase opens.

## 8. Target architecture & migration strategy

### 8.1 Principles

- **Separate recovered rules from legacy platform code.** Rules (bomb timers, tile collision,
  turn/clock state machine, scoring) form an engine-agnostic core (PORT-003). Win32 / D3D9 /
  packer specifics are not carried over.
- **The protocol is the contract.** New client and `server/` implement `docs/spec/protocol.md`.
  Original wire compatibility is required insofar as it lets recorded original traces validate
  the server (PORT-006); the shipped remaster may adopt a clean v2 protocol once parity is proven.
- **No shipped anti-cheat or online-auth dependency.** Offline/LAN-first.

### 8.2 Decisions required before choosing an engine (PORT-004)

Target platforms (desktop / web / mobile); 2D sprite+tile rendering needs; input and netcode
model; team language (the existing Rust server suggests Rust or Rust-interop **[HYPOTHESIS]**);
**asset licensing** — original art likely cannot be redistributed, which may require
replacement art. Record these, then compare 2–3 concrete engines against them.

### 8.3 Migration & rollback

Incremental, each step tagged in git: (1) specs frozen → (2) converter yields open-format
assets → (3) server parity proven by replay → (4) prototype slice → (5) feature by feature.
Rollback point = last green PORT-006 replay. The legacy observation stack (dump + Wine +
fakeserver) remains available read-only as ground truth.

### 8.4 Compatibility vs remaster

| Compatibility (must) | Remaster (optional) |
|---|---|
| original gameplay rules reproduced | modern resolutions, UI scaling |
| map / tile / bomb behavior parity | new renderer, accessibility |
| protocol validated by real traces | new maps / modes, matchmaking |
| assets faithfully converted | replacement art if licensing requires |

## 9. Prioritized phased roadmap

| Phase | Goal | Key tasks | Depends | Exit criteria |
|---|---|---|---|---|
| **P0 Preserve** | nothing irreplaceable at risk | PRES-001..004 | — | inventory + manifests exist; IDA state resolved; working tree reconciled |
| **P1 Consolidate VN** | VN knowledge trustworthy | VN-001/002/003, start PORT-001/003 | P0 | checks re-run & recorded; OpenCode docs audited; map format decoded; spec drafts |
| **P2 KR scope gate** | decide KR in/out | KR-002 (+ KR-ASSET-001, KR-001 if in) | P0 | written scope + legal decision |
| **P3 Representative subsystem** | prove one subsystem end-to-end | VN-004, PORT-002 | P1 | e.g. bomb/clock spec'd, reimplemented, replay-validated |
| **P4 Prototype** | validate highest-risk assumptions | PORT-004/005/006 | P1, P3 | new client + `server/` play one offline match from converted assets |
| **P5 Incremental port** | feature migration | VN-005, remaining PORT | P4 | features land behind green replays |
| **P6 QA & compat** | gameplay / perf / regression | WS-QA | P5 | compatibility matrix passes |
| **P7 Release & maintain** | ship, sustain | WS-LEAD / QA | P6 | reproducible builds, docs, licensing resolved |

**Single best next task: PRES-003 then PRES-001.** The IDA database (~20.7k names) and memory
snapshots cost weeks to regenerate, and the working tree is being edited by a parallel session.
Preservation is cheap, reversible, and unblocks every later phase.

## 10. Risks, dependencies & decision points

| ID | Risk / decision | Likelihood | Impact | Mitigation / owner |
|---|---|---|---|---|
| R-01 | Commit/push of copyrighted binaries or assets | med | high | ignore rules + pre-push check; WS-LEAD |
| R-02 | IDA DB unsaved → lost names | med | high | PRES-003 first |
| R-03 | Parallel sessions clobber each other | med | med | PRES-004; one owner per file set |
| R-04 | `[DOC]` claims not reproducible | med | med | VN-001 |
| R-05 | No 2-client harness → multiplayer unverifiable | med | med | build in VN-005 / PORT-006 |
| **D-01** | KR scope (offline rules-only vs online/auth) | — | high | KR-002, WS-LEAD |
| **D-02** | Asset licensing for redistribution | — | high | legal review before PORT-002 ships |
| **D-03** | Anti-cheat / ToS posture | — | gates KR runtime work | offline-only; no shipped AC component |
| **D-04** | Engine choice | — | long-term maintenance | PORT-004 after P1 |

## 11. Acceptance criteria & definition of done

- **P0 done:** no binary stageable; SHA-256 manifests committed; `.i64` saved + backed up;
  working-tree changes reconciled with their owner.
- **P1 done:** every VN-* check re-run with recorded result; all `oc_*` docs audited; English
  protocol, rules and asset specs exist with tagged claims.
- **P2 done:** written KR scope + legal decision.
- **P4 done:** a new client + `server/` play one full offline match from converted assets with no
  legacy binary in the loop; ≥1 recorded original trace replays green.
- **Project DoD:** reproducible build from source + converted assets; no dependency on the packed
  legacy exe or any anti-cheat component; licensing policy resolved; a newcomer can rebuild from
  the docs alone.

## 12. Immediate next actions (ordered)

- [ ] **PRES-003** — check whether IDA has `unpacked_ca.exe.i64` open with unsaved names; back it up.
- [ ] **PRES-004** — reconcile uncommitted `server/`, `tools/fakeserver.py`, `tools/gdb/spawnitems.gdb` changes with their session.
- [ ] **PRES-001** — artifact inventory with regen sources (`docs/archive/re/ARTIFACTS_HISTORY.md`).
- [ ] **PRES-002** — SHA-256 manifests for `~/Downloads/Boom` and `boomKR/`.
- [ ] **VN-001** — re-run README "Kiểm tra nhanh" checks + `cargo test`; record in `docs/verification.md`.
- [ ] **KR-002** — explicit decision on KR scope and legal posture before any KR runtime work.
- [ ] **PORT-001 / PORT-003** — start English protocol and rules specs from existing VN docs.
