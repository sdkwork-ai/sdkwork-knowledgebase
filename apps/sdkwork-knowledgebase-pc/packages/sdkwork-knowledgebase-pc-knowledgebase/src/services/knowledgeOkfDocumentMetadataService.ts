import { getKnowledgebaseAppSdkClient } from 'sdkwork-knowledgebase-pc-core';

function requireSdkClient() {
  return getKnowledgebaseAppSdkClient().client;
}

export async function readOkfConceptTags(conceptRowId: string): Promise<string[]> {
  const concept = await requireSdkClient().knowledge.okf.concepts.retrieve(conceptRowId);
  return concept.tags ?? [];
}

function formatOkfTagsLine(tags: string[]): string {
  if (tags.length === 0) {
    return 'tags: []';
  }
  const formatted = tags
    .map((tag) => (/\s|,|\[|\]/.test(tag) ? JSON.stringify(tag) : tag))
    .join(', ');
  return `tags: [${formatted}]`;
}

export function patchOkfMarkdownTags(markdown: string, tags: string[]): string {
  const tagLine = formatOkfTagsLine(tags);
  if (!markdown.startsWith('---')) {
    return markdown;
  }

  const closingIndex = markdown.indexOf('\n---', 3);
  if (closingIndex < 0) {
    return markdown;
  }

  let frontmatter = markdown.slice(4, closingIndex);
  if (/^tags:\s/m.test(frontmatter)) {
    frontmatter = frontmatter.replace(/^tags:\s.*$/m, tagLine);
  } else {
    frontmatter = `${frontmatter.trimEnd()}\n${tagLine}\n`;
  }

  return `---\n${frontmatter}---${markdown.slice(closingIndex + 4)}`;
}

export async function updateOkfConceptTags(
  spaceId: string,
  conceptRowId: string,
  tags: string[],
  readMarkdown: (spaceId: string, conceptRowId: string) => Promise<string>,
): Promise<void> {
  const client = requireSdkClient();
  const concept = await client.knowledge.okf.concepts.retrieve(conceptRowId);
  const markdown = patchOkfMarkdownTags(
    await readMarkdown(spaceId, conceptRowId),
    tags,
  );

  await client.knowledge.okf.concepts.update({
    spaceId,
    conceptId: concept.conceptId,
    markdown,
    publish: true,
  });
}
