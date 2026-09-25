// Injected only by the browser smoke test, before openapi-fetch captures fetch.
// Holds a real request to inspect loading, or simulates a transport failure.
(() => {
  const original = window.fetch.bind(window);
  window.irisTest = { hold: false, fail: false, requests: 0, release: null };
  window.fetch = async (...args) => {
    const url = args[0] instanceof Request ? args[0].url : String(args[0]);
    if (url.endsWith('/api/invitations/accept') || url.endsWith('/api/invitations') || url.endsWith('/api/memberships/role') || url.endsWith('/api/memberships/remove')) {
      window.irisTest.requests++;
      if (window.irisTest.hold) {
        await new Promise(resolve => { window.irisTest.release = resolve; });
      }
      if (window.irisTest.fail) throw new TypeError('Simulated network failure');
    }
    return original(...args);
  };
})();
