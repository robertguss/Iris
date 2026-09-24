import { describe, run, inspect, failure } from './runner.mjs';
import { profiles, checks } from './catalog.mjs';

const [verb, name, ...extra] = process.argv.slice(2);
try {
  if (extra.length) throw new Error();
  let result;
  if (verb === 'describe' && !name) result = await describe();
  else if (verb === 'inspect') result = await inspect(name ?? '');
  else if (verb === 'run' && Object.hasOwn(profiles, name)) result = await run(profiles[name]);
  else if (verb === 'reproduce' && Object.hasOwn(checks, name)) result = await run([name]);
  else throw new Error();
  process.stdout.write(JSON.stringify(result, null, 2) + '\n');
  if (result.status && result.status !== 'passed') process.exitCode = 1;
} catch (error) {
  process.stdout.write(JSON.stringify(failure(error)) + '\n');
  process.exitCode = 2;
}
