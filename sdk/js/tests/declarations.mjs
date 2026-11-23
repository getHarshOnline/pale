import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
import { nativeDeclarations } from "../tools/declarations.mjs";

const raw = await readFile(new URL("../target/napi-raw.d.ts", import.meta.url), "utf8");
const generated = await readFile(new URL("../index.d.ts", import.meta.url), "utf8");
assert.equal(nativeDeclarations(raw), generated);
assert.equal(nativeDeclarations(generated), generated);
const parse = (text) => ts.createSourceFile("index.d.ts", text, ts.ScriptTarget.Latest, true);
const printer = ts.createPrinter({ newLine: ts.NewLineKind.LineFeed });
const shapePrinter = ts.createPrinter({ newLine: ts.NewLineKind.LineFeed, removeComments: true });
const comments = (source) => {
  const ranges = new Map();
  const visit = (node) => {
    for (const position of [node.pos, node.end]) {
      for (const range of [...(ts.getLeadingCommentRanges(source.text, position) ?? []),
        ...(ts.getTrailingCommentRanges(source.text, position) ?? [])]) {
        ranges.set(range.pos, source.text.slice(range.pos, range.end));
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return [...ranges].sort(([left], [right]) => left - right).map(([, text]) => text);
};
const original = parse(raw);
const projected = parse(generated);
const recipientQualifiedDeclarations = new Set([
  "RecipientQualifiedComparisonSubjectCursor",
  "RecipientQualifiedComparisonSubjectPage",
  "RecipientQualifiedContributingRootsCursor",
  "RecipientQualifiedContributingRootsPage",
  "RecipientQualifiedDependencyCursor",
  "RecipientQualifiedDependencyPage",
  "RecipientQualifiedOrganizationCorrespondence",
  "RecipientQualifiedOrganizationCorrespondenceCursor",
  "RecipientQualifiedOrganizationCorrespondenceState",
  "RecipientQualifiedOrganizationCursor",
  "RecipientQualifiedOrganizationEntry",
  "RecipientQualifiedOrganizationPage",
  "RecipientQualifiedOrganizationRecordHandle",
  "RecipientQualifiedOrganizationSourceCursor",
  "RecipientQualifiedOrganizationSourceMapping",
  "RecipientQualifiedOrganizationSourcePage",
  "RecipientQualifiedSubjectInspection",
  "RecipientQualifiedSubjectInspectionCursor",
  "RecipientQualifiedWorkspaceComparisonInspection",
  "RecipientQualifiedWorkspaceInspection",
  "RecipientQualifiedWorkspacePairSummary",
  "RecipientQualifiedWorkspaceSummary",
]);
for (const statement of projected.statements) {
  if ("name" in statement && statement.name && ts.isIdentifier(statement.name)) {
    recipientQualifiedDeclarations.delete(statement.name.text);
  }
}
assert.deepEqual([...recipientQualifiedDeclarations], [],
  "Generated declarations must expose every recipient-qualified inspection type");
let classes = 0;
let factoryOnly = 0;
const restored = projected.statements.map((statement, index) => {
  const owner = original.statements[index];
  if (!ts.isClassDeclaration(statement)) return statement;
  assert(ts.isClassDeclaration(owner));
  classes++;
  const constructed = owner.members.some(ts.isConstructorDeclaration);
  if (!constructed) factoryOnly++;
  const members = statement.members.filter((member) =>
    member.name?.getText(projected) !== "__paleNativeHandle"
    && (constructed || !ts.isConstructorDeclaration(member)));
  return ts.factory.updateClassDeclaration(statement, statement.modifiers, statement.name,
    statement.typeParameters, statement.heritageClauses, members);
});
// Layout changes do not change declarations, comment text, or attribution.
assert.equal(shapePrinter.printFile(ts.factory.updateSourceFile(projected, restored)), shapePrinter.printFile(original));
assert.deepEqual(comments(projected), comments(parse(printer.printFile(original))));

const documentedUnion = 'export type Example = { kind: "value"; /** Payload text. */ value: string; '
  + 'literal: "/** not a comment */"; template: `prefix ${string} /** not a comment */`; }';
const renderedUnion = nativeDeclarations(documentedUnion);
const unionSource = parse(renderedUnion);
const payload = unionSource.statements[0].type.members.find((member) => member.name.getText(unionSource) === "value");
assert.deepEqual(payload.jsDoc?.map((doc) => doc.comment), ["Payload text."]);
assert.equal(shapePrinter.printFile(unionSource), shapePrinter.printFile(parse(documentedUnion)));
assert.deepEqual(comments(unionSource), comments(parse(documentedUnion)));
assert.equal(nativeDeclarations(renderedUnion), renderedUnion);

const proofFamilies = new Set(["OrganizationRow", "OrganizationDetailRows", "OrganizationNode", "OrganizationPlacementAnswer", "OrganizationSourceMapping", "OrganizationDetailParent", "EffectiveProfileExplanationEvidence", "EffectiveProfileFinding",
  "EffectiveProfilePathReason", "EffectiveProfileProviderDisposition", "RequirementDisposition"]);
let documentedFields = 0;
for (const statement of projected.statements) {
  if (!ts.isTypeAliasDeclaration(statement) || !proofFamilies.delete(statement.name.text)) continue;
  assert(ts.isUnionTypeNode(statement.type));
  for (const variant of statement.type.types) {
    assert(ts.isTypeLiteralNode(variant));
    for (const member of variant.members) {
      if (member.name.getText(projected) === "kind") continue;
      assert(member.jsDoc?.some((doc) => typeof doc.comment === "string" && doc.comment.length > 0),
        `${statement.name.text}.${member.name.getText(projected)} must expose field help`);
      documentedFields++;
    }
  }
}
assert.equal(proofFamilies.size, 0);
assert(documentedFields > 0);
assert.throws(() => nativeDeclarations("export declare class Broken {"), /Invalid/);
assert.throws(() => nativeDeclarations("export declare class Child extends Parent {}"), /inheritance/);
assert.throws(() => nativeDeclarations("export declare class Collision { __paleNativeHandle: string }"), /collides/);
assert.throws(
  () => nativeDeclarations(
    "export interface ProfileInspectionContext {}\nexport interface ProfileInspectionContext2 {}\n",
  ),
  /numeric collision counter: ProfileInspectionContext2/,
);
assert.throws(
  () => nativeDeclarations(
    "export interface AttributableAnswerV1 {}\nexport interface AttributableAnswerV114 {}\n",
  ),
  /meaningless collision name AttributableAnswerV114/,
);
assert.throws(
  () => nativeDeclarations(
    "export interface TypedDigestV1 {}\nexport interface TypedDigestV1TypedDigestCanonicalizationV1 {}\n",
  ),
  /repeats a version segment: TypedDigestV1TypedDigestCanonicalizationV1/,
);
assert.throws(
  () => nativeDeclarations(
    "export interface ProviderBindingV1 {}\nexport interface ProviderBindingV1ProviderCoordinates {}\n",
  ),
  /repeats its owning declaration: ProviderBindingV1ProviderCoordinates/,
);
assert.doesNotThrow(() => nativeDeclarations(
  "export interface Sha256DigestV1 {}\nexport interface Http2PolicyV1 {}\n",
));
console.log(JSON.stringify({ classes, factoryOnly, documentedFields, deterministic: true, originalDeclarationsPreserved: true }, null, 2));
