// Temporary action roundtrip: synthetic bytes and an in-memory GitHub adapter only.
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {publishDraft} from './release-controller.mjs';
const root=process.env.ROUNDTRIP_ROOT,mode=process.argv[2];
if(!root || !['create','verify'].includes(mode)) throw Error('roundtrip-input-invalid');
const source='a'.repeat(40),version='0.1.2-test.1',hash=b=>createHash('sha256').update(b).digest('hex');
if(mode==='create'){
 await mkdir(join(root,'assets'),{recursive:true});
 const entries=new Map([[`notype-${version}-offline-win-x64.zip`,Buffer.from('synthetic zip')],['Windows11-README.md',Buffer.from('synthetic readme')],['VERIFICATION.md',Buffer.from('synthetic verification')]]);
 for(const [name,data] of entries)await writeFile(join(root,'assets',name),data);
 await writeFile(join(root,'assets','SHA256SUMS.txt'),[...entries].map(([name,data])=>`${hash(data)}  ${name}\n`).join(''));
 const zip=entries.values().next().value;
 await writeFile(join(root,'assembly-report.json'),JSON.stringify({source,version,checks:['zip-crc-sha','fresh-extraction','bundled-cli','native-headless-list-models'],archive:{file:`notype-${version}-offline-win-x64.zip`,bytes:zip.length,sha256:hash(zip)}}));
} else {
 const state={draft:null,assets:[],finalized:false};
 const github={async mainHead(){return source},async exists(){return false},async tagExists(){return false},
  async createDraft(data){state.draft={...data,id:1,draft:true,prerelease:true};return 1},
  async upload(id,asset){state.assets.push(asset)},async getDraft(){return {...state.draft,assets:state.assets}},
  async finalize(id,body){state.draft.body=body;state.finalized=true}};
 const receipt=await publishDraft({input:{source,version,ref:'refs/heads/main',repository:'swchen44/notype'},artifactRoot:root,github});
 if(receipt.assets.length!==4||!state.finalized)throw Error('roundtrip-controller-failed');
 if((await readFile(join(root,'assets','Windows11-README.md'),'utf8'))!=='synthetic readme')throw Error('roundtrip-data-mismatch');
 console.log('Synthetic ID-selected artifact passed the real controller with an in-memory GitHub adapter.');
}
