# Handy 中文使用經驗：公開回饋查核

查核日期：2026-11-01。這是可找到的第一手使用者分享與問題回報，屬於自行選擇發聲的小樣本；不能推算中文使用者的整體滿意度或平均辨識率。作者、日期、模型、版本及硬體可能不同，不應將主觀比較視為控制實驗。

| 回饋來源 | 實際經驗與限制 |
| --- | --- |
| [林承慶，2026-02-19](https://vocus.cc/article/6996fe3dfd8978000192eadc) | 作者使用 Whisper Medium，認為中文轉錄不錯且免費；其體驗是標點不足、偶有簡體、句內中英日切換容易錯，講完等候約 2–5 秒。作者未提供可重現的硬體及完整測試集；這是當時版本經驗。 |
| [同一作者後續文章，2026-03-22](https://vocus.cc/article/69bf7f2efd89780001224f68) | 作者後來表示不再使用 Handy，但未在該段提供具體測試結果。不能把兩篇當成兩位使用者。 |
| [WFU BLOG，2026-10，Handy v0.9.8](https://tw.wfublog.com/2026/10/ai.html) | 作者稱 Breeze ASR 25 繁中日常用語表現佳，但無後處理時仍須花時間補標點、分段；接本機 LLM 需另外架設，對新手較不方便。對台灣用詞及簡繁轉換有具體錯字例子；單人評測並未提供完整基準資料。 |
| [Blank，2026-10-18](https://linyafeng.com/archives/2260/) | 作者以同一句話比較 SenseVoice Small、Fun-ASR Nano、Nemotron Streaming 3.5、Whisper Small，稱 SenseVoice 在該例表現最好，另舉 3 段中文與含 PRD 的短句；一個比較句及 3 個示例不足以推斷普遍排名。作者稱 CPU 快速輸出但未提供完整硬體與量測方法。 |
| [GitHub issue #2239，2026-10-08](https://github.com/cjpais/Handy/issues/2239) | Windows 11、Handy v0.9.8、Whisper Large v3 Turbo Q8_0、Auto 模式下，作者一段中英混合錄音被整段判成英文，貼出文字仍有簡體；同一錄音換 Qwen3-ASR 0.6B 與 Breeze ASR 25 時，作者回報辨識為中文並轉成繁體。這是一筆具體重現，不是通用模型排名；截至查核時 issue 開啟。 |
| [GitHub discussion #1574，2026-07-02](https://github.com/cjpais/Handy/discussions/1574) | 使用者抱怨推薦模型 Parakeet V3 無法辨識中文，並稱自己試用的 Whisper 模型中文效果差。維護者回覆 Handy 已支援 Qwen3-ASR；這不是實測評比。 |
| [小眾軟體論壇，2026-02-05](https://meta.appinn.net/t/topic/81289) | 一位使用者說中文卻收到英文音譯；另一位建議在設定選中文。原文沒有後續確認該操作是否解決問題。 |
| [DormanVoice，2025-11-07](https://dormanthink.wordpress.com/2025/11/07/2025-11-07-handyollamacherrystudio%EF%BC%8C%E6%9C%AC%E5%9C%B0%E8%AF%AD%E9%9F%B3%E8%BE%93%E5%85%A5%E7%BB%84%E5%90%88/) | 作者覺得 Whisper Medium 的中文比當時 Windows 內建輸入準，但需要說完才出字，並搭配另一工具整理標點。這是當時的個人體驗。 |
| [SpeakEasy 開發者，2026-02-26](https://davidai311.vercel.app/blog/speakeasy-dev-log/) | 作者回報 Whisper 中英混講問題，於自行修改的 fork 用 SenseVoice Auto 加 OpenCC；固定 Chinese 反而影響英文。這是其 fork 的測試，不能直接當成原版 Handy 效果或每種模型的通則。 |

使用技巧的證據邊界：繁中、簡中、中英混合要分開實測；切換自動語言偵測和中文固定模式可能產生不同取捨，受模型及版本影響。模型單次結果、個人速度與相互矛盾的心得均應保留原文條件。語音本機處理不表示啟用雲端文字後處理時，轉錄文字也不會送到外部服務。

## 官方資料與實用設定

- [Handy 模型基準](https://models.handy.computer/languages/zh/)：FLEURS 中文測試集、Q8_0 條件下，Qwen3-ASR 1.7B 的 CER 為 7.14%、0.6B 為 7.57%、SenseVoice Small 為 10.12%、Whisper Small 為 23.06%；速度列的是 AMD Ryzen 4750U 上的推論倍速。這不是台灣中英混合或端到端延遲成績，不可直接推論 notype 的 Q6_K 版本。
- [官方模型文件](https://handy.computer/docs/models)描述 Breeze ASR 針對台灣華語與語碼切換設計；實際支援與模型檔大小應以安裝版本為準。
- [進階設定](https://handy.computer/docs/advanced)說明自訂詞彙使用近似詞修正且不完美；[Debug 文件](https://handy.computer/docs/debug)說明調高閾值更積極，但可能誤改。中文詞彙歷史問題 [#1705](https://github.com/cjpais/Handy/issues/1705) 已關閉，無法僅憑此狀態確定受影響的修正版本。
- [文字後處理](https://handy.computer/docs/post-processing)可設定自己的提示詞，透過 `${output}` 注入原始轉錄；雲端端點會收到辨識文字。官方記載一般轉錄與啟用後處理分屬不同快捷鍵。
- [貼上設定](https://handy.computer/docs/paste-methods)依目標 App 提供不同方式；[Debug 文件](https://handy.computer/docs/debug)建議貼上舊剪貼簿內容時，將預設 60 ms 調至 100–200 ms。這些為 Handy 設定，不能直接推論 notype 介面有相同選項。
