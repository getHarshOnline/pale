// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
// Real public compiler inputs, not synthetic recipient or publisher admission.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

const BYTES = 4 * 1024 * 1024;
const files = ['pale-workspace.toml', 'organization.toml', 'responsibility.toml',
  'resolution.lock.toml', 'universe.lock.toml'];
export function organizationRequest(fixtureRoot, mutate = values => values) {
  return { rootManifest: files[0], documents: mutate(files.map(label => ({
    label, bytes: readFileSync(path.join(fixtureRoot, label)),
  }))) };
}

export function exerciseOrganization(bindings, api, fixtureRoot) {
  const request = mutate => organizationRequest(fixtureRoot, mutate);
  const compile = mutate => api.compilePortableWorkspace(request(mutate));
  const custodyClasses = ['OrganizationCursor', 'OrganizationRecordHandle', 'OrganizationEntry',
    'OrganizationPage', 'OrganizationSourceCursor', 'OrganizationSourcePage', 'OrganizationDetailCursor',
    'OrganizationDetailPage', 'OrganizationCommunicationResolutionHandle', 'OrganizationServiceAssignmentHandle',
    'OrganizationResponsibilityProjectionHandle', 'OrganizationGenealogyHandle', 'OrganizationResponsibility'];
  for (const name of custodyClasses) assert.throws(() => new bindings[name](), name);
  const plain = value => {
    if (value instanceof bindings.OrganizationResponsibility) {
      return Object.fromEntries(['derivationId', 'lifecycle', 'provenance', 'placement',
        'providerCompletion', 'providerAcceptance', 'consumerAcceptance', 'nativeResult', 'paleSettlement']
        .map(key => [key, plain(value[key])]));
    }
    if (Array.isArray(value)) return value.map(plain);
    if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value)
      .filter(([key, member]) => !(key === 'handle' && typeof member === 'object'))
      .map(([key, member]) => [key, plain(member)]));
    return value;
  };
  const rejection = (call, code) => assert.throws(call, error => {
    assert.equal(error.diagnostic.code, code);
    assert.equal(error.message, error.diagnostic.summary);
    assert(Buffer.isBuffer(error.diagnostic.canonicalBytes));
    assert.equal(JSON.parse(error.diagnostic.canonicalBytes).code, code);
    return true;
  });
  const rows = (owner, limit) => {
    const result = []; let after;
    for (let calls = 0; calls < 10000; calls++) {
      const page = owner.inspectOrganization(limit, limit, BYTES, after);
      assert(page instanceof bindings.OrganizationPage);
      assert(page.belongsTo(owner));
      assert.equal(page.workspaceId, owner.workspaceId);
      assert.equal(page.principalUniverseDigest, owner.principalUniverseDigest);
      assert.equal(page.semanticIrDigest, owner.semanticIrDigest);
      assert.equal(page.admissionEvidenceDigest, owner.admissionEvidenceDigest);
      const entries = page.entries();
      assert(entries.length <= limit && page.inspected <= limit);
      assert(page.copiedStringBytes <= BYTES);
      for (const entry of entries) assert(entry.handle.belongsTo(owner));
      result.push(...entries);
      after = page.continuation();
      assert.equal(page.isComplete, after === null);
      if (after === null) return result;
    }
    assert.fail('bounded fixture enumeration did not terminate');
  };
  let owner = compile();
  const equal = compile();
  assert.equal(owner.semanticIrDigest, equal.semanticIrDigest);
  let tiny = rows(owner, 1);
  assert.deepEqual(tiny.map(v => plain(v.row)), rows(owner, 256).map(v => plain(v.row)));
  const expectedKinds = ['scope','organization','group','discipline-tree','role','occupant-binding',
    'provider-binding','communication-actor','leadership','relation','object','population-rule',
    'group-lens','service','native-projection','inspection','derived-relation','responsibility'];
  assert.deepEqual([...new Set(tiny.map(v => v.row.kind))].sort(), expectedKinds.sort());
  assert(tiny.filter(entry => entry.row.kind === 'relation')
    .every(entry => entry.row.locality === bindings.OrganizationRelationLocality.Internal));
  assert(tiny.filter(entry => ['service','derived-relation'].includes(entry.row.kind))
    .every(entry => !Object.hasOwn(entry.row, 'locality')));
  const nativeOwner = compile(documents => documents.map(document => document.label === 'organization.toml'
    ? { ...document, bytes: Buffer.from(document.bytes.toString() + `
[[relations]]
relation_id = "relation.unobserved"
class = "native_presence"
issuer = "issuer.external"
state = "unknown"
[relations.from]
kind = "role"
[relations.from.role]
namespace = "alpha"
role_id = "accountable-a"
[relations.to]
kind = "native_resource"
resource = "external.unobserved"
[relations.interval]
start = 1
end_exclusive = 10
`) } : document));
  const nativeRows = rows(nativeOwner, 1);
  assert.deepEqual(nativeRows.map(entry => plain(entry.row)), rows(nativeOwner, 256).map(entry => plain(entry.row)));
  const nativeEntry = nativeRows.find(entry => entry.row.kind === 'relation' && entry.row.relationId === 'relation.unobserved');
  assert(nativeEntry); assert(nativeEntry.handle.belongsTo(nativeOwner));
  assert.equal(nativeEntry.row.locality, bindings.OrganizationRelationLocality.NativeBoundary);
  assert.equal(nativeEntry.row.state, bindings.OrganizationFactState.Unknown);
  assert.deepEqual(nativeEntry.row.to, { kind: 'native-resource', nativeResource: 'external.unobserved' });
  const role = tiny.find(v => v.row.kind === 'role' && v.row.group !== null && v.row.disciplineTree !== null);
  assert(role);
  assert.notEqual(role.row.group.groupId, role.row.disciplineTree.treeId);
  assert.notEqual(role.row.axes.owner, role.row.axes.controller);
  assert.notEqual(role.row.axes.controller, role.row.axes.authority);
  assert.equal(role.row.axes.disclosure, bindings.OrganizationDisclosure.Protected);
  assert.equal(role.row.interval.start, 1n);
  assert.equal(role.row.interval.endExclusive, 10n);
  const firstRow = kind => {
    const entry = tiny.find(value => value.row.kind === kind); assert(entry); return entry.row;
  };
  const selectors = [
    {kind:'scope',scope:firstRow('scope').scope},
    {kind:'organization',organization:firstRow('organization').organization},
    {kind:'group',group:firstRow('group').group},
    {kind:'discipline-tree',tree:firstRow('discipline-tree').tree},
    {kind:'role',role:role.row.role},
    {kind:'occupant',occupant:firstRow('occupant-binding').occupant},
    {kind:'provider',provider:firstRow('provider-binding').provider},
    {kind:'object',object:firstRow('object').objectId},
    {kind:'native-resource',nativeResource:'native.unobserved'},
  ];
  const query = (method, selector, limit) => {
    const values = []; let after; let lookup = null;
    for (let calls=0; calls<10000; calls++) {
      const page=owner[method](selector,limit,limit,BYTES,after);
      assert(page.belongsTo(owner));
      assert(page.inspected<=limit && page.entries().length<=limit);
      assert(page.copiedStringBytes<=BYTES);
      if (lookup===bindings.OrganizationEndpointLookup.Found) {
        assert.equal(page.endpointLookup,lookup,'membership remains found on later empty pages');
      }
      lookup=page.endpointLookup;
      for (const entry of page.entries()) {
        assert(entry.handle.belongsTo(owner));
        assert.equal(entry.handle.sources(1,1,BYTES).record.belongsTo(owner),true);
        values.push(plain(entry.row));
      }
      after=page.continuation(); assert.equal(page.isComplete,after===null);
      if(after===null) return {values,lookup};
    }
    assert.fail('bounded endpoint query did not terminate');
  };
  for(const selector of selectors) {
    const found=query('inspectOrganizationEndpoint',selector,1);
    assert.deepEqual(found,query('inspectOrganizationEndpoint',selector,256));
    assert.equal(found.lookup,selector.kind==='native-resource'
      ?bindings.OrganizationEndpointLookup.ExternalReference:bindings.OrganizationEndpointLookup.Found);
    assert.equal(found.values.length===0,selector.kind==='native-resource');
    const incident=query('inspectOrganizationIncidentRelations',selector,1);
    assert.deepEqual(incident,query('inspectOrganizationIncidentRelations',selector,256));
    assert.equal(incident.lookup,null);
    const expected=tiny.map(entry=>plain(entry.row)).filter(row=>
      ['relation','derived-relation'].includes(row.kind)
      &&(isDeepStrictEqual(row.from,selector)||isDeepStrictEqual(row.to,selector)));
    assert.deepEqual(incident.values,expected);
  }
  assert.equal(owner.inspectOrganization(1,1,BYTES).endpointLookup,null);
  const missing={kind:'role',role:{...role.row.role,roleId:`${role.row.role.roleId}-missing`}};
  const initialMissing=owner.inspectOrganizationEndpoint(missing,1,1,BYTES);
  assert.equal(initialMissing.entries().length,0);
  assert.equal(initialMissing.isComplete,false);
  assert.equal(initialMissing.endpointLookup,bindings.OrganizationEndpointLookup.SearchIncomplete);
  assert.deepEqual(query('inspectOrganizationEndpoint',missing,1),
    {values:[],lookup:bindings.OrganizationEndpointLookup.NotFound});
  const selectedRole=selectors[4];
  const endpointCursor=owner.inspectOrganizationEndpoint(selectedRole,1,1,BYTES).continuation();
  assert(endpointCursor);
  rejection(()=>equal.inspectOrganizationEndpoint(selectedRole,1,1,BYTES,endpointCursor),
    'workspace_organization_custody_mismatch');
  rejection(()=>owner.inspectOrganizationEndpoint(missing,1,1,BYTES,endpointCursor),
    'workspace_organization_custody_mismatch');
  rejection(()=>owner.inspectOrganizationIncidentRelations(selectedRole,1,1,BYTES,endpointCursor),
    'workspace_organization_custody_mismatch');
  rejection(()=>owner.inspectOrganizationEndpoint(selectedRole,2,1,BYTES,endpointCursor),
    'workspace_organization_custody_mismatch');
  assert(owner.inspectOrganizationEndpoint(selectedRole,1,1,BYTES,endpointCursor).belongsTo(owner));
  for(const method of ['inspectOrganizationEndpoint','inspectOrganizationIncidentRelations']) {
    rejection(()=>owner[method]({kind:'role',role:{namespace:'',roleId:'invalid'}},1,1,BYTES),
      'workspace_organization_invalid_endpoint');
    for(const count of [0,257]) rejection(()=>owner[method](selectedRole,count,1,BYTES),
      'workspace_organization_invalid_limit');
    for(const count of [NaN,Infinity,1.5,-1]) assert.throws(()=>owner[method](selectedRole,count,1,BYTES));
    for(const malformed of [{kind:'unknown'}, {kind:'role'}, {kind:'role',role:{namespace:12,roleId:'x'}}]) {
      assert.throws(()=>owner[method](malformed,1,1,BYTES));
    }
  }
  rejection(()=>owner.inspectOrganizationEndpoint(selectedRole,256,256,1),
    'workspace_organization_copy_budget');
  assert(tiny.some(v => v.row.kind === 'discipline-tree' && v.row.state === bindings.OrganizationFactState.Absent));
  const services = tiny.filter(v => v.row.kind === 'service');
  assert(services.length >= 2);
  for (const namespace of ['alpha', 'beta', 'gamma']) {
    const scope = { namespace, scopeId: 'scope' };
    const incident = query('inspectOrganizationIncidentServices', scope, 1);
    assert.deepEqual(incident, query('inspectOrganizationIncidentServices', scope, 256));
    assert.equal(incident.lookup, null);
    assert.deepEqual(incident.values, services.map(entry => plain(entry.row)).filter(row =>
      isDeepStrictEqual(row.contract.providerScope, scope) || isDeepStrictEqual(row.contract.consumerScope, scope)));
  }
  const serviceScope = { namespace: 'alpha', scopeId: 'scope' };
  const serviceFirst = owner.inspectOrganizationIncidentServices(serviceScope, 1, 1, BYTES);
  const serviceCursor = serviceFirst.continuation(); assert(serviceCursor);
  const serviceMissing = owner.inspectOrganizationIncidentServices({ namespace: 'gamma', scopeId: 'scope' }, 1, 1, BYTES);
  assert.equal(serviceMissing.entries().length, 0); assert.equal(serviceMissing.inspected, 1);
  assert(serviceMissing.continuation()); assert.equal(serviceMissing.endpointLookup, null);
  for (const call of [
    () => owner.inspectOrganization(1, 1, BYTES, serviceCursor),
    () => owner.inspectOrganizationIncidentRelations(selectedRole, 1, 1, BYTES, serviceCursor),
    () => owner.inspectOrganizationEndpoint(selectedRole, 1, 1, BYTES, serviceCursor),
    () => equal.inspectOrganizationIncidentServices(serviceScope, 1, 1, BYTES, serviceCursor),
    () => owner.inspectOrganizationIncidentServices({ ...serviceScope, namespace: 'beta' }, 1, 1, BYTES, serviceCursor),
    () => owner.inspectOrganizationIncidentServices(serviceScope, 1, 1, BYTES, endpointCursor),
  ]) rejection(call, 'workspace_organization_custody_mismatch');
  assert(owner.inspectOrganizationIncidentServices(serviceScope, 1, 1, BYTES, serviceCursor).belongsTo(owner));
  rejection(() => owner.inspectOrganizationIncidentServices(serviceScope, 1, 1, 1), 'workspace_organization_copy_budget');
  rejection(() => owner.inspectOrganizationIncidentServices({ namespace: '', scopeId: 'scope' }, 1, 1, BYTES), 'workspace_organization_invalid_endpoint');
  for (const invalid of [{ kind: 'scope', scope: serviceScope }, { namespace: 'alpha' }, { namespace: 1, scopeId: 'scope' }]) {
    assert.throws(() => owner.inspectOrganizationIncidentServices(invalid, 1, 1, BYTES));
  }
  for (const count of [0, 257]) rejection(() => owner.inspectOrganizationIncidentServices(serviceScope, count, 1, BYTES), 'workspace_organization_invalid_limit');
  for (const count of [NaN, Infinity, 1.5, -1]) assert.throws(() => owner.inspectOrganizationIncidentServices(serviceScope, count, 1, BYTES));
  for (const entry of services) {
    const row = entry.row;
    assert.notDeepEqual(row.contract.providerScope, row.contract.consumerScope);
    assert.equal(new Set(['providerCompletion','providerAcceptance','consumerAcceptance','nativeReadBack','paleSettlement']
      .map(key => row[key].evidenceRef)).size, 5);
    assert.equal(row.retentionSeconds, 300n);
  }
  const retention = value => docs => docs.map(doc => doc.label === 'organization.toml'
    ? {...doc,bytes:Buffer.from(doc.bytes.toString().replaceAll('retention_seconds = 300',
      `retention_seconds = ${value}`))} : doc);
  const maximumInteger = compile(retention('9007199254740991'));
  const maximumServices = rows(maximumInteger,256).filter(v=>v.row.kind==='service');
  assert(maximumServices.length>=2);
  assert(maximumServices.every(v=>v.row.retentionSeconds===9007199254740991n));
  // The public record grammar remains JCS-safe even though projection uses bigint.
  assert.throws(() => compile(retention('9007199254740993')), error => {
    assert.equal(error.diagnostic.code,'workspace_contract_rejected');
    assert(error.diagnostic.facts.some(fact=>fact.key==='schema_keyword'&&fact.value==='maximum'));
    assert(error.diagnostic.facts.some(fact=>fact.key==='schema_expectation'
      &&fact.value==='a value no greater than 9007199254740991'));
    assert.deepEqual(error.authoringLocation,{sourceLabel:'organization.toml',
      path:[{field:'services'},{index:0},{field:'ceiling'},{field:'retention_seconds'}]});
    return true;
  });
  const leadership = tiny.map(v => v.row).filter(v => v.kind === 'derived-relation'
    && v.source.kind === 'leadership-participation');
  assert.equal(leadership.length, 4);
  assert(leadership.every(v => v.class === bindings.OrganizationDerivedClass.LeadershipParticipation));
  assert(leadership.some((v,i) => leadership.slice(i+1).some(other => v.source.topic === other.source.topic
    && JSON.stringify([v.from,v.to]) !== JSON.stringify([other.from,other.to]))));

  // Every owning section, including seven nested methods, goes through the ABI.
  const seen = new Set(); let sectionCalls = 0;
  const section = (parent, method, kind) => {
    const whole = parent[method](256,256,BYTES);
    assert.equal(whole.rows.kind, kind);
    const expected = plain(whole.rows.values);
    assert(whole instanceof bindings.OrganizationDetailPage);
    assert(whole.record.belongsTo(owner));
    const actual=[]; let after;
    for(let calls=0;calls<10000;calls++) {
      const page=parent[method](1,1,BYTES,after); sectionCalls++;
      assert.equal(page.rows.kind,kind);
      assert.equal(page.parent.kind,whole.parent.kind);
      assert(page.rows.values.length<=1 && page.inspected<=1);
      actual.push(...plain(page.rows.values));
      after=page.continuation();
      assert.equal(page.isComplete,after===null);
      if(after===null) { assert.deepEqual(actual,expected); seen.add(kind); return whole.rows.values; }
    }
    assert.fail('bounded detail did not terminate');
  };
  for(const entry of tiny) {
    const handle=entry.handle;
    switch(entry.row.kind) {
      case 'scope': case 'organization': case 'group': case 'discipline-tree': case 'role': case 'object':
        section(handle,'axisResources','axis-resources'); break;
      case 'leadership':
        section(handle,'leadershipParticipants','leadership-participants');
        section(handle,'leadershipScopes','leadership-scopes'); break;
      case 'communication-actor':
        for(const v of section(handle,'communicationResolutions','communication-resolutions'))
          section(v.handle,'authorizedClosure','communication-closure');
        break;
      case 'group-lens':
        section(handle,'lensStandards','lens-standards'); section(handle,'lensConditions','lens-conditions');
        section(handle,'lensRights','lens-rights'); break;
      case 'service':
        for(const v of section(handle,'serviceAssignments','service-assignments')) {
          section(v.handle,'stewards','assignment-stewards'); section(v.handle,'pods','assignment-pods');
        }
        section(handle,'serviceRights','service-rights'); section(handle,'serviceControllers','service-controllers');
        section(handle,'serviceAuthorities','service-authorities'); break;
      case 'native-projection': section(handle,'nativeReferences','native-references'); break;
      case 'inspection':
        section(handle,'inspectionCoordinates','inspection-coordinates'); section(handle,'inspectionStates','inspection-states'); break;
      case 'responsibility':
        section(handle,'responsibilitySelections','responsibility-selections');
        section(handle,'responsibilityResidences','responsibility-residences');
        for(const v of section(handle,'responsibilityProjections','responsibility-projections'))
          section(v.handle,'nativeReferences','projection-references');
        for(const v of section(handle,'responsibilityGenealogy','responsibility-genealogy')) {
          section(v.handle,'responsibilities','genealogy-responsibilities');
          section(v.handle,'projections','genealogy-projections'); section(v.handle,'occurrences','genealogy-occurrences');
        }
        break;
    }
  }
  assert.equal(seen.size,25);
  const initial=owner.inspectOrganization(1,1,BYTES); const cursor=initial.continuation(); assert(cursor);
  assert.equal(initial.belongsTo(equal),false);
  assert.equal(role.handle.belongsTo(equal),false);
  rejection(()=>equal.inspectOrganization(1,1,BYTES,cursor),'workspace_organization_custody_mismatch');
  rejection(()=>owner.inspectOrganization(1,1,BYTES-1,cursor),'workspace_organization_custody_mismatch');
  for(const invalid of [0,257,65535]) {
    rejection(()=>owner.inspectOrganization(invalid,1,BYTES),'workspace_organization_invalid_limit');
    rejection(()=>owner.inspectOrganization(1,invalid,BYTES),'workspace_organization_invalid_limit');
  }
  for(const invalid of [-1,0.5,NaN,Infinity,65536]) assert.throws(()=>owner.inspectOrganization(invalid,1,BYTES));
  for(const invalid of [-1,0.5,NaN,Infinity,4294967296]) assert.throws(()=>owner.inspectOrganization(1,1,invalid));
  for(const invalid of [0,BYTES+1,4294967295]) rejection(()=>owner.inspectOrganization(1,1,invalid),'workspace_organization_invalid_limit');
  assert.throws(()=>owner.inspectOrganization(1,1,BYTES,{}));
  const second=owner.inspectOrganization(1,1,BYTES,cursor);
  assert.deepEqual(plain(second.entries()[0].row),plain(owner.inspectOrganization(1,1,BYTES,cursor).entries()[0].row));
  const exact=initial.copiedStringBytes; assert(exact>1);
  rejection(()=>owner.inspectOrganization(1,1,exact-1),'workspace_organization_copy_budget');
  for(const budget of [exact,exact+1]) assert.deepEqual(plain(initial.entries()[0].row),plain(owner.inspectOrganization(1,1,budget).entries()[0].row));

  const service=services[0].handle;
  let assignments=service.serviceAssignments(256,256,BYTES).rows.values; assert(assignments.length>=2);
  const retained=assignments[0].handle;
  const firstMembers=retained.stewards(1,1,BYTES); const memberCursor=firstMembers.continuation(); assert(memberCursor);
  rejection(()=>assignments[1].handle.stewards(1,1,BYTES,memberCursor),'workspace_organization_custody_mismatch');
  rejection(()=>retained.pods(1,1,BYTES,memberCursor),'workspace_organization_custody_mismatch');
  rejection(()=>retained.stewards(1,1,BYTES-1,memberCursor),'workspace_organization_custody_mismatch');
  rejection(()=>service.leadershipParticipants(1,1,BYTES),'workspace_organization_section_unavailable');
  const child=retained.stewards(1,1,BYTES,memberCursor);
  assert.equal(child.parent.kind,'assignment');
  assert.equal(child.parent.handle.pods(1,1,BYTES).rows.kind,'assignment-pods');
  const copied=role.row; copied.axes.owner='mutated-js-copy';
  assert.notEqual(role.row.axes.owner,copied.axes.owner);
  const assignment=assignments[0]; const original=assignment.account; assignment.account='mutated';
  assert.equal(service.serviceAssignments(256,256,BYTES).rows.values[0].account,original);

  const binding=tiny.find(v=>v.row.kind==='occupant-binding').handle;
  const unmappedSource=binding.sources(1,1,BYTES);
  assert(unmappedSource.record.sameRecord(binding));
  assert(unmappedSource.record.belongsTo(owner));
  assert.equal(unmappedSource.mapping.kind,'unmapped-record');
  let sourceAfter; const sourceValues=[]; let emptyPartial=false;
  for(let calls=0;calls<10000;calls++) {
    const page=role.handle.sources(1,1,BYTES,sourceAfter); const mapping=page.mapping;
    assert(page.record.sameRecord(role.handle));
    assert(page.record.belongsTo(owner));
    assert.equal(mapping.kind,'named'); assert(mapping.inspected<=1);
    sourceValues.push(...mapping.occurrences);
    emptyPartial ||= mapping.occurrences.length===0 && mapping.continuation!==null;
    sourceAfter=mapping.continuation;
    if(sourceAfter===null) break;
    if(calls===9999) assert.fail('source did not terminate');
  }
  assert(sourceValues.length>0); assert(emptyPartial);
  const firstSource=role.handle.sources(1,1,BYTES).mapping.continuation; assert(firstSource);
  rejection(()=>service.sources(1,1,BYTES,firstSource),'workspace_organization_custody_mismatch');
  rejection(()=>role.handle.sources(1,2,BYTES,firstSource),'workspace_organization_custody_mismatch');
  assert.equal(role.handle.sources(1,1,BYTES,firstSource).mapping.kind,'named');

  // UTF-8 accounting is based on actual compiler source locations, not code units.
  const renamed='organisation-é.toml';
  const utf=compile(docs=>docs.map(doc=>doc.label==='organization.toml'?{...doc,label:renamed}:
    doc.label==='pale-workspace.toml'?{...doc,bytes:Buffer.from(doc.bytes.toString().replace('"organization.toml"',JSON.stringify(renamed)))}:doc));
  const utfRole=rows(utf,256).find(v=>v.row.kind==='role').handle;
  const utfPage=utfRole.sources(256,256,BYTES); const utfMapping=utfPage.mapping;
  assert(utfPage.record.sameRecord(utfRole));
  assert(utfPage.record.belongsTo(utf));
  assert.equal(utfMapping.kind,'named'); assert(utfMapping.occurrences.length>0);
  assert(utfMapping.occurrences.some(v=>v.location.sourceLabel===renamed));
  assert(Buffer.byteLength(renamed)>renamed.length);
  const utfExact=utfPage.copiedStringBytes; assert(utfExact>1);
  const utfExpected=utfMapping.occurrences.reduce((total,v)=>total+Buffer.byteLength(v.location.sourceLabel)
    +v.location.path.reduce((sum,step)=>sum+(typeof step.field==='string'?Buffer.byteLength(step.field):0),0),0);
  assert.equal(utfExact,utfExpected);
  rejection(()=>utfRole.sources(256,256,utfExact-1),'workspace_organization_copy_budget');
  for(const budget of [utfExact,utfExact+1]) assert.equal(utfRole.sources(256,256,budget).copiedStringBytes,utfExact);

  // The same cursor deterministically fails at an actually larger next record.
  const larger=compile(docs=>docs.map(doc=>doc.label==='organization.toml'?{...doc,bytes:Buffer.from(doc.bytes.toString()
    .replace('controller = "controller.beta-scope"',`controller = "${'x'.repeat(255)}"`))}:doc));
  const small=larger.inspectOrganization(1,1,BYTES).copiedStringBytes;
  const limited=larger.inspectOrganization(1,1,small).continuation(); assert(limited);
  for(let i=0;i<3;i++) rejection(()=>larger.inspectOrganization(1,1,small,limited),'workspace_organization_copy_budget');
  rejection(()=>larger.inspectOrganization(1,1,BYTES,limited),'workspace_organization_custody_mismatch');

  // Equal observable assignment bodies still have different exact parents.
  const equalMembers=compile(docs=>docs.map(doc=>{
    if(doc.label!=='organization.toml') return doc;
    const text=doc.bytes.toString();
    const blocks=[...text.matchAll(/\[\[services.assignments\]\][\s\S]*?(?=\[services.ceiling\])/g)];
    assert.equal(blocks.length,2);
    return {...doc,bytes:Buffer.from(text.slice(0,blocks[1].index)+blocks[0][0]+text.slice(blocks[1].index+blocks[1][0].length))};
  }));
  const equalServices=rows(equalMembers,256).filter(v=>v.row.kind==='service');
  const twins=equalServices.map(v=>v.handle.serviceAssignments(256,256,BYTES).rows.values[0]);
  assert.deepEqual(plain(twins[0]),plain(twins[1]));
  assert.deepEqual(twins[0].handle.stewards(256,256,BYTES).rows,twins[1].handle.stewards(256,256,BYTES).rows);
  const twinCursor=twins[0].handle.stewards(1,1,BYTES).continuation(); assert(twinCursor);
  rejection(()=>twins[1].handle.stewards(1,1,BYTES,twinCursor),'workspace_organization_custody_mismatch');
  assert.equal(twins[0].handle.stewards(1,1,BYTES,twinCursor).rows.kind,'assignment-stewards');

  // Correspondence consumes retained handles, not copied row identities.
  const correspondenceKinds=new Set();
  {
  const side=bindings.OrganizationComparisonSide;
  for(const name of ['OrganizationCorrespondence','OrganizationCorrespondenceCursor']) {
    assert.throws(()=>new bindings[name]());
  }
  const comparison=api.comparePortableWorkspaces(owner,equal);
  const correspond=(pair,direction,selected,limit=1)=>{
    let after;
    for(let call=0;call<10000;call++) {
      const result=pair.correspondOrganizationRecord(direction,selected,limit,BYTES,after);
      assert(result.belongsTo(pair));
      assert.equal(result.selectedSide,direction);
      assert(result.selected.sameRecord(selected));
      assert(result.inspected<=limit && result.copiedStringBytes<=BYTES);
      const state=result.state;
      if(state.kind!=='searching') return result;
      assert.equal(result.copiedStringBytes,0);
      after=state.cursor;
    }
    assert.fail('bounded correspondence did not terminate');
  };
  const repeated=rows(owner,256), otherRows=rows(equal,256);
  assert.equal(repeated.length,tiny.length);
  assert.equal(otherRows.length,tiny.length);
  for(const [direction,selectedOwner,otherOwner,selectedRows,targetRows] of [
    [side.Prior,owner,equal,tiny,otherRows],
    [side.Current,equal,owner,otherRows,tiny],
  ]) {
    for(const [index,selected] of selectedRows.entries()) {
      correspondenceKinds.add(selected.row.kind);
      const result=correspond(comparison,direction,selected.handle);
      const state=result.state;
      assert.equal(state.kind,'matched');
      assert.equal(state.values,bindings.WorkspaceValueComparison.Equal);
      assert.deepEqual(plain(state.other.row),plain(selected.row));
      assert(result.selected.belongsTo(selectedOwner));
      assert(state.other.handle.belongsTo(otherOwner));
      assert(state.other.handle.sameRecord(targetRows[index].handle));
      assert(!state.other.handle.sameRecord(selected.handle));
      assert.equal(result.selected.sources(256,256,BYTES).mapping.kind,
        state.other.handle.sources(256,256,BYTES).mapping.kind);
    }
  }
  assert.equal(correspondenceKinds.size,18);
  for(let index=0;index<tiny.length;index++) {
    assert(tiny[index].handle.sameRecord(repeated[index].handle));
    assert(repeated[index].handle.sameRecord(tiny[index].handle));
    assert(!tiny[index].handle.sameRecord(otherRows[index].handle));
  }
  assert(!tiny[0].handle.sameRecord(tiny[1].handle));
  assert(!tiny[0].handle.sameRecord(role.handle));
  const mutateOrganization=replace=>docs=>docs.map(doc=>{
    if(doc.label!=='organization.toml') return doc;
    const source=doc.bytes.toString(); const changed=replace(source);
    assert.notEqual(changed,source,'owning fixture mutation must apply');
    return {...doc,bytes:Buffer.from(changed)};
  });
  const nested=compile(mutateOrganization(source=>source.replace(
    'resources = ["resource.role-a"]','resources = ["resource.changed"]')));
  const nestedPair=api.comparePortableWorkspaces(owner,nested);
  const changedRoles=tiny.filter(entry=>entry.row.kind==='role').map(entry=>({
    selected:entry,result:correspond(nestedPair,side.Prior,entry.handle,256),
  })).filter(item=>item.result.state.values===bindings.WorkspaceValueComparison.Changed);
  assert.equal(changedRoles.length,1);
  const changedRole=changedRoles[0];
  assert.deepEqual(plain(changedRole.selected.row),plain(changedRole.result.state.other.row));
  assert.notDeepEqual(changedRole.selected.handle.axisResources(256,256,BYTES).rows,
    changedRole.result.state.other.handle.axisResources(256,256,BYTES).rows);
  const interval=compile(mutateOrganization(source=>source.replace(
    '[occupant_bindings.interval]\nstart = 1','[occupant_bindings.interval]\nstart = 2')));
  const intervalPair=api.comparePortableWorkspaces(owner,interval);
  for(const [direction,selectedOwner] of [[side.Prior,owner],[side.Current,interval]]) {
    const binding=rows(selectedOwner,256).find(entry=>entry.row.kind==='occupant-binding');
    const first=intervalPair.correspondOrganizationRecord(direction,binding.handle,1,BYTES);
    assert.equal(first.state.kind,'searching');
    assert.equal(first.inspected,1);
    const terminal=intervalPair.correspondOrganizationRecord(direction,binding.handle,1,BYTES,first.state.cursor);
    assert.equal(terminal.state.kind,'unmatched');
    assert.equal(terminal.inspected,0);
    assert.equal(terminal.selected.sources(256,256,BYTES).mapping.kind,'unmapped-record');
  }
  const scopes=tiny.filter(entry=>entry.row.kind==='scope'); assert(scopes.length>=2);
  const selected=scopes.at(-1).handle;
  const first=comparison.correspondOrganizationRecord(side.Prior,selected,1,BYTES);
  assert.equal(first.state.kind,'searching'); const cursor=first.state.cursor;
  const anotherPair=api.comparePortableWorkspaces(owner,compile());
  const reversed=api.comparePortableWorkspaces(equal,owner);
  for(const [pair,direction,record,visits,bytes] of [
    [anotherPair,side.Prior,selected,1,BYTES],
    [reversed,side.Current,selected,1,BYTES],
    [comparison,side.Current,selected,1,BYTES],
    [comparison,side.Prior,scopes[0].handle,1,BYTES],
    [comparison,side.Prior,selected,2,BYTES],
    [comparison,side.Prior,selected,1,BYTES-1],
  ]) rejection(()=>pair.correspondOrganizationRecord(direction,record,visits,bytes,cursor),
    'workspace_organization_custody_mismatch');
  assert(!first.belongsTo(anotherPair));
  assert(comparison.correspondOrganizationRecord(side.Prior,selected,1,BYTES,cursor).belongsTo(comparison));
  for(const count of [0,257]) rejection(()=>comparison.correspondOrganizationRecord(side.Prior,selected,count,BYTES),
    'workspace_organization_invalid_limit');
  for(const count of [NaN,Infinity,1.5,-1]) {
    assert.throws(()=>comparison.correspondOrganizationRecord(side.Prior,selected,count,BYTES));
    assert.throws(()=>comparison.correspondOrganizationRecord(side.Prior,selected,1,count));
  }
  assert.throws(()=>comparison.correspondOrganizationRecord(999,selected,1,BYTES));
  assert.throws(()=>comparison.correspondOrganizationRecord(side.Prior,{},1,BYTES));
  const copySelected=scopes[1].handle;
  const copyStart=comparison.correspondOrganizationRecord(side.Prior,copySelected,1,1);
  assert.equal(copyStart.state.kind,'searching'); const copyCursor=copyStart.state.cursor;
  for(let attempt=0;attempt<2;attempt++) rejection(()=>comparison.correspondOrganizationRecord(
    side.Prior,copySelected,1,1,copyCursor),'workspace_organization_copy_budget');
  assert.equal(copySelected.sources(256,256,BYTES).mapping.kind,'named');
  assert.equal(correspond(comparison,side.Prior,copySelected).state.kind,'matched');
  }

  // Independent nested handle/cursor use remains valid after its own caller references disappear.
  // GC is requested when available; no deterministic native finalization timing is claimed.
  const independent=(()=>{
    const original=compile();
    const root=rows(original,256).find(v=>v.row.kind==='service').handle;
    const parent=root.serviceAssignments(256,256,BYTES).rows.values[0].handle;
    const after=parent.stewards(1,1,BYTES).continuation(); assert(after);
    return {root,parent,after};
  })();
  owner=null; tiny=null; assignments=null; global.gc?.();
  assert.equal(independent.parent.stewards(1,1,BYTES,independent.after).rows.kind,'assignment-stewards');
  assert.equal(independent.root.sources(1,1,BYTES).mapping.kind,'named');
  return { organizationFamilies:expectedKinds.length, endpointKinds:selectors.length,
    endpointQueries:true, incidentQueries:true, correspondenceFamilies:correspondenceKinds.size,
    correspondenceQueries:true, sameRecord:true, detailVariants:seen.size, sectionCalls,
    emptyIncompleteSource:true, exactSourcePageRecord:true, suppliedInputOnly:true };
}

if(process.argv[2]==='--module') {
  const bindings=createRequire(import.meta.url)(process.argv[3]);
  console.log(JSON.stringify(exerciseOrganization(bindings,new bindings.Pale(),process.argv[4]),null,2));
}
