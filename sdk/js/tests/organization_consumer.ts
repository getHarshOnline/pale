// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import { Pale, OrganizationFactState, OrganizationEndpointLookup, OrganizationComparisonSide,
  OrganizationRelationLocality,
  type OrganizationCorrespondenceState, type OrganizationCorrespondenceCursor,
  type WorkspaceComparison, type WorkspaceValueComparison, type OrganizationRow, type OrganizationNode,
  type PortableWorkspaceCompilation,
  type OrganizationDetailRows, type OrganizationDetailParent, type OrganizationPage,
  type OrganizationCursor, type OrganizationRecordHandle, type OrganizationDetailCursor,
  type OrganizationServiceAssignmentHandle, type OrganizationSourceMapping,
  type OrganizationSourceCursor, type OrganizationSourcePage, type OrganizationInterval } from '@rtepl/pale';

// Checked against actual generated declarations in the installed package, not a mirror.
export function observe(row: OrganizationRow): string {
  switch(row.kind) {
    case 'scope': return row.scope.namespace;
    case 'organization': return row.organization.organizationId;
    case 'group': return row.group.groupId;
    case 'discipline-tree': return row.tree.treeId;
    case 'role': {
      const group: string | undefined = row.group?.groupId;
      const tree: string | undefined = row.disciplineTree?.treeId;
      const end: bigint | null = row.interval.endExclusive;
      void group; void tree; void end;
      // @ts-expect-error No service evidence on a role variant.
      void row.providerCompletion;
      return row.role.roleId;
    }
    case 'occupant-binding': return row.occupant;
    case 'provider-binding': return row.provider;
    case 'communication-actor': return row.handle;
    case 'leadership': return row.topic;
    case 'relation': {
      const locality: OrganizationRelationLocality = row.locality;
      void locality;
      return row.relationId;
    }
    case 'object': return row.objectId;
    case 'population-rule': return row.objectClass;
    case 'group-lens': return row.profileId;
    case 'service': {
      // @ts-expect-error Explicit-relation locality is not a service law.
      void row.locality;
      const retention: bigint = row.retentionSeconds;
      void retention;
      return row.contract.consumerScope.namespace;
    }
    case 'native-projection': return row.projectionId;
    case 'inspection': return row.inspectionId;
    case 'derived-relation':
      // @ts-expect-error Derived relations keep their own class semantics.
      void row.locality;
      return endpoint(row.from);
    case 'responsibility': {
      const answer = row.record.placement;
      if(answer.kind === 'omitted') {
        // @ts-expect-error Omission does not carry a supplied value.
        void answer.value;
        return answer.reason;
      }
      return answer.value.kind;
    }
    default: { const never: never = row; return never; }
  }
}
function endpoint(node: OrganizationNode): string {
  switch(node.kind) {
    case 'scope': return node.scope.scopeId;
    case 'organization': return node.organization.organizationId;
    case 'group': return node.group.groupId;
    case 'discipline-tree': return node.tree.treeId;
    case 'role': return node.role.roleId;
    case 'occupant': return node.occupant;
    case 'provider': return node.provider;
    case 'native-resource': return node.nativeResource;
    case 'object': return node.object;
    default: { const never: never = node; return never; }
  }
}
export function section(rows: OrganizationDetailRows): number {
  switch(rows.kind) {
    case 'axis-resources': case 'lens-standards': case 'lens-conditions':
    case 'assignment-stewards': case 'assignment-pods': case 'service-rights':
    case 'service-controllers': case 'service-authorities': case 'native-references':
    case 'projection-references': case 'genealogy-responsibilities':
    case 'genealogy-projections': case 'genealogy-occurrences': {
      const strings: string[] = rows.values; return strings.length;
    }
    case 'leadership-participants': return rows.values[0]?.roleId.length ?? 0;
    case 'leadership-scopes': return rows.values[0]?.scopeId.length ?? 0;
    case 'communication-resolutions': return rows.values[0]?.localNode.kind.length ?? 0;
    case 'communication-closure': return rows.values[0]?.kind.length ?? 0;
    case 'lens-rights': return rows.values.length;
    case 'service-assignments': {
      const assignment: OrganizationServiceAssignmentHandle | undefined = rows.values[0]?.handle;
      void assignment;
      // @ts-expect-error Assignment arrays do not carry role fields.
      void rows.values[0]?.roleId;
      return rows.values.length;
    }
    case 'inspection-coordinates': return rows.values.length;
    case 'inspection-states': { const states: OrganizationFactState[] = rows.values; return states.length; }
    case 'responsibility-selections': return rows.values[0]?.selection.kind.length ?? 0;
    case 'responsibility-residences': return rows.values[0]?.resource.length ?? 0;
    case 'responsibility-projections': return rows.values[0]?.projectionId.length ?? 0;
    case 'responsibility-genealogy': return rows.values[0]?.linkId.length ?? 0;
    default: { const never: never = rows; return never; }
  }
}
export function source(value: OrganizationSourceMapping): number {
  if(value.kind === 'unmapped-record') {
    // @ts-expect-error Unmapped does not expose a named frontier.
    void value.continuation;
    return 0;
  }
  const after: OrganizationSourceCursor | null = value.continuation; void after;
  return value.occurrences.length;
}
export function sourcePage(value: OrganizationSourcePage, record: OrganizationRecordHandle): number {
  if(!value.record.sameRecord(record)) throw new Error('source page lost its exact record');
  const copiedStringBytes: number = value.copiedStringBytes;
  return source(value.mapping) + copiedStringBytes;
}
export function parent(value: OrganizationDetailParent): void {
  switch(value.kind) {
    case 'record': value.handle.axisResources(1,1,4096); return;
    case 'communication': value.handle.authorizedClosure(1,1,4096); return;
    case 'assignment': value.handle.stewards(1,1,4096); return;
    case 'projection': value.handle.nativeReferences(1,1,4096); return;
    case 'genealogy': value.handle.occurrences(1,1,4096); return;
    default: { const never: never = value; void never; }
  }
}
export function nominal(page: OrganizationPage, cursor: OrganizationCursor,
  record: OrganizationRecordHandle, detail: OrganizationDetailCursor): void {
  const next: OrganizationCursor | null = page.continuation(); void next;
  // @ts-expect-error Page is not a record handle despite shared owner metadata.
  const wrongRecord: OrganizationRecordHandle = page; void wrongRecord;
  // @ts-expect-error Enumeration cursor cannot impersonate a detail cursor.
  const wrongDetail: OrganizationDetailCursor = cursor; void wrongDetail;
  // @ts-expect-error A DTO/cursor cannot reconstruct the opaque compilation.
  record.belongsTo(detail);
  // @ts-expect-error Intervals use bigint rather than lossy number values.
  const lossy: OrganizationInterval = {start: 9007199254740993, endExclusive: null}; void lossy;
}
export function inspectEndpoint(owner: PortableWorkspaceCompilation, node: OrganizationNode,
  after: OrganizationCursor): OrganizationEndpointLookup | null {
  const page: OrganizationPage=owner.inspectOrganizationEndpoint(node,1,1,4096,after);
  owner.inspectOrganizationIncidentRelations(node,1,1,4096);
  const services: OrganizationPage = owner.inspectOrganizationIncidentServices({ namespace: 'alpha', scopeId: 'scope' }, 1, 1, 4096);
  const continuation: OrganizationCursor | null = services.continuation(); void continuation;
  // @ts-expect-error A generic endpoint is not a qualified scope selector.
  owner.inspectOrganizationIncidentServices(node, 1, 1, 4096);
  // @ts-expect-error Service cursors retain the SDK cursor type.
  owner.inspectOrganizationIncidentServices({ namespace: 'alpha', scopeId: 'scope' }, 1, 1, 4096, services.entries()[0]?.handle);
  // @ts-expect-error An endpoint needs a typed member payload, not a display name.
  owner.inspectOrganizationEndpoint({kind:'role',label:'Reviewer'},1,1,4096);
  // @ts-expect-error Record custody is not query continuation custody.
  owner.inspectOrganizationEndpoint(node,1,1,4096,page.entries()[0]?.handle);
  return page.endpointLookup;
}
export function correspondence(pair: WorkspaceComparison, record: OrganizationRecordHandle,
  after: OrganizationCorrespondenceCursor): OrganizationCorrespondenceState {
  const result=pair.correspondOrganizationRecord(OrganizationComparisonSide.Prior,record,1,4096,after);
  const belongs: boolean=result.belongsTo(pair);
  const exact: boolean=result.selected.sameRecord(record);
  const inspected: number=result.inspected;
  const copied: number=result.copiedStringBytes;
  void belongs; void exact; void inspected; void copied;
  const state=result.state;
  switch(state.kind) {
    case 'searching': { const cursor: OrganizationCorrespondenceCursor=state.cursor; void cursor; break; }
    case 'unmatched': {
      // @ts-expect-error Complete absence carries no invented counterpart.
      void state.other; break;
    }
    case 'matched': {
      const values: WorkspaceValueComparison=state.values;
      const handle: OrganizationRecordHandle=state.other.handle; void values; void handle; break;
    }
    default: { const never: never=state; void never; }
  }
  // @ts-expect-error Enumeration continuation cannot stand in for correspondence custody.
  const enumeration: OrganizationCursor=after; void enumeration;
  // @ts-expect-error Direction is the generated enum, not a caller-selected spelling.
  pair.correspondOrganizationRecord('prior',record,1,4096);
  return state;
}
const api = new Pale();
console.log(JSON.stringify({loaded: api.capabilityIds.length > 0, organizationDeclarations: true},null,2));
