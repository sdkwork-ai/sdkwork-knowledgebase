import { createHash } from "node:crypto";
import { readFile, realpath, stat } from "node:fs/promises";
import path from "node:path";

export const CERTIFICATION_ARTIFACT_PREFIX =
  "docs/releases/provider-certification/artifacts/";

export function normalizeCertificationArtifactReference(value) {
  return typeof value === "string" ? value.replaceAll("\\", "/") : "";
}

export function isCertificationArtifactReference(value) {
  const reference = normalizeCertificationArtifactReference(value);
  return reference.startsWith(CERTIFICATION_ARTIFACT_PREFIX) && !reference.includes("..");
}

export async function readBoundedCertificationArtifact(referenceValue, workspaceRoot, maxBytes) {
  const reference = normalizeCertificationArtifactReference(referenceValue);
  if (!isCertificationArtifactReference(reference)) {
    throw new Error("reference must stay inside the certification artifact root");
  }
  if (!Number.isSafeInteger(maxBytes) || maxBytes <= 0) {
    throw new Error("artifact byte limit must be a positive safe integer");
  }

  const artifactRoot = path.resolve(workspaceRoot, CERTIFICATION_ARTIFACT_PREFIX);
  // Resolve the root through realpath too: a symlinked workspace/temp directory would
  // otherwise make the resolved target look like it escaped the lexical root.
  const resolvedArtifactRoot = await realpath(artifactRoot).catch(() => artifactRoot);
  const target = await realpath(path.resolve(workspaceRoot, reference));
  const relativeTarget = path.relative(resolvedArtifactRoot, target);
  if (relativeTarget.startsWith("..") || path.isAbsolute(relativeTarget)) {
    throw new Error("resolved path escapes the certification artifact root");
  }

  const artifactStat = await stat(target);
  if (!artifactStat.isFile()) {
    throw new Error("artifact is not a regular file");
  }
  if (artifactStat.size > maxBytes) {
    throw new Error(`artifact exceeds the ${maxBytes} byte limit`);
  }

  const bytes = await readFile(target);
  return {
    bytes,
    digest: createHash("sha256").update(bytes).digest("hex"),
    reference,
  };
}
