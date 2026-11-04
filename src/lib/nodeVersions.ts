import type { NvmInfo, Project, RemoteNodeVersion } from "../types/project";

export interface ListedNodeVersion extends Partial<RemoteNodeVersion> {
  version: string;
  isInstalled: boolean;
  isCurrent: boolean;
}

export function normalizeNodeVersion(version: string) {
  return version.trim().replace(/^v/i, "");
}

export function mergeNodeVersions(
  remoteVersions: RemoteNodeVersion[],
  nvmInfo: NvmInfo | null
): ListedNodeVersion[] {
  const installed = new Set(
    nvmInfo?.availableVersions.map((item) => normalizeNodeVersion(item.version))
  );
  const versions = new Map<string, ListedNodeVersion>();
  const current = normalizeNodeVersion(nvmInfo?.currentVersion || "");

  for (const remote of remoteVersions) {
    const version = normalizeNodeVersion(remote.version);
    versions.set(version, {
      ...remote,
      version: `v${version}`,
      isInstalled: installed.has(version),
      isCurrent: current === version,
    });
  }

  for (const version of installed) {
    if (!versions.has(version)) {
      versions.set(version, {
        version: `v${version}`,
        isInstalled: true,
        isCurrent: current === version,
      });
    }
  }

  return [...versions.values()].sort((a, b) =>
    b.version.localeCompare(a.version, "en", { numeric: true })
  );
}

export function projectsUsingNodeVersion(projects: Project[], version: string) {
  const exactVersion = normalizeNodeVersion(version);
  if (!/^\d+\.\d+\.\d+$/.test(exactVersion)) return [];
  return projects.filter(
    (project) => normalizeNodeVersion(project.nodeVersion || "") === exactVersion
  );
}
