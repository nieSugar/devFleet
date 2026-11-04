import assert from "node:assert/strict";
import {
  mergeNodeVersions,
  projectsUsingNodeVersion,
} from "../src/lib/nodeVersions";
import type { NvmInfo, Project, RemoteNodeVersion } from "../src/types/project";

const local: NvmInfo = {
  manager: "builtin",
  isInstalled: true,
  currentVersion: "v22.9.0",
  availableVersions: [
    { version: "22.9.0", fullVersion: "v22.9.0" },
    { version: "v22.10.0", fullVersion: "v22.10.0" },
    { version: "18.20.0", fullVersion: "v18.20.0" },
  ],
};
const remote: RemoteNodeVersion = {
  version: "v22.9.0",
  date: "2024-09-17",
  files: [],
  lts: "Jod",
  security: false,
};

const offline = mergeNodeVersions([], local);
assert.deepEqual(offline.map((item) => item.version), [
  "v22.10.0", "v22.9.0", "v18.20.0",
]);
assert.equal(offline.find((item) => item.isCurrent)?.version, "v22.9.0");
assert.ok(offline.every((item) => item.isInstalled));
assert.equal(offline[0].lts, undefined, "Offline LTS metadata is unknown, not false");

const merged = mergeNodeVersions([remote], local);
assert.equal(merged.length, 3, "Local and remote versions must not be duplicated");
assert.equal(merged[1].lts, "Jod");
assert.equal(merged[1].isInstalled, true);
assert.deepEqual(mergeNodeVersions([], null), []);
assert.equal(mergeNodeVersions([remote], null)[0].isInstalled, false);

const projects: Project[] = ["22.9.0", "v22.9.0", " 22.9.0 ", ">=22.9.0", "22", "lts/*", "22.10.0"].map(
  (nodeVersion, index) => ({ id: String(index), name: "Project", path: "/project", scripts: [], nodeVersion })
);
assert.deepEqual(projectsUsingNodeVersion(projects, "v22.9.0").map((item) => item.id), ["0", "1", "2"]);
assert.deepEqual(projectsUsingNodeVersion(projects, "22"), []);
console.log("Node version merge and exact project-impact checks passed.");
