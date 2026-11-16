const assert = require('node:assert/strict');
const m = require('../../experiments/kwin-overview/contents/ui/BridgeModel.js');
const envelope = {schemaVersion:1,instanceId:'instance',revision:'revision',available:true,data:{items:[]}};
assert.equal(m.decode(JSON.stringify(envelope)).schemaVersion,1);
for (const bad of ['',null,'x'.repeat(131073),JSON.stringify({...envelope,schemaVersion:2}),JSON.stringify({...envelope,available:'yes'}),JSON.stringify({...envelope,instanceId:'x'.repeat(65)})]) assert.throws(()=>m.decode(bad));
assert.deepEqual(m.hierarchyRows([{kind:'world',id:'id',parentId:null,name:'Safe\u202e\nName'}]),[{kind:'world',id:'id',parentId:null,name:'SafeName'}]);
assert.throws(()=>m.hierarchyRows(Array(65).fill({})));
for (const bad of [{kind:'project',id:'id',name:'name',parentId:null},{kind:'world',id:'id',name:'x'.repeat(481),parentId:null}])assert.throws(()=>m.hierarchyRows([bad]));
const native='12345678-1234-1234-1234-123456789abc';
for(const status of ['confirmed','suggested','conflicted','unknown','stale']) {
 const rows=m.contextRows([{window:{epoch:'session',runtime_id:'runtime'},nativeId:native,status,targets:['logical']}]);
 assert.equal(m.badges(rows)[native].status,status);
 assert.deepEqual(m.badges(rows.concat(rows)),{});
}
assert.throws(()=>m.contextRows([{window:{epoch:'s',runtime_id:'r'},status:'fabricated',targets:[]}]));
assert.equal(m.contextRows([{window:{epoch:'s',runtime_id:'r'},nativeId:'app.id',status:'unknown',targets:[]}])[0].nativeId,'');
console.log('PASS bounded decoder, schema mismatch, redaction, all association statuses, ambiguous UUID refusal');
