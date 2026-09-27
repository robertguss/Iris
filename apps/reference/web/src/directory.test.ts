// The member directory's paging and pairing rules, written by hand: which
// read each transition requests, which results it accepts, and when a listing
// may predate an attempt. Runs under Node; nothing here renders.
import assert from "node:assert/strict";
import {
  attempted,
  initial,
  next,
  openProject,
  pageSize,
  received,
  reload,
  selectMember,
  signedIn,
  signedOut,
} from "./directory.ts";
import type { Directory, Request, Step } from "./directory.ts";
import type { ReadPresentation, Row } from "./present.ts";

const listed = (ids: string[], next: string | null): ReadPresentation => ({
  tone: "listed",
  title: "Listed",
  detail: "Present state when read.",
  rows: ids.map((id) => ({ id, name: `Name ${id}`, role: "editor" })),
  next,
});
const failed: ReadPresentation = {
  tone: "unavailable",
  title: "No usable response",
  detail: "Not loaded.",
};
/** The single request a step makes. */
function only(step: Step): Request {
  assert.equal(step.requests.length, 1, "exactly one read");
  return step.requests[0];
}
/** Accepts `presentation` for `request`, which must not request anything. */
function answer(state: Directory, request: Request, shown: ReadPresentation) {
  const step = received(state, request.list, request.token, shown);
  assert.deepEqual(step.requests, [], "a result never starts another read");
  return step.state;
}

let checks = 0;
/** Runs one named case; a failure names it. */
function check(name: string, run: () => void) {
  try {
    run();
  } catch (error) {
    throw new Error(`${name}: ${String(error)}`, { cause: error });
  }
  checks++;
}

check("signing in reads the first page of the caller's projects", () => {
  const step = signedIn(initial);
  assert.deepEqual(only(step), {
    list: "projects",
    token: 1,
    op: "listMyProjects",
    params: { limit: "10" },
  });
  assert.equal(step.state.projects?.shown, null);
  assert.equal(step.state.members, null);
});

check("a shown page with a cursor reads the next page with it", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41"], "c1.41"));
  assert.equal(state.projects?.page, 1);
  const step = next(state, "projects");
  assert.deepEqual(only(step), {
    list: "projects",
    token: 2,
    op: "listMyProjects",
    params: { limit: "10", cursor: "c1.41" },
  });
  state = answer(step.state, only(step), listed(["43"], null));
  assert.equal(state.projects?.page, 2);
  assert.deepEqual(
    state.projects?.shown?.tone === "listed" && state.projects.shown.rows,
    [{ id: "43", name: "Name 43", role: "editor" }],
  );
});

check("no next page at the end, while loading, or after a failure", () => {
  const signed = signedIn(initial);
  // Loading: nothing is shown yet.
  assert.deepEqual(next(signed.state, "projects").requests, []);
  const end = answer(signed.state, only(signed), listed(["41"], null));
  assert.deepEqual(next(end, "projects").requests, []);
  const down = answer(signed.state, only(signed), failed);
  assert.deepEqual(next(down, "projects").requests, []);
  assert.equal(down.projects?.shown?.tone, "unavailable");
  // A page already requested is not requested twice.
  const more = answer(signed.state, only(signed), listed(["41"], "c1.41"));
  const pending = next(more, "projects").state;
  assert.deepEqual(next(pending, "projects").requests, []);
  // Nothing to page before signing in or before a project is open.
  assert.deepEqual(next(initial, "projects").requests, []);
  assert.deepEqual(next(end, "members").requests, []);
});

check("reload reads the first page again, at page 1", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41"], "c1.41"));
  const second = next(state, "projects");
  state = answer(second.state, only(second), listed(["43"], null));
  const again = reload(state, "projects");
  assert.deepEqual(only(again).params, { limit: "10" });
  state = answer(again.state, only(again), listed(["41"], "c1.41"));
  assert.equal(state.projects?.page, 1);
  // A failed read stays as shown until the user reloads.
  const retry = reload(state, "projects");
  const down = answer(retry.state, only(retry), failed);
  assert.equal(down.projects?.shown?.tone, "unavailable");
  assert.equal(only(reload(down, "projects")).op, "listMyProjects");
});

check("opening a project reads its members and clears the selection", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41", "43"], null));
  const open = openProject(state, "41", "Launch plan");
  assert.deepEqual(only(open), {
    list: "members",
    token: 2,
    op: "listProjectMembers",
    params: { project_id: "41", limit: "10" },
  });
  state = answer(open.state, only(open), listed(["11", "29"], null));
  const bob: Row = { id: "29", name: "Bob Example", role: "editor" };
  state = selectMember(state, bob).state;
  assert.deepEqual(state.member, bob);
  const other = openProject(state, "43", "Field notes");
  assert.equal(other.state.member, null);
  assert.deepEqual(other.state.project, { id: "43", name: "Field notes" });
  assert.equal(other.state.members?.shown, null, "no rows from project 41");
  assert.deepEqual(only(other).params, { project_id: "43", limit: "10" });
});

check("a page-size change reads both lists' first pages again", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41"], "c1.41"));
  const open = openProject(state, "41", "Launch plan");
  state = answer(open.state, only(open), listed(["11"], "c1.11"));
  const second = next(state, "members");
  state = answer(second.state, only(second), listed(["29"], null));
  const bob: Row = { id: "29", name: "Bob Example", role: "editor" };
  state = selectMember(state, bob).state;
  const resized = pageSize(state, "1");
  assert.deepEqual(
    resized.requests.map(({ list, op, params }) => ({ list, op, params })),
    [
      { list: "projects", op: "listMyProjects", params: { limit: "1" } },
      {
        list: "members",
        op: "listProjectMembers",
        params: { project_id: "41", limit: "1" },
      },
    ],
  );
  assert.deepEqual(resized.state.project, { id: "41", name: "Launch plan" });
  assert.deepEqual(resized.state.member, bob, "the selection is kept");
  const [projects, members] = resized.requests;
  state = answer(resized.state, members, listed(["11"], "c1.11"));
  state = answer(state, projects, listed(["41"], null));
  assert.equal(state.members?.page, 1);
  assert.equal(state.members?.limit, "1");
  assert.deepEqual(only(next(state, "members")).params, {
    project_id: "41",
    limit: "1",
    cursor: "c1.11",
  });
  // Signed out, a page-size change only records the size.
  const quiet = pageSize(initial, "50");
  assert.deepEqual(quiet.requests, []);
  assert.equal(only(signedIn(quiet.state)).params.limit, "50");
});

check("a superseded result is dropped", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41", "43"], null));
  // Project switched: 41's late members never appear under 43.
  const first = openProject(state, "41", "Launch plan");
  const second = openProject(first.state, "43", "Field notes");
  const late = received(
    second.state,
    "members",
    only(first).token,
    listed(["11"], null),
  );
  assert.deepEqual(late.state, second.state);
  state = answer(second.state, only(second), listed(["29"], null));
  // Reloaded, resized, or signed out and in again: the earlier read is stale.
  const before = reload(state, "members");
  const after = reload(before.state, "members");
  assert.deepEqual(
    received(after.state, "members", only(before).token, failed).state,
    after.state,
  );
  const resized = pageSize(before.state, "1");
  assert.deepEqual(
    received(resized.state, "members", only(before).token, failed).state,
    resized.state,
  );
  const out = signedOut(before.state).state;
  const back = signedIn(out);
  assert.deepEqual(
    received(back.state, "projects", only(signed).token, failed).state,
    back.state,
  );
  // A result is only ever accepted by the list that requested it.
  const open = openProject(state, "41", "Launch plan");
  const crossed = received(
    open.state,
    "projects",
    only(open).token,
    listed(["11"], null),
  );
  assert.deepEqual(crossed.state, open.state);
});

check("tokens are never reused, across lists and sessions", () => {
  const seen = new Set<number>();
  const record = (step: Step) => {
    for (const { token } of step.requests) {
      assert.ok(!seen.has(token), `token ${token} reused`);
      seen.add(token);
    }
    return step.state;
  };
  let state = record(signedIn(initial));
  state = record(openProject(state, "41", "Launch plan"));
  state = record(pageSize(state, "1"));
  state = signedOut(state).state;
  state = record(signedIn(state));
  state = record(openProject(state, "41", "Launch plan"));
  record(reload(state, "members"));
  assert.equal(seen.size, 7);
});

check("an attempt marks its project's listing at send time", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41"], null));
  const open = openProject(state, "41", "Launch plan");
  state = answer(open.state, only(open), listed(["11", "29"], null));
  assert.equal(state.members?.stale, false);
  // Sent, whatever its outcome later (including unconfirmed): no read.
  const step = attempted(state, "41");
  assert.deepEqual(step.requests, []);
  assert.equal(step.state.members?.stale, true);
  assert.equal(step.state.projects?.stale, false);
  // Another project's listing is not marked.
  assert.equal(attempted(state, "43").state.members?.stale, false);
  // A read requested after the attempt is not stale.
  const again = reload(step.state, "members");
  state = answer(again.state, only(again), listed(["11"], null));
  assert.equal(state.members?.stale, false);
});

check("a read pending across an attempt completes as stale", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41"], null));
  const open = openProject(state, "41", "Launch plan");
  state = answer(open.state, only(open), listed(["11"], "c1.11"));
  // Requested before the attempt, answered after it.
  const pending = reload(state, "members");
  state = attempted(pending.state, "41").state;
  state = answer(state, only(pending), listed(["11"], "c1.11"));
  assert.equal(state.members?.stale, true);
  // A next page requested after the attempt is not stale.
  const later = next(state, "members");
  state = answer(later.state, only(later), listed(["29"], null));
  assert.equal(state.members?.stale, false);
  // Two attempts while one read is pending: still stale.
  const again = reload(state, "members");
  state = attempted(attempted(again.state, "41").state, "41").state;
  state = answer(state, only(again), listed(["11"], null));
  assert.equal(state.members?.stale, true);
});

check("signing out clears everything but the counters", () => {
  const signed = signedIn(initial);
  let state = answer(signed.state, only(signed), listed(["41"], null));
  const open = openProject(state, "41", "Launch plan");
  state = selectMember(open.state, {
    id: "29",
    name: "Bob Example",
    role: "editor",
  }).state;
  state = attempted(pageSize(state, "50").state, "41").state;
  const out = signedOut(state);
  assert.deepEqual(out.requests, []);
  assert.equal(out.state.projects, null);
  assert.equal(out.state.project, null);
  assert.equal(out.state.members, null);
  assert.equal(out.state.member, null);
  assert.equal(out.state.limit, "50", "the page size is a preference");
  assert.ok(out.state.tokens >= state.tokens);
  assert.equal(out.state.attempts, state.attempts);
});

console.log(
  `PASS: ${checks} directory transitions; results pair with their request; listings may predate attempts`,
);
