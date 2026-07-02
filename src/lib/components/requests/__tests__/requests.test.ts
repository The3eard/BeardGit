/**
 * Smoke tests for the Requests-panel IPC flows folded into the typed
 * contract in Phase 1 of spec 05.
 *
 * The feature shipped with zero tests; these lock in that each `requests_*`
 * wrapper maps to the right command name + payload (camelCase → snake_case
 * happens on the Rust side, so the wrappers pass camelCase keys), and that
 * the store's `.http` serializer round-trips a request doc. Together they
 * cover the save / rename / delete / env-switch happy paths the migrated
 * components drive.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import {
  requestsSave,
  requestsRename,
  requestsDelete,
  requestsDuplicate,
  requestsSetEnv,
  requestsGetEnvs,
  requestsRun,
} from "$lib/api/tauri";
import { requestDocToHttp, type RequestDoc } from "../stores";

beforeEach(() => {
  mocks.invoke.mockReset();
});

describe("requests IPC wrappers", () => {
  it("requestsSave maps to requests_save with camelCase args", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await requestsSave("project", "users/get.http", "/repo", "GET x\n");
    expect(mocks.invoke).toHaveBeenCalledWith("requests_save", {
      sourceKind: "project",
      sourcePath: "users/get.http",
      projectPath: "/repo",
      content: "GET x\n",
    });
  });

  it("requestsRename maps to requests_rename", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await requestsRename("project", "a.http", "b.http", "/repo");
    expect(mocks.invoke).toHaveBeenCalledWith("requests_rename", {
      sourceKind: "project",
      fromPath: "a.http",
      toPath: "b.http",
      projectPath: "/repo",
    });
  });

  it("requestsDelete maps to requests_delete", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await requestsDelete("project", "a.http", "/repo");
    expect(mocks.invoke).toHaveBeenCalledWith("requests_delete", {
      sourceKind: "project",
      sourcePath: "a.http",
      projectPath: "/repo",
    });
  });

  it("requestsDuplicate returns the new source path", async () => {
    mocks.invoke.mockResolvedValue("a copy.http");
    const out = await requestsDuplicate("project", "a.http", "/repo");
    expect(out).toBe("a copy.http");
    expect(mocks.invoke).toHaveBeenCalledWith("requests_duplicate", {
      sourceKind: "project",
      sourcePath: "a.http",
      projectPath: "/repo",
    });
  });

  it("requestsSetEnv maps the env-switch to requests_set_env", async () => {
    mocks.invoke.mockResolvedValue(undefined);
    await requestsSetEnv("/repo", "dev");
    expect(mocks.invoke).toHaveBeenCalledWith("requests_set_env", {
      projectPath: "/repo",
      envName: "dev",
    });
  });

  it("requestsGetEnvs returns the summaries the backend hands back", async () => {
    mocks.invoke.mockResolvedValue([
      { name: "default", vars_count: 2, secrets: ["TOKEN"] },
    ]);
    const envs = await requestsGetEnvs("/repo");
    expect(envs).toEqual([
      { name: "default", vars_count: 2, secrets: ["TOKEN"] },
    ]);
    expect(mocks.invoke).toHaveBeenCalledWith("requests_get_envs", {
      projectPath: "/repo",
    });
  });

  it("requestsRun forwards the nested args payload verbatim", async () => {
    mocks.invoke.mockResolvedValue({
      history_id: 1,
      status: 200,
      headers: [],
      body_base64: "",
      truncated: false,
      duration_ms: 5,
    });
    const args = {
      source_kind: "project",
      source_path: "a.http",
      project_path: "/repo",
      env_name: "dev",
      overrides: {},
      ticket_id: "t-1",
    };
    const res = await requestsRun(args);
    expect(res.status).toBe(200);
    expect(mocks.invoke).toHaveBeenCalledWith("requests_run", { args });
  });
});

describe("requestDocToHttp", () => {
  it("serializes name, request line, headers, and body", () => {
    const doc: RequestDoc = {
      name: "get user",
      method: "POST",
      url: "https://api.example.com/u",
      headers: [
        ["Content-Type", "application/json"],
        ["", "skip-me"],
      ],
      body: '{"a":1}',
    };
    expect(requestDocToHttp(doc)).toBe(
      "# @name get user\n" +
        "POST https://api.example.com/u\n" +
        "Content-Type: application/json\n" +
        "\n" +
        '{"a":1}\n',
    );
  });

  it("omits the name line and body when absent", () => {
    const doc: RequestDoc = {
      method: "GET",
      url: "https://x",
      headers: [],
    };
    expect(requestDocToHttp(doc)).toBe("GET https://x\n");
  });
});
