---
status: accepted
---

# 共用語音核心與平台 adapter，先交付 Windows

使用者於 2026-10-25 要求未來可支援 macOS、Linux，因此現在將語音 session、取消／重試、文字整理與文字歷史規則放入共用核心 module，將熱鍵、錄音、原輸入目標、受保護交付、權限、資料路徑及 native payload 組裝留在平台 adapter。先交付 Windows amd64，優先沿用 Handy 的 Rust／Tauri 2／React TypeScript 路線；以小型語義 interface 隱藏 OS 差異，並用共用核心 contract tests 與測試 adapter 驗證，接受前期拆分成本以降低日後重寫核心的代價。

## Consequences

共用核心不依賴 Windows API、native handles 或固定環境變數；組裝處選擇 adapter，保存 module 接受解析後的資料位置，因此現有 Node prototype 的 LOCALAPPDATA 預設必須在正式整合時移出共用保存邏輯。平台 adapter 對不支援、缺少權限或無法確認原輸入目標的情況必須明確回報並保留待選結果，不能以推測的前景視窗交付。

本決策延伸 [npm 配送 ADR](0001-portable-npm-distribution.md) 的平台 payload 組裝方式，保留 [文字歷史 ADR](0002-text-history-independent-of-audio.md) 的保存語義；本期仍只交付 Windows amd64，macOS／Linux 的 CPU、最低 OS、發行細節及 runtime 結果待各平台驗證。現有 Node／whisper.cpp 診斷流程是行為 prototype，不構成最終 App runtime 或模型定稿。
