import { readFile } from "node:fs/promises";
import { activateModel } from "../../../packages/notype/lib/model.mjs";

const plan = JSON.parse(await readFile(process.argv[2], "utf8"));
await activateModel(plan, {
  onProgress() {
    process.kill(process.pid, "SIGKILL");
  },
});
