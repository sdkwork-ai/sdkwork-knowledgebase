import {
  getKnowledgebaseDriveAppSdkClient,
  isKnowledgebaseDriveApiAvailable,
} from 'sdkwork-knowledgebase-pc-core';

/** Largest slice requested per Drive content read (the gateway caps responses). */
const DRIVE_CONTENT_CHUNK_BYTES = 1024 * 1024;

/** Upper bound on reassembled text; a knowledge concept never approaches it. */
const DRIVE_CONTENT_MAX_BYTES = 4 * DRIVE_CONTENT_CHUNK_BYTES;

function decodeBase64Bytes(value: string): Uint8Array {
  const binary = globalThis.atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function concatenateBytes(chunks: readonly Uint8Array[], totalLength: number): Uint8Array {
  const merged = new Uint8Array(totalLength);
  let offset = 0;
  for (const chunk of chunks) {
    merged.set(chunk, offset);
    offset += chunk.length;
  }
  return merged;
}

/**
 * Read a Drive-backed text object (an OKF concept body or a text source file).
 *
 * The OKF bundle keeps every concept as a Markdown file under `okf/` in the
 * space's Drive, and the knowledge-browser node for a concept is that same
 * file. Reading it through `drive.nodes.content.retrieve` is the authoritative
 * object read: the knowledge retrieval API is RAG-only and refuses OKF-bundle
 * spaces (`rag_retrieval_mode_required`, 40003), so it can never serve a
 * concept body.
 *
 * Content arrives base64-encoded in bounded slices. Bytes are decoded per
 * slice and concatenated before one UTF-8 decode, so a slice boundary that
 * splits a multi-byte character still yields the exact file text.
 *
 * @param driveNodeId - Drive node holding the text object.
 * @returns the decoded text, or null when Drive is unavailable, the read fails,
 *   the object is empty, or it exceeds the reassembly bound.
 */
export async function readDriveNodeTextContent(driveNodeId: string): Promise<string | null> {
  const nodeId = driveNodeId.trim();
  if (!nodeId || !isKnowledgebaseDriveApiAvailable() || typeof globalThis.atob !== 'function') {
    return null;
  }

  try {
    const drive = getKnowledgebaseDriveAppSdkClient().client;
    const chunks: Uint8Array[] = [];
    let totalLength = 0;
    let rangeStart = 0;

    for (;;) {
      const response = await drive.drive.nodes.content.retrieve(nodeId, {
        encoding: 'base64',
        maxBytes: DRIVE_CONTENT_CHUNK_BYTES,
        ...(rangeStart > 0 ? { byteRangeStart: String(rangeStart) } : {}),
      });
      const chunk = decodeBase64Bytes(response.content);
      chunks.push(chunk);
      totalLength += chunk.length;

      if (totalLength > DRIVE_CONTENT_MAX_BYTES) {
        console.warn(
          `[KnowledgeDriveContent] Drive node ${nodeId} exceeds the ${DRIVE_CONTENT_MAX_BYTES}-byte text read bound.`,
        );
        return null;
      }
      if (!response.hasMore) {
        break;
      }

      const nextStart = Number(response.returnedRangeStart) + Number(response.returnedRangeLength);
      if (!Number.isSafeInteger(nextStart) || nextStart <= rangeStart) {
        // Without a provable next offset, continuing would re-read the same
        // slice; the partial body is not this concept's text.
        console.warn(
          `[KnowledgeDriveContent] Drive node ${nodeId} reported more content without a usable next range.`,
        );
        return null;
      }
      rangeStart = nextStart;
    }

    const text = new TextDecoder().decode(concatenateBytes(chunks, totalLength));
    return text.trim() ? text : null;
  } catch (error) {
    console.warn(`[KnowledgeDriveContent] Drive node ${nodeId} content read failed.`, error);
    return null;
  }
}
