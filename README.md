# HexForge-Rust — 100% Pure-Rust 4X Strategy Game

[![Latest Release](https://img.shields.io/github/v/release/OrionFlaxan/HexForge-Rust?include_prereleases&label=Download%20Release&logo=github&color=2ea44f)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest)
[![Android 4-ABI APK](https://img.shields.io/badge/Android-arm64--v8a%20%7C%20armeabi--v7a%20%7C%20x86__64%20%7C%20x86-3DDC84?logo=android&logoColor=white)](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk)
[![Pure Rust](https://img.shields.io/badge/Engine-100%25%20Rust%20%2B%20Rayon%20%2B%20GLES3-DEA584?logo=rust&logoColor=white)](#the-4-anti-troop-lag-pillars)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**HexForge-Rust** is a brand-new, **100% Pure-Rust 4X strategy game** for **Android (all 4 ABIs, API 21+)** and **Desktop** — engineered from scratch with **zero JVM, zero garbage collector, multi-threaded Rayon AI, Army/Corps stacking, O(V) Theater Flow-Fields, and Background Speculative AI** so **Huge 22-civ maps past Turn 300+ run in milliseconds on any phone**.

---

## Download

| Platform | Direct Download | Size | Notes |
|---|---|---:|---|
| **Android (Phone / Tablet)** | [**Download `HexForge-Rust-Android.apk`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Android.apk) | **~6.8 MB** | Universal APK (Android 5.0+) with all 4 ABIs (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`). Installs **side-by-side** alongside Unciv! |
| **Linux Desktop (x86_64)** | [**Download `HexForge-Rust-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Rust-Linux-x86_64) | **~1.7 MB** | Run `chmod +x HexForge-Rust-Linux-x86_64 && ./HexForge-Rust-Linux-x86_64` |
| **Headless Benchmark CLI** | [**Download `HexForge-Bench-Linux-x86_64`**](https://github.com/OrionFlaxan/HexForge-Rust/releases/latest/download/HexForge-Bench-Linux-x86_64) | **~1.2 MB** | Run 350-turn Huge/Large benchmarks on any machine |
| **All Releases** | [**GitHub Releases**](https://github.com/OrionFlaxan/HexForge-Rust/releases) | — | Release notes, checksums & artifacts |

---

## Performance Benchmarks (Huge Map — 22 AI Civs, 350 Turns)

Measured via `hexforge-bench --size huge --players 22 --turns 350 --seed 42` (4,992 hex tiles, reaching **112 cities** and **774 active formations = 1,483 raw unit-equivalents** by Turn 350) compared against our earlier benchmarks of Vanilla Unciv and [Unciv-Rust v0.1.0](https://github.com/OrionFlaxan/Unciv-Rust):

| Metric | Vanilla Unciv | Unciv-Rust v0.1.0 (JNI) | **HexForge-Rust (Parallel)** | **HexForge-Rust (Speculative AI)** |
|---|---:|---:|---:|---:|
| **Mapgen + Boot** | 3,595 ms | 2,678 ms | **9–11 ms** | **9 ms** |
| **Turns 301–350 Mean** | 34,492 ms *(301–325)* | 3,072 ms | **3.97 ms** | **1.77 ms** *(95.2% hit rate)* |
| **Turns 301–350 Max Spike** | 56,442 ms *(timeout)* | 4,127 ms | **5.44 ms** | **3.04 ms** |
| **Total 350 Turns Wall-Clock** | > 33.3 min *(timeout @ 325)* | 9.16 min | **1.48 seconds** | **1.67 seconds** |
| **Turn-351 Save / Load** | 32.8 ms / 13.2 ms | 12.0 ms / 4.8 ms | **3.54 ms / 2.87 ms** *(119 KB)* | **3.68 ms / 2.73 ms** |
| **Universal 4-ABI APK Size** | ~30 MB | 32.3 MB | **6.8 MB** | **6.8 MB** |

### Large Map — 16 AI Civs, 350 Turns (3,344 Tiles, 57 Cities, 452 Formations)
- **Full 350 Turns Total:** **0.97 seconds** (`2.74 ms/turn` mean; `3.16 ms/turn` in Turns 301–350)
- **With Background Speculative AI:** **`1.24 ms/turn`** in Turns 301–350 (93.3% hit rate)
- **1-Thread vs. Multi-Thread Determinism:** 100% bit-for-bit identical `state_hash` (`0x78985e3cbe5c132f`) after 350 full turns!

---

## The 4 Anti-Troop-Lag Pillars

1. **Zero-GC Flat SoA Simulation + Deterministic 2-Phase Parallel Rayon AI (`crates/sim`):**
   - `#![forbid(unsafe_code)]` data-oriented world (`HexMapSoA`, `UnitSoA`, `CitySoA`, `DiploMatrix`, `u64` bitset `VisionState` + tech/building bitmasks) — zero string hashing and zero heap allocations per query via thread-local generation-stamped scratch arenas.
   - **Phase 1 (`rayon::par_iter`)** computes every AI civ's Threat Fields, Theater Flow-Fields, city production, and diplomacy in parallel across all phone cores; **Phase 2** executes plans deterministically in `CivId` order.
2. **Corps & Army Formation Stacking + O(V) Theater Flow-Fields (`ai/theater.rs`, `combat.rs`):**
   - Unlock **Military Tactics** to merge 2 identical units into an elite **Corps** (`★★`, `+10 CS`, `1.25×` durability, **1 supply slot**), and **Mobilization** to merge into a full **Army** (`★★★`, `+17 CS`, `1.5×` durability, **1 supply slot**).
   - Instead of running A\* for hundreds of troops individually, each AI civ runs **one O(V + E) Dial-bucket Dijkstra flow-field per theater** — every unit reads its optimal move in **O(1)**.
3. **Background Speculative AI Pre-Computation (`ai/speculative.rs`):**
   - While you inspect the map and command your units, a background worker pre-computes Phase 1 AI plans for all rival empires.
   - When you tap **END TURN**, untouched empires reuse their cached plans in **0 ms**; only bordering empires affected by your moves replan.
4. **Progressive Supply Cap & Elite Economy (`economy.rs`):**
   - `SupplyCap = 5 + 2*Cities + TotalPop/2 + MilitaryBuildings` (Corps & Armies still use only 1 Supply), rewarding both players and AI for forging elite formations over 1,000-unit carpet spam.

---

## Controls & Gameplay Features

- **4-Era Ruleset (`assets/ruleset/default.toml`):** 26 Technologies, 20 Unit types across 8 roles, 13 City Buildings (including *Orbital Launch Complex* Science Victory), 8 Resources, and 22 distinct Civilizations.
- **Touch Controls (Android):** 1-finger drag to pan, 2-finger pinch to zoom, tap unit to see cyan move / red attack ranges, tap **MERGE CORPS / ARMY** when identical units are adjacent, tap city twice (or **MANAGE CITY**) to produce or gold-buy, tap minimap to jump anywhere.
- **Desktop Shortcuts:** Drag / WASD / Arrow keys to pan, Mouse Wheel / `+`/`-` to zoom, `Tab` for Next Ready Unit, `Space`/`Enter` for End Turn, `Esc` for Menu.
- **Built-In Stress Benchmark Tools (in Menu & Spectator Mode):** Tap **SIM +10 / +50 / +100 TURNS** or launch **AI Spectator Mode** directly on your phone to watch Turn 350+ run live with the real-time **Rust ms/turn + FPS** telemetry badge!

---

## Building from Source

```bash
# Run all simulation & determinism unit tests
cargo test --workspace --release

# Run 350-turn Huge (22-civ) benchmark
cargo run -p hexforge-bench --release -- --size huge --players 22 --turns 350 --seed 42

# Launch native Desktop client
cargo run -p hexforge-app --release

# Build universal 4-ABI Android APK (requires Android SDK/NDK + Rust Android targets)
bash scripts/build-android.sh
./android/gradlew -p android assembleDebug -PskipRustBuild=true
```
