// Process ownership shared by the reference application's scripts: the browser
// runner (`browser.mjs`) and the development command (`dev.mjs`).
//
// Every child runs in its own process group, owned until the whole group is
// gone, not only its leader. A server is ready only when its own output
// reports its address, and its exit before cleanup begins stops everything,
// unless the caller expected it. Cleanup sends SIGTERM to every owned group,
// escalates to SIGKILL after 5 s and awaits every member's exit. There is
// one stop per process: its first cause is kept, and nothing starts once it
// has begun. Signal handling stays with each caller: `handleSignals` is the
// development command's.
import { spawn } from "node:child_process";
import { connect } from "node:net";

/** Whether something accepts connections on the port (within 2 s). */
export function listening(port) {
  return new Promise((done) => {
    const socket = connect({ host: "127.0.0.1", port });
    socket.once("connect", () => {
      socket.destroy();
      done(true);
    });
    socket.once("error", () => done(false));
    socket.setTimeout(2_000, () => {
      socket.destroy();
      done(true);
    });
  });
}

/**
 * Whether any process of the group remains. A group that refuses the probe
 * (EPERM) is treated as gone, since it cannot be signalled either.
 */
function alive(group) {
  if (!group) return false;
  try {
    process.kill(-group, 0);
    return true;
  } catch {
    return false;
  }
}
/** Resolves once no process of the group remains. */
async function emptied(group) {
  while (alive(group)) await new Promise((done) => setTimeout(done, 50));
}

export const describe = ({ error, code, signal }) =>
  error
    ? `failed to start (${error.code ?? error.message})`
    : `exited (${signal ?? code})`;

/** The cause of a stop that a signal requested. */
class Requested extends Error {}
/** The cause of a stop after the body finished its work. */
export class Completed extends Error {}

export class Supervisor {
  /**
   * With `report`, the supervisor prints its own lines (`dev: …`) to stderr:
   * each child's process group, the stop's cause, each exit during the stop
   * and the final exit code. Without it, it prints nothing.
   */
  constructor({ cwd, report = false } = {}) {
    this.cwd = cwd;
    this.report = report;
    // One stop signal for everything: a signal, or a server exiting early.
    this.stop = new AbortController();
    this.stopped = new Promise((_, reject) =>
      this.stop.signal.addEventListener(
        "abort",
        () => reject(this.stop.signal.reason),
        { once: true },
      ),
    );
    this.stopped.catch(() => {});
    this.owned = new Set();
    this.cleaning = false;
    // Children that needed SIGKILL during cleanup.
    this.killed = [];
    this.stop.signal.addEventListener("abort", () => {
      const reason = this.stop.signal.reason;
      if (
        this.owned.size > 0 &&
        !(reason instanceof Requested) &&
        !(reason instanceof Completed)
      )
        this.say(`stopping: ${reason.message}`);
    });
  }

  say(line) {
    if (this.report) console.error(`dev: ${line}`);
  }

  /**
   * Persistent SIGINT and SIGTERM handlers: the first signal begins the stop;
   * a later one neither ends nor shortens it.
   */
  handleSignals() {
    for (const signal of ["SIGINT", "SIGTERM"])
      process.on(signal, () => {
        if (this.stop.signal.aborted)
          this.say(
            `received ${signal}; already stopping, waiting for children`,
          );
        else {
          this.say(`received ${signal}; stopping`);
          this.stop.abort(new Requested(`received ${signal}`));
        }
      });
  }

  /** Spawns a child in its own process group and owns it until it exits. */
  own(command, args, options = {}, name = undefined) {
    const child = spawn(command, args, {
      cwd: this.cwd,
      detached: true,
      ...options,
    });
    let running = true;
    const exited = new Promise((done) => {
      child.once("error", (error) => {
        running = false;
        done({ error });
      });
      child.once("exit", (code, signal) => {
        running = false;
        done({ code, signal });
      });
    });
    const entry = { child, exited, name, running: () => running };
    // The group stays owned after its leader exits, until its last member
    // has: a descendant left behind is still this supervisor's to stop.
    entry.gone = exited.then(() => emptied(child.pid));
    this.owned.add(entry);
    entry.gone.then(() => this.owned.delete(entry));
    if (name && child.pid) this.say(`started ${name} (pid ${child.pid})`);
    return entry;
  }

  signalGroup({ child, running }, signal) {
    try {
      process.kill(-child.pid, signal);
    } catch {
      if (running()) child.kill(signal);
    }
  }

  /**
   * SIGTERM to the group, then SIGKILL after 5 s if any member remains;
   * resolves with the leader's exit once the whole group is gone.
   */
  async terminate(entry) {
    this.signalGroup(entry, "SIGTERM");
    const force = setTimeout(() => {
      if (!entry.running() && !alive(entry.child.pid)) return;
      if (entry.name) {
        this.killed.push(entry.name);
        this.say(
          `${entry.name} did not exit within 5 s of SIGTERM; sent SIGKILL`,
        );
      }
      this.signalGroup(entry, "SIGKILL");
    }, 5_000);
    const result = await entry.exited;
    await entry.gone;
    clearTimeout(force);
    return result;
  }

  /**
   * A server: its output goes to `output` and is scanned for the child's own
   * readiness line; its exit before cleanup stops everything, unless
   * `expected()` says otherwise. `note` follows the name in the messages.
   * Returns the owned server and its readiness.
   */
  start(
    name,
    command,
    args,
    readiness,
    {
      output,
      expected = () => false,
      note = "",
      readyTimeout = 60_000,
      ...options
    } = {},
  ) {
    // Checked just before spawning: nothing starts once a stop has begun.
    if (this.stop.signal.aborted) throw this.stop.signal.reason;
    const server = this.own(
      command,
      args,
      { stdio: ["ignore", "pipe", "pipe"], ...options },
      name,
    );
    server.exited.then((result) => {
      if (!this.cleaning && !expected(server))
        this.stop.abort(new Error(`${name} ${describe(result)}${note}`));
    });
    let seen = "";
    const ready = new Promise((done, fail) => {
      const timer = setTimeout(
        () =>
          fail(
            new Error(
              `${name} not ready within ${readyTimeout / 1000} s${note}`,
            ),
          ),
        readyTimeout,
      );
      const scan = (chunk, stream) => {
        output(chunk, stream);
        if (seen === null) return;
        // Colour codes, if any, never split the readiness line.
        seen += String(chunk).replace(/\x1b\[[0-9;]*m/g, "");
        if (readiness.test(seen)) {
          seen = null;
          clearTimeout(timer);
          done();
        }
      };
      server.child.stdout?.on("data", (chunk) => scan(chunk, "stdout"));
      server.child.stderr?.on("data", (chunk) => scan(chunk, "stderr"));
      server.exited.then(() => clearTimeout(timer));
    });
    return { server, ready: Promise.race([ready, this.stopped]) };
  }

  /**
   * Starts each server only once the one before it is ready, with its output
   * printed line by line under its name. Resolves when the last is ready.
   */
  async startInOrder(servers, readyTimeout) {
    for (const { name, command, args, readiness, options = {} } of servers) {
      const { ready } = this.start(name, command, args, readiness, {
        output: prefixed(name),
        readyTimeout,
        ...options,
      });
      await ready;
    }
  }

  /** Stops every owned child and awaits its exit. */
  async cleanup() {
    this.cleaning = true;
    await Promise.all(
      [...this.owned].map(async (entry) => {
        const result = await this.terminate(entry);
        if (entry.name)
          this.say(`${entry.name} ${describe(result)} during the stop`);
      }),
    );
  }

  /**
   * Runs `body` under this supervisor and returns the exit code: 0 only for a
   * body that completed or (with `signalled` set) a stop a signal requested,
   * in which every child exited within 5 s of SIGTERM, whatever their own exit
   * codes; 1 otherwise. The first cause of the stop is kept and printed last.
   */
  async run(body, { signalled = true } = {}) {
    try {
      await body();
    } catch (error) {
      // No effect if the stop has already begun: the first cause wins.
      this.stop.abort(error);
    }
    await this.cleanup();
    const cause = this.stop.signal.reason;
    const clean =
      ((signalled && cause instanceof Requested) ||
        cause instanceof Completed) &&
      this.killed.length === 0;
    const needed = this.killed.length
      ? `; ${this.killed.join(", ")} needed SIGKILL`
      : "";
    const code = clean ? 0 : 1;
    this.say(`exit ${code}: ${cause?.message ?? "stopped"}${needed}`);
    return code;
  }
}

/**
 * Writes a child's output to stderr line by line under its name, keeping each
 * stream's unfinished line apart.
 */
export function prefixed(name) {
  const partial = { stdout: "", stderr: "" };
  return (chunk, stream) => {
    const lines = (partial[stream] + chunk).split("\n");
    partial[stream] = lines.pop();
    for (const line of lines) process.stderr.write(`[${name}] ${line}\n`);
  };
}

/**
 * Starts the servers in order, announces readiness once all are ready, and
 * serves until the stop.
 */
export async function runSupervised(
  supervisor,
  servers,
  { readyTimeout = 60_000, ready },
) {
  await Promise.race([
    supervisor.startInOrder(servers, readyTimeout),
    supervisor.stopped,
  ]);
  supervisor.say(`ready: ${ready}`);
  await supervisor.stopped;
}
