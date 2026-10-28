const SAFE_VERSION = /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

/** Validate the public release input before any writes to GitHub. */
export function validateReleaseInput({ version, ref, source, main, repository }) {
  const match = typeof version === 'string' && version.match(SAFE_VERSION);
  if (!match || version.length > 64 || (match[1] && match[1].split('.').some(part => /^0\d+$/.test(part))) ||
      ref !== 'refs/heads/main' || repository !== 'swchen44/notype' ||
      !/^[a-f0-9]{40}$/.test(source) || source !== main) throw new Error('invalid-release-input');
  return { version, tag: `v${version}`, source };
}

import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';

async function digestFile(path) {
  const stat = await lstat(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size <= 0) throw Error('invalid-asset');
  const digest = createHash('sha256');
  for await (const part of createReadStream(path)) digest.update(part);
  return {path, size:stat.size, digest:digest.digest('hex')};
}

/** Validates real assembled files, then creates only an explicitly incomplete draft.
 * github is an adapter; failures after creation leave the draft visibly incomplete. */
export async function publishDraft({ input, artifactRoot, github, wait = ms => new Promise(resolve => setTimeout(resolve, ms)) }) {
  const main = await github.mainHead();
  const release = validateReleaseInput({...input, main});
  if (await github.exists(release.tag)) throw Error('release-or-tag-exists');
  const assetsDir = join(artifactRoot, 'assets');
  const names = [`notype-${release.version}-offline-win-x64.zip`, 'Windows11-README.md', 'VERIFICATION.md', 'SHA256SUMS.txt'];
  const present = await readdir(assetsDir);
  if (present.length !== names.length || names.some(n => !present.includes(n))) throw Error('unexpected-release-assets');
  const assets = [];
  for (const name of names) assets.push({name, ...await digestFile(join(assetsDir,name))});
  const report = JSON.parse(await readFile(join(artifactRoot, 'assembly-report.json'), 'utf8'));
  const requiredChecks = ['zip-crc-sha','fresh-extraction','bundled-cli','native-headless-list-models'];
  if (report.source !== release.source || report.version !== release.version ||
      !Array.isArray(report.checks) || requiredChecks.some(check => !report.checks.includes(check)) ||
      report.archive?.file !== names[0] || report.archive?.bytes !== assets[0].size ||
      report.archive?.sha256 !== assets[0].digest) throw Error('assembly-report-mismatch');
  const sums = await readFile(assets[3].path, 'utf8');
  const expectedSums = assets.slice(0,3).map(a => `${a.digest}  ${a.name}\n`).join('');
  if (sums !== expectedSums) throw Error('asset-checksum-mismatch');
  // Main can move during a long build. Do not create a draft from a stale source.
  if (await github.mainHead() !== release.source || await github.exists(release.tag)) throw Error('release-gate-changed');
  const incomplete = `INCOMPLETE — upload or verification has not finished. Do not publish.\n\nSource: ${release.source}\n`;
  const id = await github.createDraft({...release, body:incomplete});
  for (const asset of assets) await github.upload(id, asset);
  let verified = false;
  for (let attempt = 0; attempt < 5; attempt++) {
    const observed = await github.getDraft(id);
    if (!observed || observed.draft !== true || observed.prerelease !== true || observed.tag !== release.tag ||
        observed.source !== release.source || !Array.isArray(observed.assets) || observed.assets.length > assets.length)
      throw Error('uploaded-asset-mismatch');
    let pending = observed.assets.length !== assets.length;
    for (const asset of assets) {
      const matches = observed.assets.filter(a => a.name === asset.name);
      if (matches.length > 1 || (matches.length && matches[0].size !== asset.size) ||
          (matches.length && matches[0].digest != null && matches[0].digest !== asset.digest))
        throw Error('uploaded-asset-mismatch');
      if (!matches.length || matches[0].digest == null) pending = true;
    }
    if (!pending) { verified = true; break; }
    if (attempt < 4) await wait(2000);
  }
  if (!verified) throw Error('uploaded-asset-unavailable');
  if (await github.mainHead() !== release.source) throw Error('release-source-changed');
  const body = `# notype ${release.version} — Windows 11 測試版\n\n來源：${release.source}\n\n下載完整離線 ZIP 與 SHA256SUMS.txt，在目標 Windows 11 電腦核對雜湊後完整解壓，執行 Start-notype.cmd。ZIP 已包含語音模型及執行環境；文字整理需要連線至自行設定的 AI server。\n\nZIP SHA-256：\`${assets[0].digest}\`\n\n本次完成自動建置、檔案完整性檢查與隔離資料的原生 --list-models 啟動檢查。乾淨 Windows 11、真人麥克風、公司內網、目標應用程式及人工語音品質仍需實機驗收。草稿需維護者檢查後才發布。\n`;
  await github.finalize(id, body);
  return {tag:release.tag, source:release.source, id, assets};
}

/** Literal argv adapter: run is an async/sync gh invocation with no shell. */
export function createGitHubAdapter(run) {
  const repo = 'swchen44/notype';
  const api = `repos/${repo}`;
  let ownedTag = null;
  async function json(args) { return JSON.parse(await run(['api', ...args])); }
  return {
    async mainHead() { const data=await json([`${api}/commits/main`]); return data.sha; },
    async exists(tag) {
      if (!/^v[0-9A-Za-z.-]{1,65}$/.test(tag)) throw Error('invalid-tag');
      const pages = await json(['--paginate', '--slurp', `${api}/releases`]);
      if (!Array.isArray(pages) || pages.some(page => !Array.isArray(page))) throw Error('invalid-release-list');
      if (pages.flat().some(release => release.tag_name === tag)) return true;
      const refs = await json([`${api}/git/matching-refs/tags/${tag}`]);
      if (!Array.isArray(refs)) throw Error('invalid-tag-list');
      return refs.some(ref => ref.ref === `refs/tags/${tag}`);
    },
    async createDraft(release) {
      const data=await json(['--method','POST',`${api}/releases`, '-f',`tag_name=${release.tag}`, '-f',`target_commitish=${release.source}`, '-f',`name=notype ${release.version}`, '-f',`body=${release.body}`, '-F','draft=true','-F','prerelease=true']);
      if (!Number.isSafeInteger(data.id) || data.id <= 0 || data.tag_name !== release.tag || data.draft !== true || data.prerelease !== true || data.target_commitish !== release.source) throw Error('invalid-created-draft');
      ownedTag=release.tag; return data.id;
    },
    async getDraft(id) {
      if (!Number.isSafeInteger(id) || id <= 0) throw Error('invalid-release-id');
      const data=await json([`${api}/releases/${id}`]);
      if (data.id !== id || !Array.isArray(data.assets)) throw Error('invalid-draft-response');
      return {tag:data.tag_name,source:data.target_commitish,draft:data.draft,prerelease:data.prerelease,body:data.body,
        assets:data.assets.map(a=>({name:a.name,size:a.size,digest:a.state==='uploaded' && typeof a.digest==='string' && a.digest.startsWith('sha256:') ? a.digest.slice(7) : null}))};
    },
    async upload(id, asset) {
      const draft=await this.getDraft(id);
      if (!ownedTag || draft.tag !== ownedTag || !draft.draft || !draft.prerelease || draft.assets.some(a=>a.name===asset.name)) throw Error('draft-changed');
      await run(['release','upload',ownedTag,asset.path,'--repo',repo]);
    },
    async finalize(id, body) {
      const draft=await this.getDraft(id);
      if (!ownedTag || draft.tag !== ownedTag || !draft.draft || !draft.prerelease) throw Error('draft-changed');
      const data=await json(['--method','PATCH',`${api}/releases/${id}`,'-f',`body=${body}`]);
      if (data.id !== id || data.tag_name !== ownedTag || data.draft !== true || data.prerelease !== true || data.body !== body) throw Error('draft-finalize-failed');
    },
  };
}

import { spawn } from 'node:child_process';
import { pathToFileURL } from 'node:url';

function runGh(args) {
  return new Promise((resolve, reject) => {
    const child=spawn('gh',args,{shell:false,windowsHide:true,stdio:['ignore','pipe','pipe']});
    const chunks=[]; let size=0; let overflow=false;
    child.stdout.on('data',chunk=>{size+=chunk.length; if(size>8*1024*1024) {overflow=true; child.kill();} else chunks.push(chunk);});
    child.stderr.resume();
    child.on('error',()=>reject(Error('github-client-unavailable')));
    child.on('close',code=>code===0&&!overflow ? resolve(Buffer.concat(chunks).toString('utf8')) : reject(Error('github-request-failed')));
  });
}

async function main() {
  const mode=process.argv[2];
  const input={version:process.env.RELEASE_VERSION,ref:process.env.GITHUB_REF,source:process.env.GITHUB_SHA,repository:process.env.GITHUB_REPOSITORY};
  const github=createGitHubAdapter(runGh);
  if(mode==='preflight') {
    const release=validateReleaseInput({...input,main:await github.mainHead()});
    if(await github.exists(release.tag)) throw Error('release-or-tag-exists');
    console.log(`Release input verified: ${release.tag}, ${release.source}`);
  } else if(mode==='publish') {
    if(!process.env.RELEASE_ASSEMBLY_DIR) throw Error('assembly-directory-required');
    const receipt=await publishDraft({input,artifactRoot:process.env.RELEASE_ASSEMBLY_DIR,github});
    console.log(`Verified draft ${receipt.tag}, source ${receipt.source}.`);
  } else throw Error('unknown-mode');
}
if(process.argv[1] && import.meta.url===pathToFileURL(process.argv[1]).href) {
  main().catch(error=>{console.error(`release failed: ${error.message}`);process.exitCode=1;});
}
