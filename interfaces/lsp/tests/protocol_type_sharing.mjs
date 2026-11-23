// Test generated TypeScript symbols, not textual suffixes or equal field lists.
import assert from "node:assert/strict";
import path from "node:path";

function moduleExports(checker, source, file) {
  const module = checker.getSymbolAtLocation(source);
  assert(module, `${file}: missing module symbol`);
  return checker.getExportsOfModule(module);
}

function declared(checker, exports, file, name) {
  const symbol = exports.find(value => value.name === name);
  assert(symbol, `${file}: missing ${name}`);
  assert.equal(symbol.declarations.length, 1, `${file}: ${name} has multiple declarations`);
  return { symbol, type: checker.getDeclaredTypeOfSymbol(symbol) };
}

function propertyType(checker, type, name) {
  const property = checker.getPropertyOfType(type, name);
  assert(property, `${checker.typeToString(type)}: missing ${name}`);
  return { property, type: checker.getTypeOfSymbolAtLocation(property, property.valueDeclaration) };
}

function taggedTypes(checker, union, discriminator) {
  return new Map(union.types.map(type => {
    const tag = propertyType(checker, type, discriminator).type;
    assert(tag.isStringLiteral(), `${checker.typeToString(type)}: ${discriminator} is not a string literal`);
    return [tag.value, type];
  }));
}

function requiredState(states, name) {
  const state = states.get(name);
  assert(state, `ConfigurationImprovementResponse: missing ${name} state`);
  return state;
}

function compile(ts, filename) {
  return ts.createProgram([filename], {
    strict: true,
    noEmit: true,
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ES2022,
  });
}

export function checkTypeSharing(requireTool, catalogDirectory, selectedDirectory) {
  const ts = requireTool("typescript");
  const observations = [];
  for (const [directory, file] of [
    [catalogDirectory, "Responses.ts"],
    [selectedDirectory, "ConfigurationImprovementResponse.ts"],
  ]) {
    const filename = path.join(directory, file);
    const program = compile(ts, filename);
    assert.deepEqual(
      ts.getPreEmitDiagnostics(program).map(value => ts.flattenDiagnosticMessageText(value.messageText, "\n")),
      [],
    );
    const checker = program.getTypeChecker();
    const source = program.getSourceFile(filename);
    assert(source, `${file}: missing source file`);
    const exports = moduleExports(checker, source, file);
    const responseDeclaration = declared(checker, exports, file, "ConfigurationImprovementResponse");
    const response = responseDeclaration.type;
    assert(response.isUnion(), `${file}: response is not a union`);
    assert(response.aliasSymbol === responseDeclaration.symbol, `${file}: redundant same-owner root alias`);
    const states = taggedTypes(checker, response, "state");

    const contexts = response.types.map(type => propertyType(checker, type, "context"));
    const expectedContext = declared(checker, exports, file, "ProfileInspectionContext").symbol;
    for (const { property, type } of contexts) {
      assert(type.symbol === expectedContext, `${file}: a response state cloned its context type`);
      assert.equal(
        ts.displayPartsToString(property.getDocumentationComment(checker)),
        "Ephemeral selection bound to the review.",
      );
    }
    assert.equal(
      exports.filter(symbol => /^ProfileInspectionContext\d+$/.test(symbol.name)).length,
      0,
      `${file}: numbered context declarations remain`,
    );

    if (file === "Responses.ts") {
      declared(checker, exports, file, "EffectiveProfileEntryDetail");
      assert.equal(
        exports.filter(symbol => /^EffectiveProfileEntry\d+$/.test(symbol.name)).length,
        0,
        `${file}: flattened profile detail retained a numeric declaration`,
      );
    }

    const coordinateStates = ["previewed", "details", "subject-details", "terminal"];
    const coordinates = coordinateStates.map(state =>
      propertyType(checker, requiredState(states, state), "coordinate").type);
    const coordinateSymbol = declared(checker, exports, file, "ImprovementPreviewCoordinate").symbol;
    assert(coordinates.every(type => type.symbol === coordinateSymbol), `${file}: coordinate type was cloned`);

    const previewStates = ["previewed", "terminal"];
    const previews = previewStates.map(state =>
      propertyType(checker, requiredState(states, state), "preview").type);
    const previewSymbol = declared(checker, exports, file, "ConfigurationImprovementPreviewProjection").symbol;
    assert(previews.every(type => type.symbol === previewSymbol), `${file}: preview type was cloned`);

    const explainedBaseline = propertyType(checker, requiredState(states, "explained"), "baseline").type;
    const coordinateBaseline = propertyType(checker, coordinates[0], "baseline").type;
    assert(
      explainedBaseline.symbol === coordinateBaseline.symbol
        && explainedBaseline.symbol === declared(checker, exports, file, "ExpectedProfileCoordinateOutput").symbol,
      `${file}: baseline type was cloned`,
    );
    observations.push({
      file,
      contextUses: contexts.length,
      coordinateUses: coordinateStates.length,
      previewUses: previewStates.length,
      baselineUses: 2,
      fieldDescriptionsPreserved: true,
    });
  }
  return observations;
}

export function checkRecursiveTypeSharing(requireTool, directory) {
  const ts = requireTool("typescript");
  const file = "ReferenceSharingProbe.ts";
  const filename = path.join(directory, file);
  const program = compile(ts, filename);
  assert.deepEqual(
    ts.getPreEmitDiagnostics(program).map(value => ts.flattenDiagnosticMessageText(value.messageText, "\n")),
    [],
  );
  const checker = program.getTypeChecker();
  const source = program.getSourceFile(filename);
  assert(source, `${file}: missing source file`);
  const exports = moduleExports(checker, source, file);
  const root = declared(checker, exports, file, "ReferenceSharingProbe");
  const recursive = propertyType(checker, root.type, "next").type;
  assert(recursive.isUnion());
  const nonNull = recursive.types.filter(value => !(value.flags & ts.TypeFlags.Null));
  assert.equal(nonNull.length, 1);
  assert(nonNull[0].symbol === root.symbol, "recursive reference created a second declaration");
  const first = propertyType(checker, root.type, "first").type;
  const second = propertyType(checker, root.type, "second").type;
  assert(first.symbol !== second.symbol, "equal fields merged distinct definition owners");
  assert.equal(first.symbol.name, "FirstContext");
  assert.equal(second.symbol.name, "SecondContext");
}
