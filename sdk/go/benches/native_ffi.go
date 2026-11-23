// Command native_ffi characterizes absolute calls through the Go bridge.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"
	"time"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

const (
	smokeSamples          = 5
	characterizationRuns  = 201
	maximumCoordinateSize = 512
)

type configuration struct {
	smoke      bool
	samples    int
	source     string
	run        string
	hardware   string
	fixtureSHA string
}

type benchmarkCase struct {
	identifier    string
	requestBytes  int
	responseBytes int
	operation     func()
	release       func()
}

type bundleFixture struct {
	MemberCount          uint32 `json:"member_count"`
	SchemaManifestSHA256 string `json:"schema_manifest_sha256"`
	SchemaSetSHA256      string `json:"schema_set_sha256"`
}

type contractFixture struct {
	Schema struct {
		RegistryID   string `json:"registry_id"`
		SchemaSHA256 string `json:"schema_sha256"`
	} `json:"schema"`
	CanonicalDiagnostic string `json:"canonical_diagnostic"`
}

type portableFixture struct {
	RootManifest   string   `json:"root_manifest"`
	DocumentLabels []string `json:"document_labels"`
	Compilation    struct {
		WorkspaceID               string `json:"workspace_id"`
		PrincipalUniverseDigest   string `json:"principal_universe_digest"`
		SemanticIRDigest          string `json:"semantic_ir_digest"`
		AdmissionEvidenceDigest   string `json:"admission_evidence_digest"`
		SourceDocumentCount       uint32 `json:"source_document_count"`
		DerivationSourceCount     uint32 `json:"derivation_source_count"`
		DerivationNodeCount       uint32 `json:"derivation_node_count"`
		AdmissionEvidenceCount    uint32 `json:"admission_evidence_count"`
		AdoptedReleaseCount       uint32 `json:"adopted_release_count"`
		PackageReleaseCount       uint32 `json:"package_release_count"`
		NormativePackReleaseCount uint32 `json:"normative_pack_release_count"`
	} `json:"compilation"`
	FirstImpact struct {
		NextOffset            uint32 `json:"next_offset"`
		SourceID              string `json:"source_id"`
		State                 string `json:"state"`
		ContributingRootCount uint32 `json:"contributing_root_count"`
	} `json:"first_impact"`
}

func must[T any](value T, err error) T {
	if err != nil {
		panic(err)
	}
	return value
}

func readJSON[T any](path string) T {
	var value T
	if err := json.Unmarshal(must(os.ReadFile(path)), &value); err != nil {
		panic(err)
	}
	return value
}

func coordinate(name string, smoke bool, fallback string) string {
	value := os.Getenv(name)
	if value == "" && smoke {
		value = fallback
	}
	if value == "" {
		panic(name + " is required for a characterized run")
	}
	if len(value) > maximumCoordinateSize {
		panic(name + " exceeds 512 bytes")
	}
	if strings.ContainsAny(value, "\t\r\n") {
		panic(name + " must be one TSV-safe line")
	}
	return value
}

func nearestRank(sorted []int64, rank int) int64 {
	index := ((rank*len(sorted) + 99) / 100) - 1
	return sorted[index]
}

func measure(samples int, operation, release func()) (int64, int64, string, int64) {
	for warmup := 0; warmup < 3; warmup++ {
		operation()
		if release != nil {
			release()
		}
	}
	observations := make([]int64, 0, samples)
	for sample := 0; sample < samples; sample++ {
		started := time.Now()
		operation()
		observations = append(observations, time.Since(started).Nanoseconds())
		// Retained-result disposal is outside the measured call latency.
		if release != nil {
			release()
		}
	}
	sort.Slice(observations, func(left, right int) bool {
		return observations[left] < observations[right]
	})
	p99 := ""
	if samples >= 101 {
		p99 = fmt.Sprint(nearestRank(observations, 99))
	}
	return nearestRank(observations, 50), nearestRank(observations, 95), p99, observations[len(observations)-1]
}

func fixtureDigest(root string, portable portableFixture) string {
	members := []string{
		"bundle-identity.json",
		"contract-diagnostic.json",
		"portable-journey.json",
	}
	for _, label := range portable.DocumentLabels {
		members = append(members, filepath.Join("portable", label))
	}
	digest := sha256.New()
	digest.Write([]byte("pale.cross-leaf-ffi-benchmark-fixture.v1\x00"))
	for _, member := range members {
		digest.Write([]byte(filepath.ToSlash(member)))
		digest.Write([]byte{0})
		digest.Write(must(os.ReadFile(filepath.Join(root, member))))
		digest.Write([]byte{0})
	}
	return hex.EncodeToString(digest.Sum(nil))
}

func proveLaws(client *pale.Client, fixture bundleFixture, contract contractFixture, portable portableFixture, request pale.PortableWorkspaceRequest) (*pale.WorkspaceComparison, []byte) {
	identity := must(client.ContractBundleIdentity())
	var actualIdentity struct {
		MemberCount          uint32 `json:"memberCount"`
		SchemaManifestSHA256 string `json:"schemaManifestSha256"`
		SchemaSetSHA256      string `json:"schemaSetSha256"`
	}
	if err := json.Unmarshal(identity.Bytes, &actualIdentity); err != nil {
		panic(err)
	}
	if actualIdentity.MemberCount != fixture.MemberCount ||
		actualIdentity.SchemaManifestSHA256 != fixture.SchemaManifestSHA256 ||
		actualIdentity.SchemaSetSHA256 != fixture.SchemaSetSHA256 {
		panic("bundle identity drifted from the shared fixture")
	}

	diagnostic := []byte(contract.CanonicalDiagnostic)
	admitted := must(client.AdmitDiagnostic(diagnostic))
	if !reflect.DeepEqual(admitted.Bytes, diagnostic) {
		panic("diagnostic admission changed canonical bytes")
	}
	validated := must(client.ValidateContractDocument(contract.Schema.RegistryID, diagnostic))
	if validated.SchemaID != contract.Schema.RegistryID ||
		validated.SchemaSHA256 != contract.Schema.SchemaSHA256 ||
		!reflect.DeepEqual(validated.Bytes, diagnostic) {
		panic("contract validation drifted from the shared fixture")
	}

	prior, priorSummary, err := client.CompilePortableWorkspace(request)
	if err != nil {
		panic(err)
	}
	defer prior.Close()
	current, currentSummary, err := client.CompilePortableWorkspace(request)
	if err != nil {
		panic(err)
	}
	defer current.Close()
	actualCompilation := (portableFixture{}).Compilation
	if err := json.Unmarshal(priorSummary.Bytes, &actualCompilation); err != nil {
		panic(err)
	}
	if !reflect.DeepEqual(actualCompilation, portable.Compilation) ||
		!reflect.DeepEqual(priorSummary.Bytes, currentSummary.Bytes) {
		panic("portable compilation drifted from the shared fixture")
	}
	comparison, _, err := client.ComparePortableWorkspaces(prior, current)
	if err != nil {
		panic(err)
	}
	actualPage, page, err := comparison.Impacts(0, 1)
	if err != nil {
		panic(err)
	}
	var decodedPage pale.WorkspaceImpactPage
	if err := json.Unmarshal(page.Bytes, &decodedPage); err != nil || !reflect.DeepEqual(actualPage, decodedPage) {
		panic("typed and canonical impact pages differ")
	}
	if len(actualPage.Entries) != 1 || actualPage.NextOffset == nil {
		panic("retained impact page is incomplete")
	}
	impact := actualPage.Entries[0]
	if *actualPage.NextOffset != portable.FirstImpact.NextOffset ||
		impact.Subject.Kind != pale.WorkspaceSubjectValueSourceKind ||
		impact.Subject.Source == nil ||
		impact.Subject.Source.SourceID != portable.FirstImpact.SourceID ||
		string(impact.State) != portable.FirstImpact.State ||
		impact.ContributingRootCount != portable.FirstImpact.ContributingRootCount {
		panic("retained impact page drifted from the shared fixture")
	}
	return comparison, diagnostic
}

func renderReport(configuration configuration, cases []benchmarkCase) string {
	header := strings.Join([]string{
		"source_coordinate", "fixture_sha256", "run_coordinate", "evidence_class",
		"leaf", "operation", "law_gate_passed", "request_bytes", "response_bytes",
		"copy_lower_bound_bytes", "samples", "p50_ns", "p95_ns", "p99_ns", "max_ns",
		"hardware", "runtime", "arch", "proof_ceiling", "release_evidence",
	}, "\t")
	var report strings.Builder
	report.WriteString(header)
	report.WriteByte('\n')
	for _, benchmark := range cases {
		p50, p95, p99, maximum := measure(configuration.samples, benchmark.operation, benchmark.release)
		evidenceClass := "local_characterization"
		if configuration.smoke {
			evidenceClass = "smoke_validation"
		}
		row := []string{
			configuration.source, configuration.fixtureSHA, configuration.run, evidenceClass,
			"go", benchmark.identifier, "true", fmt.Sprint(benchmark.requestBytes),
			fmt.Sprint(benchmark.responseBytes), fmt.Sprint(benchmark.requestBytes + benchmark.responseBytes),
			fmt.Sprint(configuration.samples), fmt.Sprint(p50), fmt.Sprint(p95), p99,
			fmt.Sprint(maximum), configuration.hardware, runtime.Version(), runtime.GOOS + "-" + runtime.GOARCH,
			"local_leaf_latency_and_payload_lower_bound_only", "false",
		}
		report.WriteString(strings.Join(row, "\t"))
		report.WriteByte('\n')
	}
	payload := report.String()
	seal := sha256.Sum256([]byte(payload))
	fmt.Fprintf(&report, "# pale-native-ffi-benchmark-seal\trows=%d\tpayload_sha256=%x\tsource_coordinate=%s\trun_coordinate=%s\n", len(cases), seal, configuration.source, configuration.run)
	return report.String()
}

func main() {
	smoke := flag.Bool("smoke", false, "run five compile-and-execute observations")
	fixtureRoot := flag.String("fixtures", "tests/fixtures/cross-leaf.v1", "cross-leaf fixture root")
	flag.Parse()
	root := must(filepath.Abs(*fixtureRoot))
	fixture := readJSON[bundleFixture](filepath.Join(root, "bundle-identity.json"))
	contract := readJSON[contractFixture](filepath.Join(root, "contract-diagnostic.json"))
	portable := readJSON[portableFixture](filepath.Join(root, "portable-journey.json"))
	documents := make([]pale.Document, 0, len(portable.DocumentLabels))
	requestBytes := len(portable.RootManifest)
	for _, label := range portable.DocumentLabels {
		bytes := must(os.ReadFile(filepath.Join(root, "portable", label)))
		documents = append(documents, pale.Document{Label: label, Bytes: bytes})
		requestBytes += len(label) + len(bytes)
	}
	request := pale.PortableWorkspaceRequest{RootManifest: portable.RootManifest, Documents: documents}
	client := must(pale.NewClient())
	defer client.Close()
	comparison, diagnostic := proveLaws(client, fixture, contract, portable, request)
	defer comparison.Close()
	var compiled *pale.PortableWorkspace
	compilationResponseBytes := len(portable.Compilation.WorkspaceID) + (3 * 64) + (7 * 4)
	impactResponseBytes := len(portable.FirstImpact.SourceID) + len(portable.FirstImpact.State) + 8
	cases := []benchmarkCase{
		{"bundle_identity", 0, 4 + (2 * 64), func() { must(client.ContractBundleIdentity()) }, nil},
		{"diagnostic_admission", len(diagnostic), len(diagnostic), func() { must(client.AdmitDiagnostic(diagnostic)) }, nil},
		{"contract_validation", len(contract.Schema.RegistryID) + len(diagnostic), len(diagnostic), func() { must(client.ValidateContractDocument(contract.Schema.RegistryID, diagnostic)) }, nil},
		{"portable_compilation", requestBytes, compilationResponseBytes, func() {
			var err error
			compiled, _, err = client.CompilePortableWorkspace(request)
			if err != nil {
				panic(err)
			}
		}, func() {
			compiled.Close()
			compiled = nil
		}},
		{"retained_impact_page", 14, impactResponseBytes, func() {
			if _, _, err := comparison.Impacts(0, 1); err != nil {
				panic(err)
			}
		}, nil},
	}
	configuration := configuration{
		smoke: *smoke,
		samples: func() int {
			if *smoke {
				return smokeSamples
			}
			return characterizationRuns
		}(),
		source:     coordinate("PALE_FFI_BENCH_SOURCE_COORDINATE", *smoke, "smoke-unsealed-worktree"),
		run:        coordinate("PALE_FFI_BENCH_RUN_COORDINATE", *smoke, "smoke-validation"),
		hardware:   coordinate("PALE_FFI_BENCH_HARDWARE", *smoke, "unspecified-smoke-host"),
		fixtureSHA: fixtureDigest(root, portable),
	}
	fmt.Print(renderReport(configuration, cases))
}
