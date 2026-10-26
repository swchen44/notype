---
status: accepted
---

# 以 npm 或完整離線 ZIP 配送 Windows amd64 portable payload

公司電腦不能使用 installer，因此產品配送 Windows amd64 的預編譯 portable payload。保留 npm／公司 npm Proxy 取得方式，另提供包含 App、完整 runtime、已驗證語音模型及可選非機密預設連線的離線 ZIP，供網路受限環境解壓啟動；不要求收件者安裝 Node、開發工具或 runtime bootstrapper。通用產品包不含公司私有 endpoint、model ID 或憑證；管理者可在私有離線包另附無 key 的內網預設，使用者於 App 填入自己的 key。GitHub repo 維持 private。

基本 App 與模型取得分開：沒有模型也能啟動設定介面，模型 add-on 由使用者明確下載或指定既有資料夾。使用者可自行搬移整個模型資料夾，在重新安裝後指定新位置；App 選取與重新掃描不搬移、複製或刪除既有模型。個人設定與歷史放在程式及模型資料夾之外，已有設定優先於離線包預設。完整 ZIP 的整包驗證仍須包含所有宣告模型；一般啟動時模型缺失則提供選取資料夾的復原介面。

2026-10-26 使用者新增模型重用與內網 ZIP 場景，修訂原 npm-only／強制取得模型的限制，避免大模型重複下載。這是配送與資料邊界決策，尚不代表新版來源、原生啟動、整包組裝、乾淨環境或內網服務驗收通過；實際 payload、registry 權限、第三方 notices 仍須分別驗證。

依據：[決策紀錄](../research/08-decision-log.md)、[package 研究](../research/05-performance-and-package-delivery.md)、[#28 計畫](https://github.com/swchen44/notype/issues/28#issuecomment-6405356803)／[GPT-6-Astra 核准](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881)，以及 [#29 計畫](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391)／[核准](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255)。
