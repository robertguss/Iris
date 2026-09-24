import { z } from 'zod/v4';
import { checks } from './catalog.mjs';

export const checkId = z.enum(Object.keys(checks));
export const hash = z.string().regex(/^[a-f0-9]{64}$/);
export const scope = 'Only the listed checks ran. Events are test-observed checkpoints, not production traces. Raw process output is intentionally not exported.';
const status = z.enum(['passed', 'failed', 'blocked']);
export const resultSchema = z.object({
  id: checkId, status,
  reason: z.enum(['timeout', 'output_limit', 'prerequisite_unavailable', 'invariant_mismatch', 'check_failed', 'incomplete_evidence', 'completed']),
  exit_code: z.number().int().nullable(), duration_ms: z.number().nonnegative(),
  command: z.array(z.enum([...new Set(Object.values(checks).flatMap(c => c.command))])),
  sources: z.array(z.enum([...new Set(Object.values(checks).flatMap(c => c.sources))])),
  events: z.array(z.object({ sequence: z.number().int().positive(), event: z.enum(Object.values(checks).flatMap(c => c.events.map(e => e[0]))),
    expected: z.number().int(), observed: z.number().int(), matched: z.boolean() }).strict()),
  evidence_complete: z.boolean(),
  diagnostics: z.array(z.object({ code: z.string().regex(/^E\d{4}$/) }).strict()).max(100),
  reproduce: z.tuple([z.literal('node'), z.literal('experiments/agent-interface/cli.mjs'), z.literal('reproduce'), checkId]),
}).strict();
export const reportSchema = z.object({
  schema_version: z.literal(1), id: z.string().uuid(), created_at: z.iso.datetime(), source_digest: hash,
  source_changed_during_run: z.boolean(), lock_digest: hash,
  node_version: z.string().regex(/^v\d+\.\d+\.\d+$/),
  rust_version: z.string().regex(/^rustc \d+\.\d+\.\d+$/).nullable(),
  cargo_version: z.string().regex(/^cargo \d+\.\d+\.\d+$/).nullable(),
  status, plan: z.array(checkId).min(1), results: z.array(resultSchema), scope: z.literal(scope),
}).strict();
export const inspectionSchema = z.object({ report: reportSchema, stale: z.boolean() });
