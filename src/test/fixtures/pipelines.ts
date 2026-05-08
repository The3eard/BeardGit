/**
 * Factories for CI pipeline fixtures: CiRun, CiRunDetail, CiStage,
 * CiJob, CiJobStep.
 *
 * `makeCiRunList()` returns a mix of statuses (success, running,
 * failed, cancelled, pending) so a single screenshot exercises every
 * status badge.
 */

import type {
  CiJob,
  CiJobStep,
  CiRun,
  CiRunDetail,
  CiStage,
} from "../../lib/types";

export function makeCiRun(overrides: Partial<CiRun> = {}): CiRun {
  return {
    id: 100123,
    display_id: 4521,
    status: "success",
    ref_name: "feat/visual-tests",
    sha: "1".repeat(40),
    source: "push",
    name: "CI",
    actor: "adolfofuentes",
    created_at: "2026-05-08T10:00:00Z",
    updated_at: "2026-05-08T10:08:30Z",
    web_url: "https://github.com/adolfofuentes/beardgit/actions/runs/100123",
    ...overrides,
  };
}

export function makeCiRunList(): CiRun[] {
  return [
    makeCiRun({ id: 100123, display_id: 4521, status: "success" }),
    makeCiRun({
      id: 100122,
      display_id: 4520,
      status: "running",
      ref_name: "fix/graph-flicker",
      sha: "2".repeat(40),
      created_at: "2026-05-08T09:55:00Z",
      updated_at: "2026-05-08T09:58:00Z",
    }),
    makeCiRun({
      id: 100121,
      display_id: 4519,
      status: "failed",
      ref_name: "refactor/mutations",
      sha: "3".repeat(40),
      actor: "octocat",
      created_at: "2026-05-08T09:30:00Z",
      updated_at: "2026-05-08T09:36:00Z",
    }),
    makeCiRun({
      id: 100120,
      display_id: 4518,
      status: "cancelled",
      ref_name: "deps/bump-2026-04",
      sha: "4".repeat(40),
      actor: "dependabot[bot]",
      created_at: "2026-05-08T08:00:00Z",
      updated_at: "2026-05-08T08:03:00Z",
    }),
    makeCiRun({
      id: 100119,
      display_id: 4517,
      status: "pending",
      ref_name: "feat/another",
      sha: "5".repeat(40),
      created_at: "2026-05-08T07:50:00Z",
      updated_at: "2026-05-08T07:50:00Z",
    }),
  ];
}

export function makeCiJobStep(
  overrides: Partial<CiJobStep> = {},
): CiJobStep {
  return {
    number: 1,
    name: "Set up job",
    status: "success",
    duration: 2,
    ...overrides,
  };
}

export function makeCiJob(overrides: Partial<CiJob> = {}): CiJob {
  return {
    id: 7001,
    name: "test",
    stage: "test",
    status: "success",
    duration: 124,
    started_at: "2026-05-08T10:01:00Z",
    finished_at: "2026-05-08T10:03:04Z",
    web_url:
      "https://github.com/adolfofuentes/beardgit/actions/runs/100123/jobs/7001",
    allow_failure: false,
    steps: [
      makeCiJobStep({ number: 1, name: "Set up job", status: "success", duration: 2 }),
      makeCiJobStep({ number: 2, name: "Checkout", status: "success", duration: 4 }),
      makeCiJobStep({ number: 3, name: "Install deps", status: "success", duration: 38 }),
      makeCiJobStep({ number: 4, name: "Run tests", status: "success", duration: 78 }),
      makeCiJobStep({ number: 5, name: "Cleanup", status: "success", duration: 2 }),
    ],
    ...overrides,
  };
}

export function makeCiStage(overrides: Partial<CiStage> = {}): CiStage {
  return {
    name: "test",
    jobs: [makeCiJob()],
    ...overrides,
  };
}

export function makeCiRunDetail(
  overrides: Partial<CiRunDetail> = {},
): CiRunDetail {
  return {
    run: makeCiRun(),
    duration: 510,
    finished_at: "2026-05-08T10:08:30Z",
    stages: [
      makeCiStage({
        name: "build",
        jobs: [makeCiJob({ id: 7000, name: "build", stage: "build", duration: 220 })],
      }),
      makeCiStage({
        name: "test",
        jobs: [
          makeCiJob({ id: 7001, name: "test:rust", stage: "test", duration: 124 }),
          makeCiJob({ id: 7002, name: "test:visual", stage: "test", duration: 78 }),
        ],
      }),
      makeCiStage({
        name: "deploy",
        jobs: [
          makeCiJob({
            id: 7003,
            name: "deploy",
            stage: "deploy",
            status: "skipped",
            duration: null,
            started_at: null,
            finished_at: null,
            steps: null,
          }),
        ],
      }),
    ],
    ...overrides,
  };
}
