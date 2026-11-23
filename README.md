# HexForge-Rust — 100% Pure-Rust 4X Strategy Game

[![Latest Release](https://img.shields.io/github/v/release/OrionFlaxan/HexForge-Rust?include_prereleases&label=Download%20Release&logo=github&color=2ea44f)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest)
[![Android 4-ABI APK](https://img.shields.io/badge/Android-arm64--v8a%20%7C%20armeabi--v7a%20%7C%20x86__64%20%7C%20x86-3DDC84?logo=android&logoColor=white)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk)
[![Pure Rust](https://img.shields.io/badge/Engine-100%25%20Rust%20%2B%20Rayon%20%2B%20GLES3-DEA584?logo=rust&logoColor=white)](#the-4-anti-troop-lag-pillars)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**HexForge-Rust** is a brand-new, **100% Pure-Rust 4X strategy game** for **Android (all 4 ABIs, API 21+, Portrait & Landscape)** and **Desktop** — engineered from scratch with **zero JVM, zero garbage collector, multi-threaded Rayon AI, Corps/Army stacking, O(V) Theater Flow-Fields, and Background Speculative AI** so **Huge 22-civ maps past Turn 300+ run in milliseconds on any phone**.

---

## Download (v0.4.0)

| Platform | Direct Download | Size | Notes |
|---|---|---:|---|
| **Android (Phone / Tablet)** | [**Download `HexForge-Rust-Android.apk`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk) | **~7.7 MB** | Universal APK (Android 5.0+, Portrait & Landscape, true DPI scaling) with all 4 ABIs (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`). Installs **side-by-side** alongside Unciv! |
| **Linux Desktop (x86_64)** | [**Download `HexForge-Rust-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Linux-x86_64) | **~2.0 MB** | Run `chmod +x HexForge-Rust-Linux-x86_64 && ./HexForge-Rust-Linux-x86_64` |
| **Headless Benchmark CLI** | [**Download `HexForge-Bench-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Bench-Linux-x86_64) | **~1.3 MB** | Run 350-turn Huge/Large benchmarks on any machine |
| **All Releases** | [**GitHub Releases**](https://github.com/OrionFlaxan/HexForge-Rust/releases) | — | Release notes, checksums & artifacts |

---

## What's Inside (v0.4.0)

- **Separated City Production Tabs (`UNITS` | `BLDGS` | `WONDERS` | `BUILT`):** Clean, thumb-friendly tabs with live item counts, concise yield deltas, and one-tap gold rush-buy — never mixes units and buildings into one list!
- **Realistic Multi-Choice Worker System (`IMPROVE...` Sheet + `AUTO WORK`):**
  - **16 Distinct Tile Improvements:** *Farm, Mine, Pasture, Quarry, Plantation, Camp, Lumber Mill, Terrace Farm, Trading Post, Fort, Oil Well, Power Plant, Solar Farm, Research Outpost, Forward Airbase*, and *Arcology Dome* — with **tech-scaled yields** (*Fertilizer*, *Chemistry*, *Economics*, *Scientific Theory*, *Plastics*, *Nanotechnology*).
  - **Terrain Engineering & Warfare:** Multi-turn builds (`1–4t`, sped up by *Great Pyramids* & *Citizenship*), **Chop Forest (`+20 Prod`)**, **Clear Jungle**, **Drain Marsh (`→ Grassland`)**, **Roads & Railroads (`+15% City Prod`)**, **Forts & Forward Airbases (`+50% Def` + Air Rebase)**, and **Combat Pillaging (`+20G / +20HP`) & Repair**.
- **Full 9 Eras, 76 Technologies (`u128` Bitmask) & 58 Authentic Units (`assets/ruleset/default.toml`):**
  - Complete 9-Era progression (**Ancient → Classical → Medieval → Renaissance → Industrial → Modern → Atomic → Information → Future**) with a 2-row mobile Era Switcher.
  - **58 Units across 9 Roles** including *Chariot Archer, Longswordsman, Great War Infantry, Marine, Gatling Gun, Landship, Anti-Tank Gun, Anti-Aircraft Gun, Mobile SAM, Ironclad, Helicopter Gunship, Nuclear Submarine, Missile Cruiser, Guided Missile, Hover Tank, Orbital Strike Drone*, and *Giant Death Robot*.
  - **26 City Buildings + 12 Globally Unique World Wonders** & **5 Social Policy Trees (20 Policies, Culture Victory at 12)**.
- **Mobile-First High-DPI UI (Portrait & Landscape), Custom Rules, Options & 5-Tab Codex:** Live `75%–150%` UI Scale (`options.toml`), Difficulty (`Settler`–`Deity`), Game Speed (`Quick`–`Marathon`), 4 Map Types, 8 Start Eras, and 22 Civilizations with unique Empire Traits.

---

## Performance Benchmarks (Huge Map — 22 AI Civs, 350 Turns)

Measured via `hexforge-bench --size huge --players 22 --turns 350 --seed 42` (4,992 tiles, reaching **113 cities** and **951 active formations = 1,498 raw unit-equivalents** by Turn 351) compared against Vanilla Unciv and [Unciv-Rust v0.1.0](https://github.com/OrionFlaxan/Unciv-Rust):

| Metric | Vanilla Unciv | Unciv-Rust v0.1.0 (JNI) | **HexForge-Rust v0.4 (Parallel)** | **HexForge-Rust v0.4 (Speculative AI)** |
|---|---:|---:|---:|---:|
| **Mapgen + Boot** | 3,595 ms | 2,678 ms | **11.1 ms** | **9.0 ms** |
| **Overall 350-Turn Mean** | 6,146 ms *(1–325)* | 1,570 ms | **6.89 ms** | **2.86 ms** *(95.2% hit rate)* |
| **Turns 301–350 Mean** | 34,492 ms *(301–325)* | 3,072 ms | **9.42 ms** | **5.42 ms** |
| **Turns 301–350 Max Spike** | 56,442 ms *(timeout)* | 4,127 ms | **11.80 ms** | **10.40 ms** |
| **Total 350 Turns Wall-Clock** | > 33.3 min *(timeout @ 325)* | 9.16 min | **2.44 seconds** | **2.38 seconds** |
| **Turn-351 Save / Load** | 32.8 ms / 13.2 ms | 12.0 ms / 4.8 ms | **3.97 ms / 2.63 ms** *(126 KB)* | **1.49 ms / 3.75 ms** |
| **Universal 4-ABI APK Size** | ~30 MB | 32.3 MB | **~7.7 MB** | **~7.7 MB** |

### Large Map — 16 AI Civs, 350 Turns (3,344 Tiles, 62 Cities, 560 Formations)
- **Full 350 Turns Total:** **1.27 seconds** (`3.59 ms/turn` mean; `4.30 ms/turn` in Turns 301–350; `2.92 ms` save / `1.64 ms` load).
- **With Background Speculative AI:** **`1.44 ms/turn`** overall (`1.96 ms/turn` in Turns 301–350, 93.2% hit rate).
- **1-Thread vs. Multi-Thread Determinism:** 100% bit-for-bit identical `state_hash` (`0xc3bd7debbe7a6b36`) after 350 full turns!

---

## The 4 Anti-Troop-Lag Pillars

1. **Zero-GC Flat SoA Simulation + Deterministic 2-Phase Parallel Rayon AI (`crates/sim`):** `#![forbid(unsafe_code)]`, `u128`/`u64`/`u32` bitmasks for Techs, Buildings/Wonders, Vision, and Social Policies, and thread-local generation-stamped scratch arenas.
2. **Corps & Army Formation Stacking + O(V) Theater Flow-Fields:** Merge identical units into **Corps (`★★`, `+10 CS`, 1 Supply)** and **Armies (`★★★`, `+17 CS`, 1 Supply)**; AI navigates hundreds of formations using O(1) lookups into Dial-bucket Dijkstra flow-fields.
3. **Background Speculative AI Pre-Computation:** Pre-computes rival AI plans on a background thread while the human takes their turn — untouched empires take **0 ms** to plan when you tap **END TURN**.
4. **Progressive Supply Cap & Elite Economy:** `SupplyCap = 5 + 2*Cities + TotalPop/2 + MilitaryBuildings + Traits/Policies` — rewards elite Corps & Armies over late-game unit spam.

---

## Building from Source

```bash
# Run all 11 simulation, worker, policy, wonder & determinism tests
cargo test --workspace --release

# Run 350-turn Huge (22-civ) benchmark
cargo run -p hexforge-bench --release -- --size huge --players 22 --turns 350 --seed 42

# Launch native Desktop client
cargo run -p hexforge-app --release

# Build universal 4-ABI Android APK (requires Android SDK/NDK + Rust Android targets)
bash scripts/build-android.sh
./android/gradlew -p android assembleDebug -PskipRustBuild=true
```
