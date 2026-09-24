import assert from 'node:assert/strict';
import { createHash, createPublicKey, verify } from 'node:crypto';
import { after, before, test } from 'node:test';
import { startOidcProvider } from './oidc-provider.mjs';

const redirectUri = 'http://127.0.0.1:5173/api/auth/callback';
const verifier = 'a'.repeat(48);
const challenge = createHash('sha256').update(verifier).digest('base64url');
let provider;
let base;

before(async () => {
  provider = await startOidcProvider({ port: 0, redirectUri, testControls: true });
  base = `http://127.0.0.1:${provider.port}`;
});
after(async () => provider.close());

function authorizationParameters(overrides = {}) {
  return new URLSearchParams({
    client_id: 'iris-local', redirect_uri: redirectUri, response_type: 'code',
    code_challenge: challenge, code_challenge_method: 'S256', state: 'state-1', nonce: 'nonce-1',
    ...overrides,
  });
}

async function issueCode(identity = 'alice') {
  const response = await fetch(`${base}/authorize`, {
    method: 'POST', redirect: 'manual',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: authorizationParameters({ identity }),
  });
  assert.equal(response.status, 303);
  const location = new URL(response.headers.get('location'));
  assert.equal(location.searchParams.get('state'), 'state-1');
  return location.searchParams.get('code');
}

async function exchange(code, codeVerifier = verifier) {
  return fetch(`${base}/token`, {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({
      grant_type: 'authorization_code', client_id: 'iris-local', redirect_uri: redirectUri,
      code, code_verifier: codeVerifier,
    }),
  });
}

test('discovery, authorization page, and JWKS expose the expected protocol', async () => {
  const discovery = await (await fetch(`${base}/.well-known/openid-configuration`)).json();
  assert.equal(discovery.issuer, base);
  assert.equal(discovery.token_endpoint_auth_methods_supported[0], 'none');
  assert.deepEqual(discovery.code_challenge_methods_supported, ['S256']);

  const page = await fetch(`${base}/authorize?${authorizationParameters()}`);
  assert.equal(page.status, 200);
  assert.match(await page.text(), /Local test identity provider—not real authentication/);
  const jwks = await (await fetch(`${base}/jwks`)).json();
  assert.equal(jwks.keys[0].alg, 'RS256');
  assert.equal(jwks.keys[0].kty, 'RSA');
});

test('authorization code exchange returns a verifiable ID token and is one-use', async () => {
  const code = await issueCode('bob');
  const response = await exchange(code);
  assert.equal(response.status, 200);
  const token = await response.json();
  assert.equal(token.token_type, 'Bearer');
  const [encodedHeader, encodedClaims, signature] = token.id_token.split('.');
  const claims = JSON.parse(Buffer.from(encodedClaims, 'base64url'));
  assert.deepEqual({ iss: claims.iss, sub: claims.sub, aud: claims.aud, nonce: claims.nonce }, {
    iss: base, sub: 'bob', aud: 'iris-local', nonce: 'nonce-1',
  });
  const jwks = await (await fetch(`${base}/jwks`)).json();
  assert.equal(verify('RSA-SHA256', Buffer.from(`${encodedHeader}.${encodedClaims}`),
    createPublicKey({ key: jwks.keys[0], format: 'jwk' }), Buffer.from(signature, 'base64url')), true);

  const replay = await exchange(code);
  assert.equal(replay.status, 400);
  assert.equal((await replay.json()).error, 'invalid_grant');
});

test('a failed PKCE attempt consumes the code', async () => {
  const code = await issueCode();
  assert.equal((await exchange(code, 'wrong-verifier')).status, 400);
  assert.equal((await exchange(code)).status, 400);
});

test('test controls inject one fault and observe exchanges', async () => {
  const control = await fetch(`${base}/__test/next-id-token-fault`, {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ fault: 'missing_id_token' }),
  });
  assert.equal(control.status, 200);
  const token = await (await exchange(await issueCode())).json();
  assert.equal('id_token' in token, false);
  const stats = await (await fetch(`${base}/__test/stats`)).json();
  assert.equal(stats.exchange_count, 5);
});

test('test routes are absent unless explicitly enabled', async () => {
  const plain = await startOidcProvider({ port: 0, redirectUri });
  try {
    assert.equal((await fetch(`http://127.0.0.1:${plain.port}/__test/stats`)).status, 404);
  } finally {
    await plain.close();
  }
});
