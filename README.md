# RustStrap

<p align="center">
  <b>A modern Roblox bootstrapper rewritten in Rust</b><br/>
  Built with Tauri + React + TypeScript for performance and native integration
</p>

<p align="center">
  <img src="https://skillicons.dev/icons?i=rust,tauri,react,ts,nodejs" />
</p>

---

## Overview

**RustStrap** is a high-performance fork of Fishstrap, fully rewritten in **Rust** for speed and memory safety, with a modern desktop interface powered by **Tauri (React/TypeScript)**.

What began as an experiment to integrate exploit functionality via the weao API has evolved into a **feature-complete, near 1:1 rewrite** of the original C# WPF application.

---

## Features

- Full parity with Fishstrap and Bloxstrap
- Native desktop performance via Tauri
- Memory-safe backend powered by Rust
- Modern UI built with React + TypeScript
- Modular and extensible architecture

---

## Tech Stack

| Layer      | Technology |
|------------|-----------|
| Backend    | Rust      |
| Frontend   | React     |
| Framework  | Tauri     |
| Language   | TypeScript|

---

## Getting Started

### Prerequisites

- Node.js → https://nodejs.org/
- Rust → https://rustup.rs/
- MSVC C++ Build Tools (Windows)

---

## Build from Source

```bash
git clone https://github.com/RoRvzzz/RustStrap.git
cd RustStrap/app-tauri
npm install
npm run tauri build
