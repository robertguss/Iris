// The member directory's state as pure transitions: which read each user
// action requests, which results are accepted, and when a listing may predate
// an attempt. The page sends the requests and feeds results back through
// `received`; nothing here renders or fetches.
import type { ReadParams } from "./membership.ts";
import type { ReadPresentation, Row } from "./present.ts";

/** Rows per page; 1 lets the two seeded members span two pages. */
export const PAGE_SIZES = ["1", "10", "50"] as const;
export type PageSize = (typeof PAGE_SIZES)[number];

export type ListName = "projects" | "members";
export type List = {
  /** `"mine"`, or the project whose members these are. */
  key: string;
  /** The page size this list was read with. */
  limit: PageSize;
  /** The shown page's number, counted from the first page read. */
  page: number;
  /** The latest accepted read, or null before the first one completes. */
  shown: ReadPresentation | null;
  /** The read in flight: its token, the attempt count when it was sent, its page. */
  pending: { token: number; attempts: number; page: number } | null;
  /** An attempt was sent after the shown page was requested. */
  stale: boolean;
};
export type Directory = {
  limit: PageSize;
  /** The last token issued; never reset, so a token is never reused. */
  tokens: number;
  /** Attempts sent; never reset. */
  attempts: number;
  projects: List | null;
  project: { id: string; name: string } | null;
  members: List | null;
  member: Row | null;
};
export type Request =
  | {
      list: "projects";
      token: number;
      op: "listMyProjects";
      params: ReadParams<"listMyProjects">;
    }
  | {
      list: "members";
      token: number;
      op: "listProjectMembers";
      params: ReadParams<"listProjectMembers">;
    };
export type Step = { state: Directory; requests: Request[] };

export const initial: Directory = {
  limit: "10",
  tokens: 0,
  attempts: 0,
  projects: null,
  project: null,
  members: null,
  member: null,
};

const still = (state: Directory): Step => ({ state, requests: [] });

/** Requests one page of `name`; the first page when `cursor` is absent. */
function request(
  state: Directory,
  name: ListName,
  key: string,
  cursor?: string,
): Step {
  const token = state.tokens + 1;
  const current = state[name];
  const paging = cursor !== undefined && current !== null;
  const limit = paging ? current.limit : state.limit;
  const list: List = {
    key,
    limit,
    page: paging ? current.page : 0,
    // A first page replaces whatever was shown, which may be another key's.
    shown: paging ? current.shown : null,
    pending: {
      token,
      attempts: state.attempts,
      page: paging ? current.page + 1 : 1,
    },
    stale: paging ? current.stale : false,
  };
  const query = cursor === undefined ? { limit } : { limit, cursor };
  return {
    state: { ...state, tokens: token, [name]: list },
    requests: [
      name === "projects"
        ? { list: "projects", token, op: "listMyProjects", params: query }
        : {
            list: "members",
            token,
            op: "listProjectMembers",
            params: { project_id: key, ...query },
          },
    ],
  };
}

/** Both steps' requests, in order, ending in the second step's state. */
const then = (first: Step, second: (state: Directory) => Step): Step => {
  const after = second(first.state);
  return {
    state: after.state,
    requests: [...first.requests, ...after.requests],
  };
};

/** A session was established: read the caller's projects. */
export const signedIn = (state: Directory): Step =>
  request(
    { ...state, projects: null, project: null, members: null, member: null },
    "projects",
    "mine",
  );

/** The session ended: forget everything except the counters and page size. */
export const signedOut = (state: Directory): Step =>
  still({
    ...state,
    projects: null,
    project: null,
    members: null,
    member: null,
  });

/** Both lists start again from their first page at the new size. */
export function pageSize(state: Directory, limit: PageSize): Step {
  let step = still({ ...state, limit });
  if (state.projects)
    step = then(step, (s) => request(s, "projects", state.projects!.key));
  if (state.members)
    step = then(step, (s) => request(s, "members", state.members!.key));
  return step;
}

export const openProject = (state: Directory, id: string, name: string): Step =>
  request({ ...state, project: { id, name }, member: null }, "members", id);

/** The page after the shown one, if it had a cursor and nothing is pending. */
export function next(state: Directory, name: ListName): Step {
  const list = state[name];
  if (
    !list ||
    list.pending ||
    list.shown?.tone !== "listed" ||
    list.shown.next === null
  )
    return still(state);
  return request(state, name, list.key, list.shown.next);
}

/** The first page again: a new observation of present state. */
export function reload(state: Directory, name: ListName): Step {
  const list = state[name];
  return list ? request(state, name, list.key) : still(state);
}

/**
 * Accepts a read's presentation only for the request still pending on that
 * list; anything superseded is dropped. Never starts another read.
 */
export function received(
  state: Directory,
  name: ListName,
  token: number,
  shown: ReadPresentation,
): Step {
  const list = state[name];
  if (!list?.pending || list.pending.token !== token) return still(state);
  return still({
    ...state,
    [name]: {
      ...list,
      page: list.pending.page,
      shown,
      pending: null,
      // Ordering only: whether an attempt was sent after this read was
      // requested. It says nothing about what the read observed.
      stale: state.attempts !== list.pending.attempts,
    },
  });
}

/**
 * An attempt on a member of `projectId` was sent, whatever its outcome will
 * be: a shown listing of that project may predate it. Requests nothing.
 */
export function attempted(state: Directory, projectId: string): Step {
  const attempts = state.attempts + 1;
  const members =
    state.members?.key === projectId && state.members.shown
      ? { ...state.members, stale: true }
      : state.members;
  return still({ ...state, attempts, members });
}

export const selectMember = (state: Directory, member: Row): Step =>
  still({ ...state, member });
