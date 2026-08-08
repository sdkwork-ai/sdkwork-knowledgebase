#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const baselinePath = path.join(
  root,
  'database/ddl/baseline/postgres/0001_knowledgebase_baseline.sql',
);
const sql = fs.readFileSync(baselinePath, 'utf8');
const seen = new Set();
const tableNames = [];
for (const match of sql.matchAll(/CREATE TABLE(?: IF NOT EXISTS)? ([a-z0-9_]+)/gi)) {
  const name = match[1];
  if (seen.has(name)) {
    continue;
  }
  seen.add(name);
  tableNames.push(name);
}

const tableRegistry = {
  schemaVersion: 1,
  kind: 'sdkwork.database.table-registry',
  tables: tableNames.map((table_name) => ({
    table_name,
    owner: 'knowledgebase-platform',
    compliance_level: 'L2',
    lifecycle_status: 'active',
  })),
};

const prefixRegistry = {
  schemaVersion: 1,
  kind: 'sdkwork.database.prefix-registry',
  prefixes: [{ prefix: 'kb_', owner: 'knowledgebase-platform', domain: 'knowledgebase' }],
};

const manifestPath = path.join(root, 'database/database.manifest.json');
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));

const schemaYaml = [
  'schema_version: 1',
  'kind: sdkwork.database.schema',
  'database_role: authoritative-server',
  'module_id: knowledgebase',
  `contract_version: ${manifest.contractVersion ?? '1.0.0'}`,
  'owner_team: knowledgebase-platform',
  'compliance_level: L2',
  'engines:',
  '  - postgres',
  'table_prefix: kb_',
  'tables:',
  ...tableNames.map(
    (name) =>
      `  - name: ${name}\n    lifecycle_status: active\n    owner: knowledgebase-platform`,
  ),
  '',
].join('\n');

fs.writeFileSync(
  path.join(root, 'database/contract/table-registry.json'),
  `${JSON.stringify(tableRegistry, null, 2)}\n`,
);
fs.writeFileSync(
  path.join(root, 'database/contract/prefix-registry.json'),
  `${JSON.stringify(prefixRegistry, null, 2)}\n`,
);
fs.writeFileSync(path.join(root, 'database/contract/schema.yaml'), schemaYaml);

// The manifest is the committed contract authority (postgres-only, autoMigrate=false);
// this tool must never rewrite it, otherwise a regeneration would silently downgrade the
// published contract.
process.stdout.write(
  `materialized ${tableNames.length} tables into knowledgebase database contract\n`,
);
