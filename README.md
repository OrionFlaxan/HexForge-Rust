# HexForge-Rust — 100% Pure-Rust 4X Strategy Game

[![Latest Release](https://img.shields.io/github/v/release/OrionFlaxan/HexForge-Rust?include_prereleases&label=Download%20Release&logo=github&color=2ea44f)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest)
[![Android 4-ABI APK](https://img.shields.io/badge/Android-arm64--v8a%20%7C%20armeabi--v7a%20%7C%20x86__64%20%7C%20x86-3DDC84?logo=android&logoColor=white)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk)
[![Pure Rust](https://img.shields.io/badge/Engine-100%25%20Rust%20%2B%20Rayon%20%2B%20GLES3-DEA584?logo=rust&logoColor=white)](#the-4-anti-troop-lag-pillars)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**HexForge-Rust** is a brand-new, **100% Pure-Rust 4X strategy game** for **Android (all 4 ABIs, API 21+, Portrait & Landscape)** and **Desktop** — engineered from scratch with **zero JVM, zero garbage collector, multi-threaded Rayon AI, Corps/Army stacking, O(V) Theater Flow-Fields, and Background Speculative AI** so **Huge 22-civ maps past Turn 300+ run in milliseconds on any phone**.

---

## Download (v0.3.0)

| Platform | Direct Download | Size | Notes |
|---|---|---:|---|
| **Android (Phone / Tablet)** | [**Download `HexForge-Rust-Android.apk`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk) | **~7.6 MB** | Universal APK (Android 5.0+, Portrait & Landscape, true DPI scaling) with all 4 ABIs (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`). Installs **side-by-side** alongside Unciv! |
| **Linux Desktop (x86_64)** | [**Download `HexForge-Rust-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Linux-x86_64) | **~1.9 MB** | Run `chmod +x HexForge-Rust-Linux-x86_64 && ./HexForge-Rust-Linux-x86_64` |
| **Headless Benchmark CLI** | [**Download `HexForge-Bench-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Bench-Linux-x86_64) | **~1.2 MB** | Run 350-turn Huge/Large benchmarks on any machine |
| **All Releases** | [**GitHub Releases**](https://github.com/OrionFlaxan/HexForge-Rust/releases) | — | Release notes, checksums & artifacts |

---

## What's Inside (v0.3.0)

- **Mobile-First High-DPI UI (Portrait & Landscape):** Automatically reads Android's `DisplayMetrics.density` for large, thumb-friendly buttons on any phone or tablet, plus a **live UI Scale (`75%–150%`)** control and display toggles saved to `options.toml`.
- **Custom Game Rules & 22-Civ Picker:** Configure **Difficulty** (`Settler`–`Deity`), **Game Speed** (`Quick`–`Marathon`), **Map Type** (`Continents`, `Pangaea`, `Archipelago`, `Fractal`), **Start Era** (`Ancient`–`Industrial`), **Victory Conditions** (`Domination`, `Science`, `Culture`, `Score`), **Corps/Armies**, **Fog of War**, and choose **any of the 22 Civilizations** — each with a unique Empire Trait.
- **5-Tab In-Game Rules & Codex (Civilopedia):** Browse *How to Play*, *Combat/Corps/Supply Rules*, all *32 Units*, *50 Techs & 10 Unique World Wonders*, and *22 Civs & 20 Social Policies* right from the Title Screen or In-Game.
- **6 Full Eras & Deep 4X Content (`assets/ruleset/default.toml`):**
  - **50 Technologies** (`Ancient → Classical → Medieval → Renaissance → Industrial → Modern & Future`), unlocking Roads, Railroads, Embarkation, Corps, Armies, Air Power, Robotics, and Orbital Colonization.
  - **32 Unit Types across 9 Roles** including Air Fighters/Bombers/Stealth Bombers, Submarines, Destroyers, Battleships, Rocket Artillery, and Giant Mechs.
  - **21 City Buildings + 10 Globally Unique World Wonders** (Great Library, Pyramids, Hanging Gardens, Colossus, Notre Dame, Machu Picchu, Taj Mahal, Big Ben, Pentagon, Orbital Elevator).
  - **5 Social Policy Trees (20 Policies):** Spend Culture across *Tradition*, *Liberty*, *Honor*, *Rationalism*, and *Industry* — adopt 12 policies (3 full trees) for a **Culture Victory**.

---

## Performance Benchmarks (Huge Map — 22 AI Civs, 350 Turns)

Measured via `hexforge-bench --size huge --players 22 --turns 350 --seed 42` (4,992 tiles, reaching **106 cities** and **927 active formations = 1,522 raw unit-equivalents** by Turn 351) compared against Vanilla Unciv and [Unciv-Rust v0.1.0](https://github.com/OrionFlaxan/Unciv-Rust):

| Metric | Vanilla Unciv | Unciv-Rust v0.1.0 (JNI) | **HexForge-Rust v0.3 (Parallel)** | **HexForge-Rust v0.3 (Speculative AI)** |
|---|---:|---:|---:|---:|
| **Mapgen + Boot** | 3,595 ms | 2,678 ms | **9.6 ms** | **10.9 ms** |
| **Overall 350-Turn Mean** | 6,146 ms *(1–325)* | 1,570 ms | **5.54 ms** | **2.51 ms** *(95.2% hit rate)* |
| **Turns 301–350 Mean** | 34,492 ms *(301–325)* | 3,072 ms | **7.24 ms** | **3.91 ms** |
| **Turns 301–350 Max Spike** | 56,442 ms *(timeout)* | 4,127 ms | **9.14 ms** | **4.70 ms** |
| **Total 350 Turns Wall-Clock** | > 33.3 min *(timeout @ 325)* | 9.16 min | **1.96 seconds** | **2.00 seconds** |
| **Turn-351 Save / Load** | 32.8 ms / 13.2 ms | 12.0 ms / 4.8 ms | **3.57 ms / 2.78 ms** *(124 KB)* | **3.44 ms / 2.55 ms** |
| **Universal 4-ABI APK Size** | ~30 MB | 32.3 MB | **7.6 MB** | **7.6 MB** |

### Large Map — 16 AI Civs, 350 Turns (3,344 Tiles, 56 Cities, 498 Formations)
- **Full 350 Turns Total:** **1.16 seconds** (`3.28 ms/turn` mean; `3.69 ms/turn` in Turns 301–350; `0.88 ms` save / `1.54 ms` load).
- **With Background Speculative AI:** **`1.38 ms/turn`** overall (`1.80 ms/turn` in Turns 301–350, 93.3% hit rate).
- **1-Thread vs. Multi-Thread Determinism:** 100% bit-for-bit identical `state_hash` (`0x5a95e03a5fed7b94`) after 350 full turns!

---

## The 4 Anti-Troop-Lag Pillars

1. **Zero-GC Flat SoA Simulation + Deterministic 2-Phase Parallel Rayon AI (`crates/sim`):** `#![forbid(unsafe_code)]`, `u64`/`u32` bitmasks for Techs, Buildings/Wonders, Vision, and Social Policies, and thread-local generation-stamped scratch arenas.
2. **Corps & Army Formation Stacking + O(V) Theater Flow-Fields:** Merge identical units into **Corps (`★★`, `+10 CS`, 1 Supply)** and **Armies (`★★★`, `+17 CS`, 1 Supply)**; AI navigates hundreds of formations using O(1) lookups into Dial-bucket Dijkstra flow-fields.
3. **Background Speculative AI Pre-Computation:** Pre-computes rival AI plans on a background thread while the human takes their turn — untouched empires take **0 ms** to plan when you tap **END TURN**.
4. **Progressive Supply Cap & Elite Economy:** `SupplyCap = 5 + 2*Cities + TotalPop/2 + MilitaryBuildings + Traits/Policies` — rewards elite Corps & Armies over late-game unit spam.

---

## Building from Source

```bash
# Run all 10 simulation, policy, wonder & determinism tests
cargo test --workspace --release

# Run 350-turn Huge (22-civ) benchmark
cargo run -p hexforge-bench --release -- --size huge --players 22 --turns 350 --seed 42

# Launch native Desktop client
cargo run -p hexforge-app --release

# Build universal 4-ABI Android APK (requires Android SDK/NDK + Rust Android targets)
bash scripts/build-android.sh
./android/gradlew -p android assembleDebug -PskipRustBuild=true
```
