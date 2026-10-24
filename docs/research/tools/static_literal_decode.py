from pathlib import Path
import ast, re, json, operator, hashlib

# Restricted literal substitution only. Never evaluates or imports target JavaScript.
OPS = {ast.Add: operator.add, ast.Sub: operator.sub, ast.Mult: operator.mul, ast.Div: operator.truediv}
def arithmetic(s):
    def visit(n):
        if isinstance(n, ast.Expression): return visit(n.body)
        if isinstance(n, ast.Constant) and isinstance(n.value, (int, float)): return n.value
        if isinstance(n, ast.BinOp) and type(n.op) in OPS: return OPS[type(n.op)](visit(n.left), visit(n.right))
        if isinstance(n, ast.UnaryOp) and isinstance(n.op, ast.USub): return -visit(n.operand)
        if isinstance(n, ast.UnaryOp) and isinstance(n.op, ast.UAdd): return visit(n.operand)
        raise ValueError('Unsupported arithmetic syntax')
    return visit(ast.parse(s, mode='eval'))

def decode(s):
    decoder = re.search(r'function (_0x\w+)\((_0x\w+),_0x\w+\)\{\2=\2-(0x[0-9a-f]+);const _0x\w+=(_0x\w+)\(\);', s)
    if not decoder: return None
    name, _, offset, table_name = decoder.groups(); offset = int(offset,16)
    start = re.search(r'function '+table_name+r'\(\)\{const _0x\w+=',s).end()
    array = re.match(r"\[(?:'(?:\\.|[^'\\])*'(?:,|(?=\])))*\]",s[start:])
    if not array: raise ValueError('Unrecognized literal array')
    values=ast.literal_eval(array.group())
    rotation = re.search(r'try\{const _0x\w+=(.*?);if\(_0x\w+===_0x\w+\)',s)
    target = re.search(r'\}\('+table_name+r',(0x[0-9a-f]+)\)\)',s)
    if not rotation or not target: raise ValueError('Unrecognized rotation')
    expression=rotation.group(1); target=int(target.group(1),16)
    for r in range(len(values)):
        def replace_parse(m):
            v=values[(int(m.group(1),16)-offset+r)%len(values)]
            n=re.match(r'\s*([+-]?\d+)',v)
            if not n: raise ValueError('NaN')
            return n.group(1)
        try:
            expr=re.sub(r'parseInt\(_0x\w+\((0x[0-9a-f]+)\)\)',replace_parse,expression)
            if arithmetic(expr)==target: break
        except (ValueError,ZeroDivisionError): pass
    else: raise ValueError('No checksum rotation matched')
    values=values[r:]+values[:r]
    aliases={name}
    assignments=re.findall(r'\b(_0x\w+)\s*=\s*(_0x\w+)\b',s)
    for _ in range(len(assignments)+1):
        new=aliases|{a for a,b in assignments if b in aliases}
        if new==aliases:break
        aliases=new
    pattern=re.compile(r'\b('+'|'.join(sorted(aliases,key=len,reverse=True))+r')\((0x[0-9a-f]+)\)')
    replacements=[]
    def sub(m):
        index=int(m.group(2),16)-offset
        if not 0<=index<len(values):return m.group()
        result=values[index]
        replacements.append({'start':m.start(),'end':m.end(),'value':result})
        return json.dumps(result,ensure_ascii=False)
    result=pattern.sub(sub,s)
    return result,{'decoder':name,'table':table_name,'offset':offset,'rotation':r,'entries':len(values),'aliases':len(aliases),'replacements':replacements}

import argparse
parser=argparse.ArgumentParser(description='Statically substitute literal string-table lookups in selected Typeless 2.8.3 bundles. Never executes target JavaScript.')
parser.add_argument('extracted_root',type=Path)
root=parser.parse_args().extracted_root
output=root/'decoded';output.mkdir(exist_ok=True)
manifest=[]
for rel in ['dist/main/index.js','dist/preload/index.mjs','dist/main/worker/chunkSessionWorker.js','dist/main/worker/opusWorker.js']:
    p=root/rel;s=p.read_text(); d=decode(s)
    if not d: print('NO_DECODER',rel);continue
    t,meta=d; dest=output/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(t)
    meta['path']=rel;meta['sha256']=hashlib.sha256(p.read_bytes()).hexdigest();manifest.append(meta)
    print(rel,{k:v for k,v in meta.items() if k!='replacements'})
(output/'substitutions.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2))
