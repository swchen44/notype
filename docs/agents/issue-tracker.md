# Issue tracker: GitHub

Repository：`swchen44/notype_study`。工作與規格記錄在 GitHub Issues，使用 `gh` CLI。

- 建立：`gh issue create --repo swchen44/notype_study --title "..." --body-file <file>`。
- 讀取：`gh issue view <number> --repo swchen44/notype_study --json number,title,body,labels,comments`。
- 列出：`gh issue list --repo swchen44/notype_study --state open --json number,title,body,labels`，按需加 label 篩選。
- 留言、標籤與結案使用 `gh issue comment`、`gh issue edit`、`gh issue close`，均明確指定 `--repo`；多行內容使用 `--body-file`。
- 新工作使用 `needs-triage`；其他狀態依 `triage-labels.md` 的對應關係設定。
- 工作成果以 issue 連回 commit 與驗證 receipt；來源研究、build 與 Windows runtime 分開記錄。

PRs as a request surface: no.
