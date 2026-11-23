package contract_codegen_test

import (
	"bytes"
	"go/ast"
	"go/parser"
	"go/token"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"testing"
)

var meaninglessGeneratedName = regexp.MustCompile(`\b(?:[A-Z][A-Za-z0-9]*V1[1-9][0-9]*|[A-Z][A-Za-z0-9]*Variant[0-9]+)\b`)
var numericGeneratedSuffix = regexp.MustCompile(`^(.+?)([1-9][0-9]*)$`)

func TestGeneratedContractsUseMeaningfulDeclarationNames(t *testing.T) {
	t.Parallel()

	root := filepath.Join("..", "..", "contracts", "types")
	err := filepath.WalkDir(root, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() || filepath.Ext(path) != ".go" {
			return nil
		}
		source, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		if name := meaninglessGeneratedName.Find(source); name != nil {
			t.Fatalf("%s contains meaningless generated declaration %q", path, name)
		}
		if counter, err := numericDeclarationCounter(source); err != nil {
			return err
		} else if counter != "" {
			t.Fatalf("%s contains a numeric generated declaration counter: %s", path, counter)
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}

func numericDeclarationCounter(source []byte) (string, error) {
	file, err := parser.ParseFile(token.NewFileSet(), "generated.go", source, 0)
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
			match := numericGeneratedSuffix.FindStringSubmatch(name)
			if len(match) == 0 {
				continue
			}
			if groups[match[1]] == nil {
				groups[match[1]] = make(map[string]struct{})
			}
			groups[match[1]][name] = struct{}{}
		}
	}
	for base, members := range groups {
		_, hasBase := declared[base]
		if !hasBase && len(members) == 1 {
			continue
		}
		names := make([]string, 0, len(members))
		for name := range members {
			names = append(names, name)
		}
		sort.Strings(names)
		return strings.Join(names, ", "), nil
	}
	return "", nil
}

func TestNumericDeclarationCounterRecognizesGenericCollisions(t *testing.T) {
	t.Parallel()

	bad := []byte("package fixture\ntype ProfileInspectionContext struct{}\ntype ProfileInspectionContext2 struct{}\n")
	if counter, err := numericDeclarationCounter(bad); err != nil || counter != "ProfileInspectionContext2" {
		t.Fatalf("got counter %q and error %v", counter, err)
	}
	good := []byte("package fixture\ntype SHA256DigestV1 string\ntype HTTP2PolicyV1 string\n")
	if counter, err := numericDeclarationCounter(good); err != nil || counter != "" {
		t.Fatalf("got counter %q and error %v", counter, err)
	}
	versionCounter := []byte("package fixture\ntype AttributableAnswerV114 struct{}\n")
	if name := meaninglessGeneratedName.Find(versionCounter); !bytes.Equal(name, []byte("AttributableAnswerV114")) {
		t.Fatalf("got collision name %q, want AttributableAnswerV114", name)
	}
}

func TestGeneratedContractsUseTaggedMeaningAndSharedReferences(t *testing.T) {
	t.Parallel()

	root := filepath.Join("..", "..", "contracts", "types")
	cases := []struct {
		path string
		want []string
	}{
		{
			path: filepath.Join(root, "genealogy_edge_v1", "contract.go"),
			want: []string{"GenealogyProofNativeStructuralV1", "GenealogyProofPaleConsequenceLineageV1", "GenealogyProofSemanticInferredV1"},
		},
		{
			path: filepath.Join(root, "workspace_derivation_receipt_v1", "contract.go"),
			want: []string{"WorkspaceDerivationOutputSourceContentV1", "WorkspaceDerivationSubjectWorkspaceV1"},
		},
	}
	for _, testCase := range cases {
		source, err := os.ReadFile(testCase.path)
		if err != nil {
			t.Fatal(err)
		}
		for _, name := range testCase.want {
			if !bytes.Contains(source, []byte("type "+name+" struct")) {
				t.Errorf("%s does not contain semantic declaration %s", testCase.path, name)
			}
		}
	}

	packagePath := filepath.Join(root, "agent_surface_package_v1", "contract.go")
	packageSource, err := os.ReadFile(packagePath)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(packageSource, []byte("License *PackageLicense")) {
		t.Fatalf("%s does not reuse the nullable referenced declaration", packagePath)
	}
	if got := bytes.Count(packageSource, []byte("type PackageLicense struct")); got != 1 {
		t.Fatalf("%s contains %d PackageLicense declarations, want one", packagePath, got)
	}
	if bytes.Contains(packageSource, []byte("type AgentSurfacePackageV1License")) {
		t.Fatalf("%s retained a property-named copy of PackageLicense", packagePath)
	}
}
