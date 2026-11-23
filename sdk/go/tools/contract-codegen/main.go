// Command pale-go-contract-codegen projects the canonical Pale contract
// bundle into native Go data types.
//
// The command deliberately configures go-jsonschema in models-only mode. The
// generated package improves native authoring without acquiring structural or
// semantic validation authority from pale-sdk.
package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/format"
	"go/parser"
	"go/printer"
	"go/token"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strings"

	"github.com/atombender/go-jsonschema/pkg/generator"
	"github.com/atombender/go-jsonschema/pkg/schemas"
)

const (
	generatorName    = "github.com/atombender/go-jsonschema"
	generatorVersion = "v0.24.1"
	registryFileName = "agent-surface-schemas.v1.json"
	generatedNotice  = "// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.\n// SPDX-License-Identifier: Apache-2.0\n\n"
)

type options struct {
	bundle string
	check  bool
}

type registry struct {
	Registry             string                   `json:"registry"`
	SchemaManifestSHA256 string                   `json:"schema_manifest_sha256"`
	SchemaSetSHA256      string                   `json:"schema_set_sha256"`
	Schemas              map[string]registryEntry `json:"schemas"`
}

type registryEntry struct {
	Path         string `json:"path"`
	SchemaSHA256 string `json:"schema_sha256"`
	Lifecycle    struct {
		Role string `json:"role"`
	} `json:"lifecycle"`
}

type loadedSchema struct {
	family     string
	id         string
	path       string
	schema     *schemas.Schema
	objectSums []objectSum
}

type objectSum struct {
	name          string
	discriminator string
	variants      []objectSumVariant
}

type objectSumVariant struct {
	name  string
	value string
}

type bundleLoader struct {
	byLocation map[string]*schemas.Schema
}

func (loader *bundleLoader) Load(uri, _ string) (*schemas.Schema, error) {
	schema, ok := loader.byLocation[uri]
	if !ok {
		return nil, fmt.Errorf("schema reference %q is outside the admitted bundle", uri)
	}
	return schema, nil
}

type outputMember struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Size   int    `json:"size"`
}

type sourceCoordinate struct {
	Schema  string `json:"schema"`
	Version int    `json:"version"`
	Source  struct {
		RegistrySHA256       string `json:"registry_sha256"`
		SchemaManifestSHA256 string `json:"schema_manifest_sha256"`
		SchemaSetSHA256      string `json:"schema_set_sha256"`
		SchemaCount          int    `json:"schema_count"`
	} `json:"source"`
	Generator struct {
		Name                 string `json:"name"`
		Version              string `json:"version"`
		ImplementationSHA256 string `json:"implementation_sha256"`
		DependencyLockSHA256 string `json:"dependency_lock_sha256"`
		Options              struct {
			SchemaDraft       string   `json:"schema_draft"`
			OnlyModels        bool     `json:"only_models"`
			ScalarSumsAsEnums bool     `json:"scalar_sums_as_enums"`
			ClosedObjectSums  bool     `json:"closed_object_sums"`
			PatternMaps       bool     `json:"pattern_maps"`
			Tags              []string `json:"tags"`
		} `json:"options"`
	} `json:"generator"`
	OutputSet struct {
		AggregateSHA256 string         `json:"aggregate_sha256"`
		MemberCount     int            `json:"member_count"`
		Members         []outputMember `json:"members"`
	} `json:"output_set"`
}

func main() {
	var parsed options
	flag.StringVar(&parsed.bundle, "bundle", "", "canonical agent-surface.v1 bundle directory")
	flag.BoolVar(&parsed.check, "check", false, "verify that checked-in generated sources are fresh")
	flag.Parse()
	if parsed.bundle == "" || flag.NArg() != 0 {
		fatal(errors.New("usage: pale-go-contract-codegen [--check] --bundle <agent-surface.v1-directory>"))
	}
	if err := run(parsed); err != nil {
		fatal(err)
	}
}

func run(options options) error {
	moduleRoot, implementationBytes, err := generatorSource()
	if err != nil {
		return err
	}
	registryPath := filepath.Join(options.bundle, registryFileName)
	registryBytes, err := os.ReadFile(registryPath)
	if err != nil {
		return fmt.Errorf("read registry: %w", err)
	}
	var manifest registry
	if err := json.Unmarshal(registryBytes, &manifest); err != nil {
		return fmt.Errorf("decode registry: %w", err)
	}
	loaded, err := loadSchemas(options.bundle, manifest)
	if err != nil {
		return err
	}
	outputs, err := generateModels(loaded)
	if err != nil {
		return err
	}
	outputs["registry_generated.go"], err = generateRegistry(manifest)
	if err != nil {
		return err
	}
	lockBytes, err := os.ReadFile(filepath.Join(moduleRoot, "tools", "contract-codegen", "go.sum"))
	if err != nil {
		return fmt.Errorf("read dependency lock: %w", err)
	}
	coordinate, err := generateCoordinate(registryBytes, manifest, implementationBytes, lockBytes, outputs)
	if err != nil {
		return err
	}
	outputs["source-coordinate.json"] = coordinate
	return reconcile(
		filepath.Join(moduleRoot, "contracts"),
		outputs,
		options.check,
		len(manifest.Schemas),
		manifest.SchemaSetSHA256,
	)
}

func loadSchemas(bundle string, manifest registry) ([]loadedSchema, error) {
	families := sortedKeys(manifest.Schemas)
	loaded := make([]loadedSchema, 0, len(families))
	for _, family := range families {
		entry := manifest.Schemas[family]
		if filepath.IsAbs(entry.Path) || strings.Contains(entry.Path, "..") {
			return nil, fmt.Errorf("%s: registry path is not confined", family)
		}
		path := filepath.Join(bundle, filepath.FromSlash(entry.Path))
		schemaBytes, err := os.ReadFile(path)
		if err != nil {
			return nil, fmt.Errorf("%s: read schema: %w", family, err)
		}
		if sha256Hex(schemaBytes) != entry.SchemaSHA256 {
			return nil, fmt.Errorf("%s: registry digest is stale", family)
		}
		var header struct {
			ID     string `json:"$id"`
			Schema string `json:"$schema"`
		}
		if err := json.Unmarshal(schemaBytes, &header); err != nil {
			return nil, fmt.Errorf("%s: decode schema header: %w", family, err)
		}
		if header.Schema != "https://json-schema.org/draft/2020-12/schema" {
			return nil, fmt.Errorf("%s: schema is not Draft 2020-12", family)
		}
		projectionBytes, objectSums, err := projectGeneratorInput(schemaBytes, pascal(family))
		if err != nil {
			return nil, fmt.Errorf("%s: project generator input: %w", family, err)
		}
		parsed, err := schemas.FromJSONReader(bytes.NewReader(projectionBytes))
		if err != nil {
			return nil, fmt.Errorf("%s: decode generator schema: %w", family, err)
		}
		loaded = append(loaded, loadedSchema{
			family:     family,
			id:         header.ID,
			path:       path,
			schema:     parsed,
			objectSums: objectSums,
		})
	}
	return loaded, nil
}

// projectGeneratorInput lowers canonical sums into faithful models-only Go
// representations. Distinct scalar constants become native enums. Tagged
// object sums are inventoried before go-jsonschema sees its anyOf projection;
// generateModels later replaces the upstream first-branch merge with a closed
// typed union. Pale-owned validation remains authoritative for both forms.
func projectGeneratorInput(schemaBytes []byte, rootType string) ([]byte, []objectSum, error) {
	var value any
	if err := json.Unmarshal(schemaBytes, &value); err != nil {
		return nil, nil, err
	}
	if err := nameObjectSumVariants(value, rootType); err != nil {
		return nil, nil, err
	}
	var objectSums []objectSum
	if err := collectObjectSums(value, rootType, &objectSums); err != nil {
		return nil, nil, err
	}
	nameNullableReferenceDefinitions(value)
	normalizeScalarSums(value)
	exposeRootObjectSum(value)
	projected, err := json.Marshal(value)
	return projected, objectSums, err
}

// nameNullableReferenceDefinitions preserves the referenced definition's
// semantic name when go-jsonschema dereferences it from an optional property.
// Otherwise the generator can emit both a property-named copy and a numbered
// alias back to the same definition.
func nameNullableReferenceDefinitions(value any) {
	root, ok := value.(map[string]any)
	if !ok {
		return
	}
	definitions, ok := root["$defs"].(map[string]any)
	if !ok {
		return
	}
	var visit func(any)
	visit = func(value any) {
		switch value := value.(type) {
		case []any:
			for _, element := range value {
				visit(element)
			}
		case map[string]any:
			if reference, ok := nullableReference(value); ok && strings.HasPrefix(reference, "#/$defs/") {
				definitionName := strings.TrimPrefix(reference, "#/$defs/")
				if definition, ok := definitions[definitionName].(map[string]any); ok {
					if _, named := definition["title"]; !named {
						definition["title"] = definitionName
					}
				}
			}
			for _, element := range value {
				visit(element)
			}
		}
	}
	visit(root)
}

// nameObjectSumVariants gives every tagged object branch a stable semantic
// name before go-jsonschema sees it. Without these titles the upstream
// generator exposes source-order counters such as `StateV1_3`; those counters
// are not domain names and change when an earlier branch is inserted.
func nameObjectSumVariants(value any, owner string) error {
	object, ok := value.(map[string]any)
	if !ok {
		return nil
	}
	if definitions, ok := object["$defs"].(map[string]any); ok {
		for _, name := range sortedKeys(definitions) {
			definition := definitions[name]
			if err := nameObjectSumVariants(definition, name); err != nil {
				return err
			}
		}
	}
	for _, keyword := range []string{"oneOf", "anyOf"} {
		alternatives, ok := object[keyword].([]any)
		if !ok {
			continue
		}
		names := make(map[string]struct{}, len(alternatives))
		for _, alternative := range alternatives {
			branch, ok := alternative.(map[string]any)
			if !ok || branch["type"] != "object" {
				continue
			}
			discriminator, ok := objectBranchDiscriminator(branch)
			if !ok {
				continue
			}
			name := insertBeforeVersion(owner, pascal(discriminator))
			if _, duplicate := names[name]; duplicate {
				return fmt.Errorf("%s: repeated semantic object-branch name %q", owner, name)
			}
			names[name] = struct{}{}
			branch["title"] = name
			if err := nameObjectSumVariants(branch, name); err != nil {
				return err
			}
		}
	}
	if properties, ok := object["properties"].(map[string]any); ok {
		for _, property := range sortedKeys(properties) {
			schema := properties[property]
			if err := nameObjectSumVariants(schema, insertBeforeVersion(owner, pascal(property))); err != nil {
				return err
			}
		}
	}
	if items, ok := object["items"]; ok {
		if err := nameObjectSumVariants(items, insertBeforeVersion(owner, "Item")); err != nil {
			return err
		}
	}
	return nil
}

func objectBranchDiscriminator(branch map[string]any) (string, bool) {
	_, value, ok := objectBranchDiscriminatorField(branch)
	return value, ok
}

func objectBranchDiscriminatorField(branch map[string]any) (string, string, bool) {
	properties, ok := branch["properties"].(map[string]any)
	if !ok {
		return "", "", false
	}
	for _, key := range []string{
		"kind", "state", "phase", "source", "reason", "disposition", "origin",
		"basis", "status", "membership", "mode", "action", "selection", "scope",
		"truth_class", "visibility", "target", "outcome", "impact", "output",
		"change", "subject",
	} {
		property, ok := properties[key].(map[string]any)
		if !ok {
			continue
		}
		constant, ok := property["const"].(string)
		if ok && constant != "" {
			return key, constant, true
		}
	}
	return "", "", false
}

// collectObjectSums retains the semantic names and discriminator values that
// go-jsonschema loses when it merges object anyOf branches. Every all-object
// sum must be tagged: silently accepting an untagged merge would recreate the
// first-branch collapse this projection exists to prevent.
func collectObjectSums(value any, owner string, sums *[]objectSum) error {
	object, ok := value.(map[string]any)
	if !ok {
		return nil
	}
	if definitions, ok := object["$defs"].(map[string]any); ok {
		for _, name := range sortedKeys(definitions) {
			definition := definitions[name]
			if err := collectObjectSums(definition, name, sums); err != nil {
				return err
			}
		}
	}
	for _, keyword := range []string{"oneOf", "anyOf"} {
		alternatives, ok := object[keyword].([]any)
		if !ok || len(alternatives) == 0 {
			continue
		}
		allObjects := true
		variants := make([]objectSumVariant, 0, len(alternatives))
		discriminatorField := ""
		for _, alternative := range alternatives {
			branch, branchOK := alternative.(map[string]any)
			if !branchOK || branch["type"] != "object" {
				allObjects = false
				break
			}
			discriminator, value, tagged := objectBranchDiscriminatorField(branch)
			name, named := branch["title"].(string)
			if !tagged || !named || name == "" {
				return fmt.Errorf("%s: object sum branch lacks a stable tag or type name", owner)
			}
			if discriminatorField == "" {
				discriminatorField = discriminator
			} else if discriminator != discriminatorField {
				return fmt.Errorf("%s: object sum branches use different discriminators", owner)
			}
			variants = append(variants, objectSumVariant{name: name, value: value})
		}
		_, typed := object["type"]
		_, shaped := object["properties"]
		if allObjects && !typed && !shaped {
			if len(variants) < 2 {
				return fmt.Errorf("%s: object sum has fewer than two variants", owner)
			}
			*sums = append(*sums, objectSum{
				name:          owner,
				discriminator: discriminatorField,
				variants:      variants,
			})
		}
		for _, alternative := range alternatives {
			branch, ok := alternative.(map[string]any)
			if !ok {
				continue
			}
			branchOwner, _ := branch["title"].(string)
			if branchOwner == "" {
				branchOwner = owner
			}
			if err := collectObjectSums(branch, branchOwner, sums); err != nil {
				return err
			}
		}
	}
	if properties, ok := object["properties"].(map[string]any); ok {
		for _, property := range sortedKeys(properties) {
			schema := properties[property]
			if err := collectObjectSums(schema, insertBeforeVersion(owner, pascal(property)), sums); err != nil {
				return err
			}
		}
	}
	if items, ok := object["items"]; ok {
		return collectObjectSums(items, insertBeforeVersion(owner, "Item"), sums)
	}
	return nil
}

var versionSuffix = regexp.MustCompile(`^(.*?)(V[0-9]+)$`)

func insertBeforeVersion(name, qualifier string) string {
	parts := versionSuffix.FindStringSubmatch(name)
	if len(parts) == 3 {
		return parts[1] + qualifier + parts[2]
	}
	return name + qualifier
}

func normalizeScalarSums(value any) {
	switch value := value.(type) {
	case []any:
		for _, element := range value {
			normalizeScalarSums(element)
		}
	case map[string]any:
		for _, element := range value {
			normalizeScalarSums(element)
		}
		normalizePatternMap(value)
		if normalizeNullableReference(value) {
			return
		}
		alternatives, ok := value["oneOf"].([]any)
		if !ok || len(alternatives) == 0 {
			return
		}
		delete(value, "oneOf")
		if kind, constants, scalar := scalarEnum(alternatives); scalar {
			value["type"] = kind
			value["enum"] = constants
			return
		}
		if _, typed := value["type"]; typed {
			value["oneOf"] = alternatives
			return
		}
		if _, shaped := value["properties"]; shaped {
			value["oneOf"] = alternatives
			return
		}
		for _, alternative := range alternatives {
			branch, ok := alternative.(map[string]any)
			if !ok || branch["type"] != "object" {
				value["oneOf"] = alternatives
				return
			}
		}
		// go-jsonschema emits every named branch from anyOf. generateModels
		// replaces its lossy merged use-site with the inventoried closed union.
		value["anyOf"] = alternatives
	}
}

func scalarEnum(alternatives []any) (string, []any, bool) {
	kind := ""
	constants := make([]any, 0, len(alternatives))
	seen := make(map[string]struct{}, len(alternatives))
	for _, alternative := range alternatives {
		branch, ok := alternative.(map[string]any)
		if !ok || len(branch) != 2 {
			return "", nil, false
		}
		branchKind, kindOK := branch["type"].(string)
		constant, constantOK := branch["const"]
		if !kindOK || !constantOK || branchKind == "object" || branchKind == "array" || branchKind == "null" {
			return "", nil, false
		}
		if kind == "" {
			kind = branchKind
		} else if kind != branchKind {
			return "", nil, false
		}
		encoded, err := json.Marshal(constant)
		if err != nil {
			return "", nil, false
		}
		key := string(encoded)
		if _, duplicate := seen[key]; duplicate {
			return "", nil, false
		}
		seen[key] = struct{}{}
		constants = append(constants, constant)
	}
	return kind, constants, true
}

// normalizePatternMap retains the value type of a single regex-keyed map for
// native authoring. Key-pattern enforcement remains exclusively in pale-sdk.
func normalizePatternMap(value map[string]any) {
	if value["type"] != "object" || len(value) > 4 {
		return
	}
	patterns, ok := value["patternProperties"].(map[string]any)
	if !ok || len(patterns) != 1 || value["additionalProperties"] != false {
		return
	}
	for _, item := range patterns {
		value["additionalProperties"] = item
	}
	delete(value, "patternProperties")
}

// normalizeNullableReference gives the generator a native pointer projection
// while retaining the owning reference. Local references use the referenced
// definition's semantic name instead of creating a duplicate property-named
// declaration. The projection affects Go representation only: the canonical
// schema and pale-sdk still decide whether a value is valid.
func normalizeNullableReference(value map[string]any) bool {
	reference, ok := nullableReference(value)
	if !ok {
		return false
	}
	delete(value, "anyOf")
	value["$ref"] = reference
	value["goJSONSchema"] = map[string]any{"pointer": true}
	return true
}

func nullableReference(value map[string]any) (string, bool) {
	if len(value) != 1 {
		return "", false
	}
	alternatives, ok := value["anyOf"].([]any)
	if !ok || len(alternatives) != 2 {
		return "", false
	}
	var reference string
	hasNull := false
	for _, alternative := range alternatives {
		branch, ok := alternative.(map[string]any)
		if !ok || len(branch) != 1 {
			return "", false
		}
		if candidate, ok := branch["$ref"].(string); ok {
			reference = candidate
			continue
		}
		if kind, ok := branch["type"].(string); ok && kind == "null" {
			hasNull = true
			continue
		}
		return "", false
	}
	return reference, reference != "" && hasNull
}

func generateModels(loaded []loadedSchema) (map[string][]byte, error) {
	locations := make(map[string]*schemas.Schema, len(loaded)*2)
	mappings := make([]generator.SchemaMapping, 0, len(loaded))
	schemasByOutput := make(map[string]loadedSchema, len(loaded))
	for _, item := range loaded {
		packageName := familyPackage(item.family)
		outputName := filepath.ToSlash(filepath.Join("types", packageName, "contract.go"))
		locations[item.id] = item.schema
		locations[item.path] = item.schema
		schemasByOutput[outputName] = item
		mappings = append(mappings, generator.SchemaMapping{
			SchemaID:    item.id,
			PackageName: "github.com/getHarshOnline/pale/sdk/go/contracts/types/" + packageName,
			RootType:    pascal(item.family),
			OutputName:  outputName,
		})
	}
	projector, err := generator.New(generator.Config{
		SchemaMappings:      mappings,
		DefaultPackageName:  "github.com/getHarshOnline/pale/sdk/go/contracts/types/contracts",
		DefaultOutputName:   "types/contracts/contract.go",
		StructNameFromTitle: true,
		Capitalizations:     []string{"API", "ID", "IP", "JCS", "JSON", "SHA256", "URI", "URL", "UTF8"},
		Tags:                []string{"json"},
		OnlyModels:          true,
		Loader:              &bundleLoader{byLocation: locations},
	})
	if err != nil {
		return nil, fmt.Errorf("configure Go contract projection: %w", err)
	}
	for _, item := range loaded {
		if err := addFile(projector, item); err != nil {
			return nil, fmt.Errorf("%s: generate Go contract: %w", item.family, err)
		}
	}
	sources, err := projector.Sources()
	if err != nil {
		return nil, fmt.Errorf("render Go contract sources: %w", err)
	}
	outputs := make(map[string][]byte, len(sources))
	for path, source := range sources {
		source = stripUpstreamBanner(source)
		source = normalizeGeneratedNames(source)
		source, err = internDuplicateAliases(source)
		if err != nil {
			return nil, fmt.Errorf("intern duplicate declarations in %s: %w", path, err)
		}
		item := schemasByOutput[path]
		source, err = closeObjectSums(source, item.objectSums)
		if err != nil {
			return nil, fmt.Errorf("close object sums in %s: %w", path, err)
		}
		notice := generatedTypeNotice(path, item.family)
		formatted, err := format.Source(append([]byte(notice), source...))
		if err != nil {
			return nil, fmt.Errorf("format %s: %w", path, err)
		}
		outputs[filepath.ToSlash(path)] = formatted
	}
	if err := rejectMeaninglessCollisionNames(outputs); err != nil {
		return nil, err
	}
	return outputs, nil
}

// closeObjectSums replaces go-jsonschema's first-branch anyOf use-sites with
// closed, strongly typed tagged unions. Branch structs remain generated from
// the canonical schemas; the wrapper admits and emits exactly one known tag.
func closeObjectSums(source []byte, sums []objectSum) ([]byte, error) {
	if len(sums) == 0 {
		return source, nil
	}
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, "generated.go", source, parser.ParseComments)
	if err != nil {
		return nil, err
	}
	types := make(map[string]*ast.TypeSpec)
	var unionDeclarations strings.Builder
	for _, declaration := range file.Decls {
		generic, ok := declaration.(*ast.GenDecl)
		if !ok || generic.Tok != token.TYPE {
			continue
		}
		for _, specification := range generic.Specs {
			typeSpec := specification.(*ast.TypeSpec)
			types[typeSpec.Name.Name] = typeSpec
		}
	}
	for _, sum := range sums {
		resolved := sum
		for index, variant := range resolved.variants {
			actual, ok := resolveGeneratedTypeName(types, variant.name)
			if !ok {
				return nil, fmt.Errorf("object sum %s is missing variant %s", sum.name, variant.name)
			}
			resolved.variants[index].name = actual
		}
		resolved.name, err = removeVariantQualifier(
			resolved.variants[0].name,
			pascal(resolved.variants[0].value),
		)
		if err != nil {
			return nil, err
		}
		if types[resolved.name] != nil {
			removeTypeDeclaration(file, resolved.name)
		}
		first := resolved.variants[0].name
		for _, declaration := range file.Decls {
			generic, ok := declaration.(*ast.GenDecl)
			if !ok || generic.Tok != token.TYPE {
				continue
			}
			for _, specification := range generic.Specs {
				typeSpec := specification.(*ast.TypeSpec)
				ast.Inspect(typeSpec.Type, func(node ast.Node) bool {
					identifier, ok := node.(*ast.Ident)
					if ok && identifier.Name == first {
						identifier.Name = resolved.name
					}
					return true
				})
			}
		}
		unionDeclarations.WriteString(renderObjectSumDeclarations(resolved))
	}
	ensureImport(file, "encoding/json")
	ensureImport(file, "fmt")
	var rendered bytes.Buffer
	if err := printer.Fprint(&rendered, fileSet, file); err != nil {
		return nil, err
	}
	rendered.WriteString("\n")
	rendered.WriteString(unionDeclarations.String())
	return rendered.Bytes(), nil
}

func removeTypeDeclaration(file *ast.File, name string) {
	declarations := file.Decls[:0]
	for _, declaration := range file.Decls {
		generic, ok := declaration.(*ast.GenDecl)
		if !ok || generic.Tok != token.TYPE {
			declarations = append(declarations, declaration)
			continue
		}
		specifications := generic.Specs[:0]
		for _, specification := range generic.Specs {
			if specification.(*ast.TypeSpec).Name.Name != name {
				specifications = append(specifications, specification)
			}
		}
		generic.Specs = specifications
		if len(specifications) != 0 {
			declarations = append(declarations, declaration)
		}
	}
	file.Decls = declarations
}

func resolveGeneratedTypeName(types map[string]*ast.TypeSpec, requested string) (string, bool) {
	for name := range types {
		if strings.EqualFold(name, requested) {
			return name, true
		}
	}
	return "", false
}

func removeVariantQualifier(name, qualifier string) (string, error) {
	parts := versionSuffix.FindStringSubmatch(name)
	if len(parts) == 3 && len(parts[1]) >= len(qualifier) && strings.EqualFold(parts[1][len(parts[1])-len(qualifier):], qualifier) {
		return parts[1][:len(parts[1])-len(qualifier)] + parts[2], nil
	}
	if len(name) < len(qualifier) || !strings.EqualFold(name[len(name)-len(qualifier):], qualifier) {
		return "", fmt.Errorf("variant type %s does not end in semantic qualifier %s", name, qualifier)
	}
	return name[:len(name)-len(qualifier)], nil
}

func renderObjectSumDeclarations(sum objectSum) string {
	var source strings.Builder
	fmt.Fprintf(&source, "type %s struct {\n", sum.name)
	for _, variant := range sum.variants {
		fmt.Fprintf(&source, "\t%s *%s `json:\"-\"`\n", pascal(variant.value), variant.name)
	}
	source.WriteString("}\n\n")
	fmt.Fprintf(&source, "func (value %s) MarshalJSON() ([]byte, error) {\n", sum.name)
	source.WriteString("\tvar encoded []byte\n\tselected := 0\n")
	for _, variant := range sum.variants {
		field := pascal(variant.value)
		discriminator := pascal(sum.discriminator)
		fmt.Fprintf(&source, "\tif value.%s != nil {\n", field)
		fmt.Fprintf(&source, "\t\tif value.%s.%s != %q { return nil, fmt.Errorf(%q, value.%s.%s) }\n", field, discriminator, variant.value, sum.name+": selected variant has discriminator %q", field, discriminator)
		fmt.Fprintf(&source, "\t\tcandidate, err := json.Marshal(value.%s)\n", field)
		source.WriteString("\t\tif err != nil { return nil, err }\n\t\tencoded = candidate\n\t\tselected++\n\t}\n")
	}
	fmt.Fprintf(&source, "\tif selected != 1 { return nil, fmt.Errorf(%q, selected) }\n", sum.name+": expected exactly one variant, got %d")
	source.WriteString("\treturn encoded, nil\n}\n\n")
	fmt.Fprintf(&source, "func (value *%s) UnmarshalJSON(data []byte) error {\n", sum.name)
	fmt.Fprintf(&source, "\tvar discriminator struct { %s string `json:\"%s\"` }\n", pascal(sum.discriminator), sum.discriminator)
	source.WriteString("\tif err := json.Unmarshal(data, &discriminator); err != nil { return err }\n")
	fmt.Fprintf(&source, "\t*value = %s{}\n", sum.name)
	fmt.Fprintf(&source, "\tswitch discriminator.%s {\n", pascal(sum.discriminator))
	for _, variant := range sum.variants {
		field := pascal(variant.value)
		fmt.Fprintf(&source, "\tcase %q:\n\t\tvar variant %s\n", variant.value, variant.name)
		source.WriteString("\t\tif err := json.Unmarshal(data, &variant); err != nil { return err }\n")
		fmt.Fprintf(&source, "\t\tvalue.%s = &variant\n", field)
	}
	fmt.Fprintf(&source, "\tdefault:\n\t\treturn fmt.Errorf(%q, discriminator.%s)\n\t}\n", sum.name+": unknown discriminator %q", pascal(sum.discriminator))
	source.WriteString("\treturn nil\n}\n\n")
	return source.String()
}

func ensureImport(file *ast.File, path string) {
	quoted := fmt.Sprintf("%q", path)
	for _, declaration := range file.Decls {
		generic, ok := declaration.(*ast.GenDecl)
		if !ok || generic.Tok != token.IMPORT {
			continue
		}
		for _, specification := range generic.Specs {
			if specification.(*ast.ImportSpec).Path.Value == quoted {
				return
			}
		}
		generic.Specs = append(generic.Specs, &ast.ImportSpec{Path: &ast.BasicLit{Kind: token.STRING, Value: quoted}})
		generic.Lparen = generic.Pos()
		return
	}
	declaration := &ast.GenDecl{
		Tok:   token.IMPORT,
		Specs: []ast.Spec{&ast.ImportSpec{Path: &ast.BasicLit{Kind: token.STRING, Value: quoted}}},
	}
	file.Decls = append([]ast.Decl{declaration}, file.Decls...)
}

var numericVariant = regexp.MustCompile(`\b([A-Za-z][A-Za-z0-9]*)_([0-9]+)\b`)
var numberedVariantName = regexp.MustCompile(`^([A-Z][A-Za-z0-9]*)Variant[0-9]+$`)
var meaninglessCollisionName = regexp.MustCompile(`\b(?:[A-Z][A-Za-z0-9]*V1[1-9][0-9]*|[A-Z][A-Za-z0-9]*Variant[0-9]+)\b`)
var numericDeclarationSuffix = regexp.MustCompile(`^(.+?)([1-9][0-9]*)$`)
var declarationVersionSegment = regexp.MustCompile(`V[1-9][0-9]*`)

func normalizeGeneratedNames(source []byte) []byte {
	source = bytes.ReplaceAll(source, []byte("interface{}"), []byte("any"))
	source = numericVariant.ReplaceAll(source, []byte(`${1}Variant${2}`))
	lines := bytes.Split(source, []byte{'\n'})
	for index, line := range lines {
		fields := bytes.Fields(line)
		if len(fields) < 4 || !bytes.Equal(fields[0], []byte("const")) {
			continue
		}
		prefix := append(append([]byte(nil), fields[2]...), '_')
		if bytes.HasPrefix(fields[1], prefix) {
			lines[index] = bytes.Replace(line, prefix, fields[2], 1)
		}
	}
	return bytes.Join(lines, []byte{'\n'})
}

// internDuplicateAliases removes the upstream generator's redundant shape for
// a nullable local reference. The generator occasionally emits all three of a
// property-named declaration, the referenced declaration, and a numbered alias
// from the first back to the second. We only intern the property declaration
// when the alias establishes the target and both declaration shapes are exact.
func internDuplicateAliases(source []byte) ([]byte, error) {
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, "generated.go", source, parser.ParseComments)
	if err != nil {
		return nil, err
	}
	types := make(map[string]*ast.TypeSpec)
	for _, declaration := range file.Decls {
		generic, ok := declaration.(*ast.GenDecl)
		if !ok || generic.Tok != token.TYPE {
			continue
		}
		for _, specification := range generic.Specs {
			typeSpec, ok := specification.(*ast.TypeSpec)
			if ok {
				types[typeSpec.Name.Name] = typeSpec
			}
		}
	}
	replacements := make(map[string]string)
	aliases := make(map[string]struct{})
	for name, alias := range types {
		match := numberedVariantName.FindStringSubmatch(name)
		target, targetOK := alias.Type.(*ast.Ident)
		if len(match) != 2 || !alias.Assign.IsValid() || !targetOK {
			continue
		}
		base := types[match[1]]
		semantic := types[target.Name]
		if base == nil || semantic == nil || astTypeKey(fileSet, base.Type) != astTypeKey(fileSet, semantic.Type) {
			continue
		}
		replacements[base.Name.Name] = semantic.Name.Name
		aliases[name] = struct{}{}
	}
	if len(replacements) == 0 {
		return source, nil
	}
	declarationNames := make(map[*ast.Ident]struct{}, len(types))
	for _, typeSpec := range types {
		declarationNames[typeSpec.Name] = struct{}{}
	}
	ast.Inspect(file, func(node ast.Node) bool {
		identifier, ok := node.(*ast.Ident)
		if !ok {
			return true
		}
		if _, declaration := declarationNames[identifier]; declaration {
			return true
		}
		if replacement, ok := replacements[identifier.Name]; ok {
			identifier.Name = replacement
		}
		return true
	})
	type sourceSpan struct {
		start token.Pos
		end   token.Pos
	}
	removedSpans := make([]sourceSpan, 0, len(replacements)+len(aliases))
	keptDeclarations := file.Decls[:0]
	for _, declaration := range file.Decls {
		generic, ok := declaration.(*ast.GenDecl)
		if !ok || generic.Tok != token.TYPE {
			keptDeclarations = append(keptDeclarations, declaration)
			continue
		}
		kept := generic.Specs[:0]
		for _, specification := range generic.Specs {
			typeSpec := specification.(*ast.TypeSpec)
			_, removeBase := replacements[typeSpec.Name.Name]
			_, removeAlias := aliases[typeSpec.Name.Name]
			if !removeBase && !removeAlias {
				kept = append(kept, specification)
			} else {
				removedSpans = append(removedSpans, sourceSpan{start: typeSpec.Pos(), end: typeSpec.End()})
			}
		}
		generic.Specs = kept
		if len(kept) != 0 {
			keptDeclarations = append(keptDeclarations, declaration)
		}
	}
	file.Decls = keptDeclarations
	keptComments := file.Comments[:0]
	for _, comments := range file.Comments {
		removed := false
		for _, span := range removedSpans {
			if comments.Pos() >= span.start && comments.End() <= span.end {
				removed = true
				break
			}
		}
		if !removed {
			keptComments = append(keptComments, comments)
		}
	}
	file.Comments = keptComments
	var rendered bytes.Buffer
	if err := printer.Fprint(&rendered, fileSet, file); err != nil {
		return nil, err
	}
	return rendered.Bytes(), nil
}

func astTypeKey(fileSet *token.FileSet, expression ast.Expr) string {
	var rendered bytes.Buffer
	_ = printer.Fprint(&rendered, fileSet, expression)
	return rendered.String()
}

func rejectMeaninglessCollisionNames(outputs map[string][]byte) error {
	for _, path := range sortedKeys(outputs) {
		if name := meaninglessCollisionName.Find(outputs[path]); name != nil {
			return fmt.Errorf("%s: generated declaration retained meaningless collision name %q", path, name)
		}
		counter, err := numericDeclarationCounter(outputs[path])
		if err != nil {
			return fmt.Errorf("%s: inspect generated declarations: %w", path, err)
		}
		if counter != "" {
			return fmt.Errorf("%s: generated declarations use a numeric collision counter: %s", path, counter)
		}
		if name, err := repeatedVersionDeclarationName(outputs[path]); err != nil {
			return fmt.Errorf("%s: inspect generated declarations: %w", path, err)
		} else if name != "" {
			return fmt.Errorf("%s: generated declaration repeats a version segment: %s", path, name)
		}
	}
	return nil
}

func numericDeclarationCounter(source []byte) (string, error) {
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, "generated.go", source, 0)
	if err != nil {
		return "", err
	}
	declared := make(map[string]struct{})
	groups := make(map[string]map[string]struct{})
	for _, declaration := range file.Decls {
		generic, ok := declaration.(*ast.GenDecl)
		if !ok || generic.Tok != token.TYPE {
			continue
		}
		for _, specification := range generic.Specs {
			name := specification.(*ast.TypeSpec).Name.Name
			declared[name] = struct{}{}
			match := numericDeclarationSuffix.FindStringSubmatch(name)
			if len(match) == 0 {
				continue
			}
			if groups[match[1]] == nil {
				groups[match[1]] = make(map[string]struct{})
			}
			groups[match[1]][name] = struct{}{}
		}
	}
	for _, base := range sortedKeys(groups) {
		members := groups[base]
		_, hasBase := declared[base]
		if hasBase || len(members) > 1 {
			return strings.Join(sortedKeys(members), ", "), nil
		}
	}
	return "", nil
}

func repeatedVersionDeclarationName(source []byte) (string, error) {
	file, err := parser.ParseFile(token.NewFileSet(), "generated.go", source, 0)
	if err != nil {
		return "", err
	}
	names := make([]string, 0)
	for _, declaration := range file.Decls {
		generic, ok := declaration.(*ast.GenDecl)
		if !ok || generic.Tok != token.TYPE {
			continue
		}
		for _, specification := range generic.Specs {
			names = append(names, specification.(*ast.TypeSpec).Name.Name)
		}
	}
	sort.Strings(names)
	for _, name := range names {
		if len(declarationVersionSegment.FindAllString(name, -1)) > 1 {
			return name, nil
		}
	}
	return "", nil
}

func generatedTypeNotice(path, family string) string {
	packageName := filepath.Base(filepath.Dir(path))
	return fmt.Sprintf(
		"%s// Package %s contains the generated %s data model.\n",
		generatedNotice,
		packageName,
		family,
	)
}

func addFile(projector *generator.Generator, item loadedSchema) (err error) {
	defer func() {
		if recovered := recover(); recovered != nil {
			err = fmt.Errorf("generator panic: %v", recovered)
		}
	}()
	return projector.AddFile(item.path, item.schema)
}

func stripUpstreamBanner(source []byte) []byte {
	const marker = "// Code generated by github.com/atombender/go-jsonschema, DO NOT EDIT.\n\n"
	return bytes.TrimPrefix(source, []byte(marker))
}

func generateRegistry(manifest registry) ([]byte, error) {
	families := sortedKeys(manifest.Schemas)
	authored := make([]string, 0, len(families))
	var source strings.Builder
	source.WriteString(generatedNotice)
	source.WriteString("package contracts\n\n")
	fmt.Fprintf(&source, "const RegistryID = %q\n", manifest.Registry)
	fmt.Fprintf(&source, "const SchemaManifestSHA256 = %q\n", manifest.SchemaManifestSHA256)
	fmt.Fprintf(&source, "const SchemaSetSHA256 = %q\n\n", manifest.SchemaSetSHA256)
	source.WriteString("var schemaFamilies = [...]string{\n")
	for _, family := range families {
		fmt.Fprintf(&source, "\t%q,\n", family)
		if manifest.Schemas[family].Lifecycle.Role == "authored_input" {
			authored = append(authored, family)
		}
	}
	source.WriteString("}\n\nvar authoredInputFamilies = [...]string{\n")
	for _, family := range authored {
		fmt.Fprintf(&source, "\t%q,\n", family)
	}
	source.WriteString("}\n\nvar contractIdentities = map[string]string{\n")
	for _, family := range families {
		fmt.Fprintf(&source, "\t%q: %q,\n", family, manifest.Schemas[family].SchemaSHA256)
	}
	source.WriteString("}\n\n")
	source.WriteString("// SchemaFamilies returns the exact active families in deterministic order.\n")
	source.WriteString("func SchemaFamilies() []string { return append([]string(nil), schemaFamilies[:]...) }\n\n")
	source.WriteString("// AuthoredInputFamilies returns the families eligible for authored input.\n")
	source.WriteString("func AuthoredInputFamilies() []string { return append([]string(nil), authoredInputFamilies[:]...) }\n\n")
	source.WriteString("// ContractIdentity returns the canonical schema digest for family.\n")
	source.WriteString("func ContractIdentity(family string) (string, bool) { digest, ok := contractIdentities[family]; return digest, ok }\n")
	formatted, err := format.Source([]byte(source.String()))
	if err != nil {
		return nil, fmt.Errorf("format generated registry: %w", err)
	}
	return formatted, nil
}

func generateCoordinate(
	registryBytes []byte,
	manifest registry,
	implementationBytes []byte,
	lockBytes []byte,
	outputs map[string][]byte,
) ([]byte, error) {
	paths := sortedKeys(outputs)
	members := make([]outputMember, 0, len(paths))
	var preimage bytes.Buffer
	for _, path := range paths {
		member := outputMember{Path: path, SHA256: sha256Hex(outputs[path]), Size: len(outputs[path])}
		members = append(members, member)
		fmt.Fprintf(&preimage, "%s\x00%s\x00%d\n", member.Path, member.SHA256, member.Size)
	}
	var coordinate sourceCoordinate
	coordinate.Schema = "pale.go-contract-source-coordinate.v1"
	coordinate.Version = 1
	coordinate.Source.RegistrySHA256 = sha256Hex(registryBytes)
	coordinate.Source.SchemaManifestSHA256 = manifest.SchemaManifestSHA256
	coordinate.Source.SchemaSetSHA256 = manifest.SchemaSetSHA256
	coordinate.Source.SchemaCount = len(manifest.Schemas)
	coordinate.Generator.Name = generatorName
	coordinate.Generator.Version = generatorVersion
	coordinate.Generator.ImplementationSHA256 = sha256Hex(implementationBytes)
	coordinate.Generator.DependencyLockSHA256 = sha256Hex(lockBytes)
	coordinate.Generator.Options.SchemaDraft = "2020-12"
	coordinate.Generator.Options.OnlyModels = true
	coordinate.Generator.Options.ScalarSumsAsEnums = true
	coordinate.Generator.Options.ClosedObjectSums = true
	coordinate.Generator.Options.PatternMaps = true
	coordinate.Generator.Options.Tags = []string{"json"}
	coordinate.OutputSet.AggregateSHA256 = sha256Hex(preimage.Bytes())
	coordinate.OutputSet.MemberCount = len(members)
	coordinate.OutputSet.Members = members
	encoded, err := json.MarshalIndent(coordinate, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("encode generated source coordinate: %w", err)
	}
	return append(encoded, '\n'), nil
}

func reconcile(
	directory string,
	outputs map[string][]byte,
	check bool,
	schemaCount int,
	schemaSetSHA256 string,
) error {
	if check {
		observed, err := generatedFiles(directory)
		if err != nil {
			return err
		}
		expected := sortedKeys(outputs)
		if !equalStrings(observed, expected) {
			return errors.New("generated contract file inventory is stale")
		}
		for path, expectedBytes := range outputs {
			actual, err := os.ReadFile(filepath.Join(directory, filepath.FromSlash(path)))
			if err != nil {
				return fmt.Errorf("read generated %s: %w", path, err)
			}
			if !bytes.Equal(actual, expectedBytes) {
				return fmt.Errorf("%s is stale", path)
			}
		}
		fmt.Printf("fresh: %d schemas at %s\n", schemaCount, schemaSetSHA256)
		return nil
	}
	if err := os.RemoveAll(filepath.Join(directory, "types")); err != nil {
		return fmt.Errorf("replace generated type directories: %w", err)
	}
	for _, path := range []string{"registry_generated.go", "source-coordinate.json"} {
		if err := os.Remove(filepath.Join(directory, path)); err != nil && !errors.Is(err, os.ErrNotExist) {
			return fmt.Errorf("replace generated %s: %w", path, err)
		}
	}
	for path, content := range outputs {
		target := filepath.Join(directory, filepath.FromSlash(path))
		if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
			return fmt.Errorf("create generated directory: %w", err)
		}
		if err := os.WriteFile(target, content, 0o644); err != nil {
			return fmt.Errorf("write generated %s: %w", path, err)
		}
	}
	fmt.Printf("generated: %d schemas at %s\n", schemaCount, schemaSetSHA256)
	return nil
}

func generatedFiles(directory string) ([]string, error) {
	paths := make([]string, 0)
	for _, name := range []string{"registry_generated.go", "source-coordinate.json"} {
		if _, err := os.Stat(filepath.Join(directory, name)); err == nil {
			paths = append(paths, name)
		} else if !errors.Is(err, os.ErrNotExist) {
			return nil, fmt.Errorf("inventory generated %s: %w", name, err)
		}
	}
	err := filepath.WalkDir(filepath.Join(directory, "types"), func(path string, entry os.DirEntry, err error) error {
		if err != nil {
			if errors.Is(err, os.ErrNotExist) {
				return nil
			}
			return err
		}
		if entry.IsDir() {
			return nil
		}
		relative, err := filepath.Rel(directory, path)
		if err != nil {
			return err
		}
		paths = append(paths, filepath.ToSlash(relative))
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("inventory generated contracts: %w", err)
	}
	sort.Strings(paths)
	return paths, nil
}

func generatorSource() (string, []byte, error) {
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		return "", nil, errors.New("resolve generator source path")
	}
	sourceDirectory := filepath.Dir(sourceFile)
	entries, err := os.ReadDir(sourceDirectory)
	if err != nil {
		return "", nil, fmt.Errorf("inventory generator implementation: %w", err)
	}
	var implementation bytes.Buffer
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != ".go" || strings.HasSuffix(entry.Name(), "_test.go") {
			continue
		}
		content, err := os.ReadFile(filepath.Join(sourceDirectory, entry.Name()))
		if err != nil {
			return "", nil, fmt.Errorf("read generator implementation %s: %w", entry.Name(), err)
		}
		fmt.Fprintf(&implementation, "%s\x00", entry.Name())
		implementation.Write(content)
		implementation.WriteByte('\n')
	}
	root := filepath.Dir(filepath.Dir(filepath.Dir(sourceFile)))
	return root, implementation.Bytes(), nil
}

func sortedKeys[V any](values map[string]V) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

func pascal(value string) string {
	parts := strings.FieldsFunc(value, func(character rune) bool {
		return character < '0' || character > '9' && character < 'A' || character > 'Z' && character < 'a' || character > 'z'
	})
	for index, part := range parts {
		parts[index] = strings.ToUpper(part[:1]) + part[1:]
	}
	return strings.Join(parts, "")
}

func familyPackage(value string) string {
	return strings.Trim(strings.Map(func(character rune) rune {
		if character >= 'A' && character <= 'Z' {
			return character + ('a' - 'A')
		}
		if character >= 'a' && character <= 'z' || character >= '0' && character <= '9' {
			return character
		}
		return '_'
	}, value), "_")
}

func sha256Hex(value []byte) string {
	digest := sha256.Sum256(value)
	return hex.EncodeToString(digest[:])
}

func equalStrings(left, right []string) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if left[index] != right[index] {
			return false
		}
	}
	return true
}

func fatal(err error) {
	fmt.Fprintf(os.Stderr, "pale-go-contract-codegen: %v\n", err)
	os.Exit(1)
}
