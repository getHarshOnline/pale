package tests_test

import (
	"context"
	"encoding/json"
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"testing"
	"time"
)

// Compile one owning testdata source into the actual SDK package. The exact
// root test cannot recurse into the external runner. No source is written into
// the SDK root, and testdata is excluded from ordinary package discovery.
func runPackageOverlayTest(t *testing.T, sourceFile, testName string, fixture []byte) {
	t.Helper()
	root, err := filepath.Abs("..")
	if err != nil {
		t.Fatal(err)
	}
	target := filepath.Join(root, "zz_package_overlay_test.go")
	if _, err := os.Stat(target); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("overlay target must be absent: %s (%v)", target, err)
	}
	temporary := t.TempDir()
	fixturePath := ""
	if fixture != nil {
		fixturePath = filepath.Join(temporary, "fixture.json")
		if err := os.WriteFile(fixturePath, fixture, 0600); err != nil {
			t.Fatal(err)
		}
	}
	overlayPath := filepath.Join(temporary, "overlay.json")
	encoded, err := json.Marshal(struct{ Replace map[string]string }{map[string]string{
		target: filepath.Join(root, "tests", "testdata", sourceFile),
	}})
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(overlayPath, encoded, 0600); err != nil {
		t.Fatal(err)
	}
	// Watchdogs bound a failed process, not acceptable operation latency. The
	// child is sequential; an outer -race does not instrument its test binary.
	ctx, cancel := context.WithTimeout(t.Context(), 2*time.Minute)
	defer cancel()
	command := exec.CommandContext(ctx, filepath.Join(runtime.GOROOT(), "bin", "go"),
		"test", "-overlay", overlayPath, "-run", "^"+regexp.QuoteMeta(testName)+"$", "-count=1", "-timeout=90s", "-v", ".")
	command.Dir = root
	command.Env = append(os.Environ(), "GOPROXY=off", "GOSUMDB=off", "GOTOOLCHAIN=local", "PALE_GO_TEST_FIXTURE="+fixturePath)
	output, err := command.CombinedOutput()
	if err != nil {
		t.Fatalf("package-local native test failed: %v (watchdog: %v)\n%s", err, ctx.Err(), output)
	}
	t.Logf("package-local native test:\n%s", output)
}
