package tests_test

import (
	"errors"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

// Nil and zero-value handles must return a bridge error before dispatch.
func TestUnavailableHandlesRejectBeforeDispatch(t *testing.T) {
	for _, state := range []string{"nil", "zero"} {
		var workspace *pale.PortableWorkspace
		var comparison *pale.WorkspaceComparison
		var profile *pale.EffectiveProfile
		var compilation *pale.OneShotCompilation
		var local *pale.LocalCompilation
		var plan *pale.LocalPlan
		var run *pale.LocalRun
		if state == "zero" {
			workspace = &pale.PortableWorkspace{}
			comparison = &pale.WorkspaceComparison{}
			profile = &pale.EffectiveProfile{}
			compilation = &pale.OneShotCompilation{}
			local = &pale.LocalCompilation{}
			plan = &pale.LocalPlan{}
			run = &pale.LocalRun{}
		}
		for _, test := range []struct {
			name string
			call func() error
		}{
			{"configuration-review", func() error {
				_, err := workspace.InspectConfigurationImprovement()
				return err
			}},
			{"profile-inspection", func() error {
				_, err := profile.Inspect(pale.ProfileIntendedUseCase, 0, 1)
				return err
			}},
			{"profile-explanation", func() error {
				_, err := profile.Explain("")
				return err
			}},
			{"compilation-summary", func() error {
				_, err := compilation.Summary()
				return err
			}},
			{"observation-targets", func() error {
				_, err := compilation.ObservationTargets("engine", pale.PlanApply, pale.PriorMaterialization{})
				return err
			}},
			{"captured-plan", func() error {
				_, _, _, err := compilation.Plan("engine", pale.PlanApply, pale.PriorMaterialization{}, nil)
				return err
			}},
			{"change-roots", func() error {
				_, _, err := comparison.ChangeRoots(0, 1)
				return err
			}},
			{"impacts", func() error {
				_, _, err := comparison.Impacts(0, 1)
				return err
			}},
			{"comparison-explanation", func() error {
				_, _, err := comparison.Explain(pale.ModuleSubject("module"))
				return err
			}},
			{"repository-status", func() error {
				_, err := local.RepositoryStatus("engine")
				return err
			}},
			{"local-plan", func() error {
				_, _, _, err := local.Plan("engine", pale.PlanApply)
				return err
			}},
			{"plan-publication", func() error {
				_, err := plan.Publish("")
				return err
			}},
			{"run-admission", func() error {
				_, _, err := local.AdmitRun(pale.PlanApply, "", "")
				return err
			}},
			{"run-settlement", func() error {
				_, err := run.Settle()
				return err
			}},
			{"run-reconciliation", func() error {
				_, err := run.Reconciliation()
				return err
			}},
			{"executable-binding", func() error {
				_, err := local.VerifyCurrentExecutableBinding()
				return err
			}},
		} {
			t.Run(state+"/"+test.name, func(t *testing.T) {
				defer func() {
					if failure := recover(); failure != nil {
						t.Errorf("unavailable handle panicked instead of returning a bridge error: %v", failure)
					}
				}()
				var failure *pale.BridgeError
				if err := test.call(); !errors.As(err, &failure) || failure.Code != "invalid_handle" {
					t.Fatalf("expected invalid_handle before dispatch, got %v", err)
				}
			})
		}
	}
}
