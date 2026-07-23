# Thin development aliases. The Just recipes and scripts remain the command source of truth.

.PHONY: help hooks-install hooks-check doctor gates-list gates-run gates-self-tests run run-derived readiness run-dawn-legacy run-living-legacy view view-gpu view-living-legacy view-living-gpu-legacy ledger-inventory ledger-inventory-check verify check-fast check check-pr check-full check-nightly check-legacy ci ci-local ci-list ci-legacy ci-list-legacy test test-legacy audit-parked fmt fmt-check fmt-legacy fmt-check-legacy lint lint-legacy pins-dawn-legacy stop-gate cache-info gc gc-dry trim-wsl

GATE_TIER ?= pr

ifeq ($(OS),Windows_NT)
DEV := pwsh -NoProfile -File scripts/dev.ps1
HOOKS_INSTALL_CMD := $(DEV) hooks-install
HOOKS_CHECK_CMD := $(DEV) hooks-check
DOCTOR_CMD := $(DEV) doctor
GATES_LIST_CMD := $(DEV) gates-list $(GATE_TIER)
GATES_RUN_CMD := $(DEV) gates-run $(GATE_TIER)
GATES_SELF_TESTS_CMD := $(DEV) gates-self-tests $(GATE_TIER)
VERIFY_CMD := $(DEV) verify
CHECK_FAST_CMD := $(DEV) check-fast
CHECK_CMD := $(DEV) check
CHECK_PR_CMD := $(DEV) check-pr
CHECK_FULL_CMD := $(DEV) check-full
CHECK_NIGHTLY_CMD := $(DEV) check-nightly
CHECK_LEGACY_CMD := $(DEV) check-legacy
CI_CMD := $(DEV) ci
CI_LOCAL_CMD := $(DEV) ci-local
CI_LIST_CMD := $(DEV) ci-list
CI_LEGACY_CMD := $(DEV) ci-legacy
CI_LIST_LEGACY_CMD := $(DEV) ci-list-legacy
TEST_CMD := $(DEV) test
TEST_LEGACY_CMD := $(DEV) test-legacy
AUDIT_PARKED_CMD := $(DEV) audit-parked
FMT_CMD := $(DEV) fmt
FMT_CHECK_CMD := $(DEV) fmt-check
FMT_LEGACY_CMD := $(DEV) fmt-legacy
FMT_CHECK_LEGACY_CMD := $(DEV) fmt-check-legacy
LINT_CMD := $(DEV) lint
LINT_LEGACY_CMD := $(DEV) lint-legacy
RUN_CMD := $(DEV) run
RUN_DERIVED_CMD := $(DEV) run-derived
RUN_DAWN_LEGACY_CMD := $(DEV) run-dawn-legacy
RUN_LIVING_LEGACY_CMD := $(DEV) run-living-legacy
READINESS_CMD := $(DEV) readiness
VIEW_CMD := $(DEV) view
VIEW_GPU_CMD := $(DEV) view-gpu
VIEW_LIVING_LEGACY_CMD := $(DEV) view-living-legacy
VIEW_LIVING_GPU_LEGACY_CMD := $(DEV) view-living-gpu-legacy
LEDGER_INVENTORY_CMD := $(DEV) ledger-inventory
LEDGER_INVENTORY_CHECK_CMD := $(DEV) ledger-inventory-check
PINS_DAWN_LEGACY_CMD := $(DEV) pins-dawn-legacy
STOP_CMD := $(DEV) stop-gate
CACHE_INFO_CMD := $(DEV) cache-info
GC_CMD := $(DEV) gc
GC_DRY_CMD := $(DEV) gc-dry
TRIM_WSL_CMD := $(DEV) trim-wsl
else
HOOKS_INSTALL_CMD := just hooks-install
HOOKS_CHECK_CMD := just hooks-check
DOCTOR_CMD := just doctor
GATES_LIST_CMD := just gates-list $(GATE_TIER)
GATES_RUN_CMD := just gates-run $(GATE_TIER)
GATES_SELF_TESTS_CMD := just gates-self-tests $(GATE_TIER)
VERIFY_CMD := just verify
CHECK_FAST_CMD := just check-fast
CHECK_CMD := just check-pr
CHECK_PR_CMD := just check-pr
CHECK_FULL_CMD := just check-full
CHECK_NIGHTLY_CMD := just check-nightly
CHECK_LEGACY_CMD := just check-legacy
CI_CMD := just ci
CI_LOCAL_CMD := just ci-local
CI_LIST_CMD := just ci-list
CI_LEGACY_CMD := just ci-legacy
CI_LIST_LEGACY_CMD := just ci-list-legacy
TEST_CMD := just test
TEST_LEGACY_CMD := just test-legacy
AUDIT_PARKED_CMD := just audit-parked
FMT_CMD := just fmt
FMT_CHECK_CMD := just fmt-check
FMT_LEGACY_CMD := just fmt-legacy
FMT_CHECK_LEGACY_CMD := just fmt-check-legacy
LINT_CMD := just lint
LINT_LEGACY_CMD := just lint-legacy
RUN_CMD := just run
RUN_DERIVED_CMD := just run-derived
RUN_DAWN_LEGACY_CMD := just run-dawn-legacy
RUN_LIVING_LEGACY_CMD := just run-living-legacy
READINESS_CMD := just readiness
VIEW_CMD := just view
VIEW_GPU_CMD := just view-gpu
VIEW_LIVING_LEGACY_CMD := just view-living-legacy
VIEW_LIVING_GPU_LEGACY_CMD := just view-living-gpu-legacy
LEDGER_INVENTORY_CMD := just ledger-inventory
LEDGER_INVENTORY_CHECK_CMD := just ledger-inventory-check
PINS_DAWN_LEGACY_CMD := just pins-dawn-legacy
STOP_CMD := just stop-gate
CACHE_INFO_CMD := just cache-info
GC_CMD := just gc
GC_DRY_CMD := just gc-dry
TRIM_WSL_CMD := just trim-wsl
endif

help:
	@printf '%s\n' \
	  'make hooks-install    install the tracked Stone 0 pre-push hook for this clone' \
	  'make hooks-check      verify the tracked pre-push hook is active' \
	  'make doctor           verify tools, manifests, registries, and boundaries' \
	  'make gates-list GATE_TIER=pr print ordered declarative gate ids' \
	  'make gates-run GATE_TIER=pr run one declarative structural tier' \
	  'make gates-self-tests GATE_TIER=canonical run declared detector self-tests' \
	  'make run RUN_ARGS=... canonical planet run or structured refusal' \
	  'make run-derived       floor-only alias for the former derived view' \
	  'make readiness        canonical planet readiness receipt' \
	  'make run-dawn-legacy  parked dawn development fixture' \
	  'make run-living-legacy parked living-world fixture' \
	  'make view             snapshot-only viewer or visible refusal' \
	  'make view-gpu         canonical GPU viewer refusal until an adapter exists' \
	  'make view-living-legacy parked causal viewer' \
	  'make view-living-gpu-legacy parked causal GPU viewer' \
	  'make ledger-inventory regenerate four-tier by seven-tag inventory' \
	  'make ledger-inventory-check verify the checked-in inventory' \
	  'make verify           document and prose gate' \
	  'make check-fast       non-certifying developer compile loop' \
	  'make check            canonical PR tier' \
	  'make check-full       canonical full CPU tier' \
	  'make check-nightly    scheduled canonical tier' \
	  'make check-legacy     parked workspace checks (not CI parity)' \
	  'make ci               canonical CI recipe' \
	  'make ci-local         exact canonical CI recipe' \
	  'make ci-list          display the canonical CI recipe' \
	  'make ci-legacy        parked and legacy aggregate checks' \
	  'make ci-list-legacy   display the parked aggregate recipe' \
	  'make test             canonical abiotic workspace tests' \
	  'make test-legacy      parked workspace tests' \
	  'make audit-parked     retired calibration/profile/quarantine checks' \
	  'make fmt              format the canonical workspace' \
	  'make fmt-check        check canonical formatting' \
	  'make fmt-legacy       format the parked workspace' \
	  'make fmt-check-legacy check parked formatting' \
	  'make lint             canonical planet Clippy gate' \
	  'make lint-legacy      parked workspace Clippy gate' \
	  'make pins-dawn-legacy compare old dawn fixture digests' \
	  'make stop-gate        repository Stop hook' \
	  'make cache-info       show bounded native-WSL cache paths' \
	  'make gc               bound Cargo build artifacts' \
	  'make gc-dry           report artifact cleanup without deleting' \
	  'make trim-wsl         issue an online WSL filesystem trim'

hooks-install:
	@$(HOOKS_INSTALL_CMD)

hooks-check:
	@$(HOOKS_CHECK_CMD)

run:
	@$(RUN_CMD) $(RUN_ARGS)

run-derived:
	@$(RUN_DERIVED_CMD)

doctor:
	@$(DOCTOR_CMD)

gates-list:
	@$(GATES_LIST_CMD)

gates-run:
	@$(GATES_RUN_CMD)

gates-self-tests:
	@$(GATES_SELF_TESTS_CMD)

run-dawn-legacy:
	@$(RUN_DAWN_LEGACY_CMD)

run-living-legacy:
	@$(RUN_LIVING_LEGACY_CMD)

readiness:
	@$(READINESS_CMD)

view:
	@$(VIEW_CMD)

view-gpu:
	@$(VIEW_GPU_CMD)

view-living-legacy:
	@$(VIEW_LIVING_LEGACY_CMD)

view-living-gpu-legacy:
	@$(VIEW_LIVING_GPU_LEGACY_CMD)

ledger-inventory:
	@$(LEDGER_INVENTORY_CMD)

ledger-inventory-check:
	@$(LEDGER_INVENTORY_CHECK_CMD)

verify:
	@$(VERIFY_CMD)

check-fast:
	@$(CHECK_FAST_CMD)

check:
	@$(CHECK_CMD)

check-pr:
	@$(CHECK_PR_CMD)

check-full:
	@$(CHECK_FULL_CMD)

check-nightly:
	@$(CHECK_NIGHTLY_CMD)

check-legacy:
	@$(CHECK_LEGACY_CMD)

ci:
	@$(CI_CMD)

ci-local:
	@$(CI_LOCAL_CMD)

ci-list:
	@$(CI_LIST_CMD)

ci-legacy:
	@$(CI_LEGACY_CMD)

ci-list-legacy:
	@$(CI_LIST_LEGACY_CMD)

test:
	@$(TEST_CMD)

test-legacy:
	@$(TEST_LEGACY_CMD)

audit-parked:
	@$(AUDIT_PARKED_CMD)

fmt:
	@$(FMT_CMD)

fmt-check:
	@$(FMT_CHECK_CMD)

fmt-legacy:
	@$(FMT_LEGACY_CMD)

fmt-check-legacy:
	@$(FMT_CHECK_LEGACY_CMD)

lint:
	@$(LINT_CMD)

lint-legacy:
	@$(LINT_LEGACY_CMD)

pins-dawn-legacy:
	@$(PINS_DAWN_LEGACY_CMD)

stop-gate:
	@$(STOP_CMD)

cache-info:
	@$(CACHE_INFO_CMD)

gc:
	@$(GC_CMD)

gc-dry:
	@$(GC_DRY_CMD)

trim-wsl:
	@$(TRIM_WSL_CMD)
