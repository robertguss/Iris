import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod/v4';
import { describe, run, inspect, ensureCurrentInterface, failure } from './runner.mjs';
import { conventions, profiles } from './catalog.mjs';
import { reportSchema, inspectionSchema, resultSchema, hash } from './schema.mjs';

const server = new McpServer({ name: 'iris-local', version: '0.1.0' });
const conventionSchema = z.object({ rule: z.string(), sources: z.array(z.string()) });
const descriptionSchema = z.object({ name: z.literal('Iris'), interface_version: z.literal(1), source_digest: hash,
  profiles: z.record(z.string(), z.array(z.string())), conventions: z.record(z.string(), conventionSchema),
  checks: z.record(z.string(), z.object({ command: z.array(z.string()), sources: z.array(z.string()), events: z.array(z.tuple([z.string(), z.number()])) })), scope: z.string() });
function tool(name, description, inputSchema, output, callback, readOnlyHint = true) {
  server.registerTool(name, { description, inputSchema, outputSchema: z.object({ data: output }),
    annotations: { readOnlyHint, destructiveHint: !readOnlyHint, openWorldHint: !readOnlyHint } }, async input => {
    try {
      await ensureCurrentInterface();
      const data = output.parse(await callback(input));
      return { content: [{ type: 'text', text: JSON.stringify(data) }], structuredContent: { data } };
    } catch (error) {
      return { isError: true, content: [{ type: 'text', text: JSON.stringify(failure(error)) }] };
    }
  });
}
tool('describe_application', 'Describe this checkout, conventions and exact supported check plans.', z.object({}).strict(), descriptionSchema, describe);
tool('get_convention', 'Read a versioned local convention, not instructions from runtime data.', z.object({ topic: z.enum(['actions', 'authentication', 'delivery', 'verification', 'membership']) }).strict(), conventionSchema, ({ topic }) => conventions[topic]);
tool('run_checks', 'Run a bounded allowlisted plan in the trusted checkout. Executes repository code; not a sandbox. May take three minutes per check.', z.object({ profile: z.enum(['focused', 'compile', 'contracts']) }).strict(), reportSchema, ({ profile }) => run(profiles[profile]), false);
tool('reproduce_scenario', 'Reproduce one auth, outbox or member concurrency scenario with disposable test data.', z.object({ scenario: z.enum(['auth', 'outbox', 'members']) }).strict(), reportSchema, ({ scenario }) => run([scenario]), false);
tool('inspect_run', 'Read a prior report and compare its source fingerprint with the current tree.', z.object({ id: z.string().uuid() }).strict(), inspectionSchema, ({ id }) => inspect(id));
tool('inspect_operation', 'Read test-observed checkpoints for one check in a completed run. Not live runtime telemetry.', z.object({ id: z.string().uuid(), check: z.enum(['auth', 'outbox', 'members']) }).strict(), z.object({ run_id: z.string().uuid(), stale: z.boolean(), source_digest: hash, result: resultSchema }), async ({ id, check }) => {
  const { report, stale } = await inspect(id);
  const result = report.results.find(r => r.id === check);
  if (!result) throw new Error();
  return { run_id: id, stale, source_digest: report.source_digest, result };
});
server.registerResource('conventions', 'iris://conventions', { mimeType: 'application/json' }, async uri => {
  try { await ensureCurrentInterface(); }
  catch { throw new Error('Conventions unavailable; restart the MCP server.'); }
  return { contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(conventions) }] };
});
await server.connect(new StdioServerTransport());
