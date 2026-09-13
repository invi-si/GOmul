import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
function moduleUrl(name){
 const source=readFileSync(new URL('../src/ts/'+name+'.ts',import.meta.url),'utf8');
 let js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
 js=js.replace(/from ['"]\.\/([^'"]+)['"]/g,(_,dependency)=>'from '+JSON.stringify(moduleUrl(dependency)));
 return 'data:text/javascript;base64,'+Buffer.from(js).toString('base64');
}
const {validateBackup}=await import(moduleUrl('wasm-save-backup'));
const valid=()=>({format:'gomul-browser-save',version:1,game:'game-a',settings:{width:240,height:320},stores:[{kind:'records',appId:'pid',entries:[{key:['save','0'],data:btoa('\0\xff')},{key:'empty',data:''}]}]});
test('backup accepts binary and empty records without changing keys or bytes',()=>{
 const b=valid();assert.equal(validateBackup(b,'game-a'),b);
 assert.equal(atob(b.stores[0].entries[0].data),'\0\xff');
});
test('backup rejects another game, unknown versions and malformed records before installation',()=>{
 assert.throws(()=>validateBackup(valid(),'game-b'));
 for(const mutate of [b=>b.version=2,b=>b.stores[0].kind='arbitrary-database',b=>b.stores[0].entries[0].key={},b=>b.stores[0].entries[0].data='@',b=>b.settings.width=0,b=>b.settings.phoneNumber='123',b=>b.stores.push(b.stores[0]),b=>b.stores[0].entries.push(b.stores[0].entries[0])]){
  const b=valid();mutate(b);assert.throws(()=>validateBackup(b,'game-a'));
 }
});
