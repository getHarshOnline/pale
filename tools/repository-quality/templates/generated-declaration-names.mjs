import ts from "typescript";

const knownCollisionName = /^(?:[A-Z][A-Za-z0-9]*V1[1-9][0-9]*|[A-Z][A-Za-z0-9]*Variant[0-9]+)$/;
const numericSuffix = /^(.+?)([1-9][0-9]*)$/;
const versionSegment = /V[1-9][0-9]*/g;

function declarationNames(source) {
  return source.statements.flatMap((statement) => {
    const named = ts.isClassDeclaration(statement)
      || ts.isEnumDeclaration(statement)
      || ts.isInterfaceDeclaration(statement)
      || ts.isTypeAliasDeclaration(statement);
    return named && statement.name ? [statement.name.text] : [];
  });
}

/** Reject declaration names produced by traversal or collision mechanics. */
export function assertMeaningfulDeclarationNames(text, file) {
  const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
  if (source.parseDiagnostics.length !== 0) {
    throw new Error(`${file}: generated declaration is not valid TypeScript`);
  }
  const names = declarationNames(source);
  const known = names.find((name) => knownCollisionName.test(name));
  if (known !== undefined) {
    throw new Error(`${file}: generated declaration retained meaningless collision name ${known}`);
  }
  const repeatedVersion = names.find((name) => (name.match(versionSegment) ?? []).length > 1);
  if (repeatedVersion !== undefined) {
    throw new Error(`${file}: generated declaration repeats a version segment: ${repeatedVersion}`);
  }

  const versionedOwners = names.filter((name) => /V[1-9][0-9]*$/.test(name));
  const redundantOwner = names.find((name) => versionedOwners.some((owner) => (
    name !== owner
      && name.startsWith(owner)
      && /^[A-Z]/.test(name.slice(owner.length))
  )));
  if (redundantOwner !== undefined) {
    throw new Error(`${file}: generated declaration repeats its owning declaration: ${redundantOwner}`);
  }

  const declared = new Set(names);
  const suffixGroups = new Map();
  for (const name of names) {
    const match = name.match(numericSuffix);
    if (match === null) continue;
    const members = suffixGroups.get(match[1]) ?? new Set();
    members.add(name);
    suffixGroups.set(match[1], members);
  }
  for (const [base, members] of suffixGroups) {
    if (declared.has(base) || members.size > 1) {
      throw new Error(
        `${file}: generated declarations use a numeric collision counter: ${[...members].sort().join(", ")}`,
      );
    }
  }
}
