# HexForge-Rust — 100% Pure-Rust 4X Strategy Game

[![Latest Release](https://img.shields.io/github/v/release/OrionFlaxan/HexForge-Rust?include_prereleases&label=Download%20Release&logo=github&color=2ea44f)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest)
[![Android 4-ABI APK](https://img.shields.io/badge/Android-arm64--v8a%20%7C%20armeabi--v7a%20%7C%20x86__64%20%7C%20x86-3DDC84?logo=android&logoColor=white)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk)
[![Pure Rust](https://img.shields.io/badge/Engine-100%25%20Rust%20%2B%20Rayon%20%2B%20GLES3-DEA584?logo=rust&logoColor=white)](#the-4-anti-troop-lag-pillars)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**HexForge-Rust** is a brand-new, **100% Pure-Rust 4X strategy game** for **Android (all 4 ABIs, API 21+, Portrait & Landscape)** and **Desktop** — engineered from scratch with **zero JVM, zero garbage collector, multi-threaded Rayon AI, Corps/Army stacking, O(V) Theater Flow-Fields, and Background Speculative AI** so **Huge 22–24 civ maps past Turn 300+ run in milliseconds on any phone**.

---

## Download (v0.5.0)

| Platform | Direct Download | Size | Notes |
|---|---|---:|---|
| **Android (Phone / Tablet)** | [**Download `HexForge-Rust-Android.apk`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk) | **~8.2 MB** | Universal APK (Android 5.0+, Portrait & Landscape, true DPI scaling) with all 4 ABIs (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`). Installs **side-by-side** alongside Unciv! |
| **Linux Desktop (x86_64)** | [**Download `HexForge-Rust-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Linux-x86_64) | **~2.1 MB** | Run `chmod +x HexForge-Rust-Linux-x86_64 && ./HexForge-Rust-Linux-x86_64` |
| **Headless Benchmark CLI** | [**Download `HexForge-Bench-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Bench-Linux-x86_64) | **~1.3 MB** | Run 350-turn Huge/Large benchmarks on any machine |
| **All Releases** | [**GitHub Releases**](https://github.com/OrionFlaxan/HexForge-Rust/releases) | — | Release notes, checksums & artifacts |

---

## What's Inside (v0.5.0)

- **Dual Live-Switchable UI Themes (`PIXEL ART` & `MODERN SIMPLE`) + Procedural Sprites:**
  - Switch live in **Options** (`options.toml`) between **Pixel Art (Retro 4X)** (beveled RPG HUD panels, dithered pixel terrain, stepped pixel mountains & forests, procedural 12×12 unit/city/improvement pixel sprites) and **Modern Simple (Clean)** (flat minimal high-contrast cards & geometric vector icons).
  - Miniature procedural sprites appear directly inside **City Production Tabs** and the **Worker Actions Sheet**!
- **24 Authentic Historical Civilizations with Leaders, Detailed Buffs, 24 Unique Units & 24 Unique Buildings:**
  - Play as or battle **24 historical empires** (*Rome, Egypt, China, Japan, England, France, Germany, Greece, Persia, India, Mongolia, Russia, Arabia, Inca, Aztec, Carthage, Byzantium, Spain, Ottomans, Babylon, Korea, America, Norway, Indonesia*) — each with a historical **Leader**, **detailed gameplay buffs** (*Japan Bushido* full strength when wounded, *Babylon* starts with *Writing*, *Norway* starts with *Sailing* + extra pillage gold, *Egypt/France* Wonder speed, *Rome/Inca* `-1t` Worker speed, *Germany* Rush-Buy discount, and more!), a **Civ-exclusive Unique Unit (UU)**, and a **Civ-exclusive Unique Building (UB)** (`u128` O(1) `BuildingBits` supporting all **62 structures** & **82 units**!).
- **Mandatory Pre-Turn Choice Blocker (Never Forget Research or Production Again!):**
  - The bottom-right button automatically transforms into **`CHOOSE TECH`** or **`BUILD: <City>`** (focusing the camera & opening that city) whenever research or production is unset, turning green as **`END TURN *`** only once every target is set (plus optional **Force Policy Pick** toggle in Options).
- **Separated City Production Tabs (`UNITS` | `BLDGS` | `WONDERS` | `BUILT`) & 16 Realistic Tile Improvements:**
  - Multi-turn builds (`1–4t`), **Chop Forest (`+20P`)**, **Clear Jungle**, **Drain Marsh (`→ Grassland`)**, **Roads & Railroads (`+15% City Prod`)**, **Forts & Forward Airbases (`+50% Def` + Air Rebase)**, **Combat Pillaging & Repair**, and **`AUTO WORK`**.
- **Full 9 Eras, 76 Technologies (`u128` Bitmask), 82 Units, 12 World Wonders & 20 Social Policies:**
  - Complete 9-Era arc (**Ancient → Classical → Medieval → Renaissance → Industrial → Modern → Atomic → Information → Future**), 4 Victory Paths, Difficulty (`Settler`–`Deity`), Game Speed (`Quick`–`Marathon`), 4 Map Types, 8 Start Eras, and a 5-tab **Rules & Codex**.

---

## Performance Benchmarks (Huge Map — 22 & 24 Historical Civs, 350 Turns)

Measured via `hexforge-bench --size huge --players 22 --turns 350 --seed 42` (4,992 tiles, reaching **115 cities** and **958 active formations = 1,442 raw unit-equivalents** by Turn 351) compared against Vanilla Unciv and [Unciv-Rust v0.1.0](https://github.com/OrionFlaxan/Unciv-Rust):

| Metric | Vanilla Unciv | Unciv-Rust v0.1.0 (JNI) | **HexForge-Rust v0.5 (22 Civs, Parallel)** | **HexForge-Rust v0.5 (22 Civs, Speculative)** | **HexForge-Rust v0.5 (All 24 Civs)** |
|---|---:|---:|---:|---:|---:|
| **Mapgen + Boot** | 3,595 ms | 2,678 ms | **9.05 ms** | **16.00 ms** | **11.32 ms** |
| **Overall 350-Turn Mean** | 6,146 ms *(1–325)* | 1,570 ms | **6.34 ms** | **3.26 ms** *(95.0% hit)* | **7.14 ms** |
| **Turns 301–350 Mean** | 34,492 ms *(301–325)* | 3,072 ms | **8.99 ms** | **6.39 ms** | **9.60 ms** |
| **Total 350 Turns Wall-Clock** | > 33.3 min *(timeout @ 325)* | 9.16 min | **2.24 seconds** | **2.80 seconds** | **2.52 seconds** |
| **Turn-351 Save / Load** | 32.8 ms / 13.2 ms | 12.0 ms / 4.8 ms | **3.54 ms / 2.66 ms** *(128 KB)* | **7.44 ms** *(124 KB)* | **2.56 ms / 2.72 ms** *(132 KB)* |
| **Universal 4-ABI APK Size** | ~30 MB | 32.3 MB | **~8.2 MB** | **~8.2 MB** | **~8.2 MB** |

### Large Map — 16 Historical Civs, 350 Turns (3,344 Tiles, 61 Cities, 572 Formations)
- **Full 350 Turns Total:** **1.40 seconds** (`3.97 ms/turn` mean; `4.76 ms/turn` in Turns 301–350; `2.36 ms` save / `1.47 ms` load).
- **With Background Speculative AI:** **`1.70 ms/turn`** overall (`3.08 ms/turn` in Turns 301–350, 92.8% hit rate).
- **1-Thread vs. Multi-Thread Determinism:** 100% bit-for-bit identical `state_hash` (`0x9abb8dee13d2601a`) after 350 full turns!

---

## The 4 Anti-Troop-Lag Pillars

1. **Zero-GC Flat SoA Simulation + Deterministic 2-Phase Parallel Rayon AI (`crates/sim`):** `#![forbid(unsafe_code)]`, `u128`/`u64`/`u32` bitmasks for Techs, Buildings/Wonders/UBs, Vision, and Social Policies, and thread-local generation-stamped scratch arenas.
2. **Corps & Army Formation Stacking + O(V) Theater Flow-Fields:** Merge identical units into **Corps (`★★`, `+10 CS`, 1 Supply)** and **Armies (`★★★`, `+17 CS`, 1 Supply)**; AI navigates hundreds of formations using O(1) lookups into Dial-bucket Dijkstra flow-fields.
3. **Background Speculative AI Pre-Computation:** Pre-computes rival AI plans on a background thread while the human takes their turn — untouched empires take **0 ms** to plan when you tap **END TURN**.
4. **Progressive Supply Cap & Elite Economy:** `SupplyCap = 5 + 2*Cities + TotalPop/2 + MilitaryBuildings + Traits/Policies` — rewards elite Corps & Armies over late-game unit spam.

---

## Building from Source

```bash
# Run all 13 simulation, historical civ, mandatory turn blocker, worker, wonder & determinism tests
cargo test --workspace --release

# Run 350-turn Huge (22 & 24-civ) benchmarks
cargo run -p hexforge-bench --release -- --size huge --players 24 --turns 350 --seed 42

# Launch native Desktop client
cargo run -p hexforge-app --release

# Build universal 4-ABI Android APK (requires Android SDK/NDK + Rust Android targets)
bash scripts/build-android.sh
./android/gradlew -p android assembleDebug -PskipRustBuild=true
```
