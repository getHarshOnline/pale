// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import ts from "typescript";
import { assertMeaningfulDeclarationNames } from "./declaration-names.mjs";

/** Preserve napi-rs class identity without introducing JavaScript members.
 *
 * A private member belongs to its declaring class in TypeScript's compatibility
 * relation. Factory-only native classes also have no public constructor. DTO
 * interfaces remain structural; Rust/napi-rs still own every callable shape.
 * Field documentation starts on its own line so editors attach it to the field.
 */
export function nativeDeclarations(text) {
  let source = ts.createSourceFile("index.d.ts", text, ts.ScriptTarget.Latest, true);
  if (source.parseDiagnostics.length) throw new Error("Invalid napi-rs declarations");
  assertMeaningfulDeclarationNames(text, "index.d.ts");
  // napi-rs can place a field's documentation after the preceding semicolon.
  // Inspect actual member boundaries, not comment-like text inside literals.
  const fieldComments = new Set();
  const visit = (node) => {
    if (ts.isPropertySignature(node) || ts.isPropertyDeclaration(node)) {
      for (const comment of ts.getTrailingCommentRanges(text, node.pos) ?? []) {
        if (text.startsWith("/**", comment.pos)) fieldComments.add(comment.pos);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  if (fieldComments.size) {
    const chunks = [];
    let offset = 0;
    for (const position of [...fieldComments].sort((left, right) => left - right)) {
      chunks.push(text.slice(offset, position), "\n");
      offset = position;
    }
    chunks.push(text.slice(offset));
    source = ts.createSourceFile("index.d.ts", chunks.join(""), ts.ScriptTarget.Latest, true);
  }
  const statements = source.statements.map((statement) => {
    if (!ts.isClassDeclaration(statement)) return statement;
    if (statement.heritageClauses?.length) {
      throw new Error("Native class inheritance requires an explicit identity review");
    }
    const members = [...statement.members];
    const identity = members.find((member) => member.name?.getText(source) === "__paleNativeHandle");
    if (identity) {
      if (!ts.isPropertyDeclaration(identity) || identity.type?.kind !== ts.SyntaxKind.VoidKeyword
        || !identity.modifiers?.some((modifier) => modifier.kind === ts.SyntaxKind.PrivateKeyword)) {
        throw new Error("Native identity declaration collides with a generated member");
      }
    } else {
      members.unshift(ts.factory.createPropertyDeclaration(
        [ts.factory.createModifier(ts.SyntaxKind.PrivateKeyword)], "__paleNativeHandle",
        undefined, ts.factory.createKeywordTypeNode(ts.SyntaxKind.VoidKeyword), undefined,
      ));
    }
    if (!members.some(ts.isConstructorDeclaration)) {
      members.push(ts.factory.createConstructorDeclaration(
        [ts.factory.createModifier(ts.SyntaxKind.PrivateKeyword)], [], undefined,
      ));
    }
    return ts.factory.updateClassDeclaration(statement, statement.modifiers, statement.name,
      statement.typeParameters, statement.heritageClauses, members);
  });
  return ts.createPrinter({ newLine: ts.NewLineKind.LineFeed }).printFile(
    ts.factory.updateSourceFile(source, statements),
  );
}
