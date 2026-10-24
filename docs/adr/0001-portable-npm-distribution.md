---
status: accepted
---

# 以 npm 配送 Windows amd64 portable payload

公司電腦不能使用 installer，但公司 npm Proxy 可取得自製套件，因此產品只發 Windows amd64 的預編譯 portable payload，將必要 App、DLL、runtime 與模型納入明確的套件／manifest，讓使用者透過 npm 取得；Python wheel 與 setup／MSI 不是交付方式。通用產品包以公開 npm 為發布方向，公司 endpoint、model ID 與憑證保存在本機，GitHub 研究／程式碼 repo 維持 private。

安裝後的執行不應臨時要求 bootstrapper 或 native build tools；持久資料須與 npm 管理的版本化程式檔分開，以保留模型及文字歷史。實際 payload、registry 權限、第三方 notices 與公司 Windows 冷啟動仍須驗證；本決策不代表已建置或發布產品。

依據：[Q9、Q12 與部署限制](../research/08-decision-log.md)，以及 [package 研究](../research/05-performance-and-package-delivery.md)。
