#!/usr/bin/env node

import { createHash, generateKeyPairSync, randomBytes, sign, timingSafeEqual } from 'node:crypto';
import { createServer } from 'node:http';
import { pathToFileURL } from 'node:url';

const CLIENT_ID = 'iris-local';
const CODE_LIFETIME_MS = 2 * 60 * 1000;
const TOKEN_LIFETIME_SECONDS = 5 * 60;
const identities = Object.freeze({
  alice: Object.freeze({ sub: 'alice', name: 'Alice' }),
  bob: Object.freeze({ sub: 'bob', name: 'Bob' }),
});

function base64url(value) {
  return Buffer.from(value).toString('base64url');
}

function sha256(value) {
  return createHash('sha256').update(value).digest();
}

function sendJson(response, status, value) {
  const body = JSON.stringify(value);
  response.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'content-length': Buffer.byteLength(body),
    'cache-control': 'no-store',
  });
  response.end(body);
}

function oauthError(response, status, error, description) {
  sendJson(response, status, { error, error_description: description });
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[character]);
}

function readBody(request, limit = 16 * 1024) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    request.on('data', (chunk) => {
      size += chunk.length;
      if (size > limit) {
        reject(new Error('request body too large'));
        request.destroy();
      } else {
        chunks.push(chunk);
      }
    });
    request.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')));
    request.on('error', reject);
  });
}

function parseAuthorization(parameters, redirectUri) {
  const required = ['client_id', 'redirect_uri', 'response_type', 'code_challenge', 'code_challenge_method'];
  for (const name of required) {
    if (!parameters.get(name)) throw new Error(`missing ${name}`);
  }
  if (parameters.get('client_id') !== CLIENT_ID) throw new Error('unknown client_id');
  if (parameters.get('redirect_uri') !== redirectUri) throw new Error('invalid redirect_uri');
  if (parameters.get('response_type') !== 'code') throw new Error('response_type must be code');
  if (parameters.get('code_challenge_method') !== 'S256') throw new Error('code_challenge_method must be S256');
  if (!/^[A-Za-z0-9_-]{43,128}$/.test(parameters.get('code_challenge'))) {
    throw new Error('invalid code_challenge');
  }
}

function jwt(privateKey, kid, claims) {
  const header = base64url(JSON.stringify({ alg: 'RS256', typ: 'JWT', kid }));
  const payload = base64url(JSON.stringify(claims));
  const input = `${header}.${payload}`;
  return `${input}.${base64url(sign('RSA-SHA256', Buffer.from(input), privateKey))}`;
}

/** Start the local OIDC fixture. Pass port: 0 to let the OS choose a test port. */
export async function startOidcProvider({ port = 4000, issuer, redirectUri, testControls = false } = {}) {
  if (!redirectUri) throw new Error('redirectUri is required');
  const keyPair = generateKeyPairSync('rsa', { modulusLength: 2048 });
  const publicJwk = keyPair.publicKey.export({ format: 'jwk' });
  const kid = base64url(randomBytes(12));
  const codes = new Map();
  const controls = { nextFault: null, exchangeCount: 0, hold: false, release: null };

  const server = createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url, 'http://127.0.0.1');
      const effectiveIssuer = issuer ?? `http://127.0.0.1:${server.address().port}`;

      if (request.method === 'GET' && requestUrl.pathname === '/.well-known/openid-configuration') {
        return sendJson(response, 200, {
          issuer: effectiveIssuer,
          authorization_endpoint: `${effectiveIssuer}/authorize`,
          token_endpoint: `${effectiveIssuer}/token`,
          jwks_uri: `${effectiveIssuer}/jwks`,
          response_types_supported: ['code'],
          subject_types_supported: ['public'],
          id_token_signing_alg_values_supported: ['RS256'],
          grant_types_supported: ['authorization_code'],
          code_challenge_methods_supported: ['S256'],
          token_endpoint_auth_methods_supported: ['none'],
        });
      }

      if (request.method === 'GET' && requestUrl.pathname === '/jwks') {
        return sendJson(response, 200, { keys: [{ ...publicJwk, kid, use: 'sig', alg: 'RS256' }] });
      }

      if (request.method === 'GET' && requestUrl.pathname === '/authorize') {
        try {
          parseAuthorization(requestUrl.searchParams, redirectUri);
        } catch (error) {
          return oauthError(response, 400, 'invalid_request', error.message);
        }
        const hidden = [...requestUrl.searchParams].map(([name, value]) =>
          `<input type="hidden" name="${escapeHtml(name)}" value="${escapeHtml(value)}">`).join('\n');
        const body = `<!doctype html><html><head><meta charset="utf-8"><title>Local test identity provider</title></head>
<body><main><h1>Local test identity provider—not real authentication</h1>
<p>This development-only provider does not ask for or verify credentials. Choose a fixed test identity:</p>
<form method="post" action="/authorize">${hidden}
<button type="submit" name="identity" value="alice">Continue as Alice</button>
<button type="submit" name="identity" value="bob">Continue as Bob</button></form></main></body></html>`;
        response.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'content-length': Buffer.byteLength(body), 'cache-control': 'no-store' });
        return response.end(body);
      }

      if (request.method === 'POST' && requestUrl.pathname === '/authorize') {
        const parameters = new URLSearchParams(await readBody(request));
        try {
          parseAuthorization(parameters, redirectUri);
        } catch (error) {
          return oauthError(response, 400, 'invalid_request', error.message);
        }
        const identity = identities[parameters.get('identity')];
        if (!identity) return oauthError(response, 400, 'invalid_request', 'identity must be alice or bob');
        const code = base64url(randomBytes(32));
        codes.set(code, Object.freeze({
          sub: identity.sub,
          nonce: parameters.get('nonce') ?? undefined,
          challenge: parameters.get('code_challenge'),
          expiresAt: Date.now() + CODE_LIFETIME_MS,
        }));
        const destination = new URL(redirectUri);
        destination.searchParams.set('code', code);
        if (parameters.has('state')) destination.searchParams.set('state', parameters.get('state'));
        response.writeHead(303, { location: destination.href, 'cache-control': 'no-store' });
        return response.end();
      }

      if (request.method === 'POST' && requestUrl.pathname === '/token') {
        controls.exchangeCount += 1;
        const parameters = new URLSearchParams(await readBody(request));
        const codeValue = parameters.get('code');
        const record = codeValue && codes.get(codeValue);
        // Consume before any validation or asynchronous work: every presented valid code is one-use.
        if (record) codes.delete(codeValue);
        if (!record) return oauthError(response, 400, 'invalid_grant', 'unknown or already used code');
        if (Date.now() > record.expiresAt) return oauthError(response, 400, 'invalid_grant', 'authorization code expired');
        if (parameters.get('grant_type') !== 'authorization_code' ||
            parameters.get('client_id') !== CLIENT_ID || parameters.get('redirect_uri') !== redirectUri) {
          return oauthError(response, 400, 'invalid_grant', 'token request does not match authorization request');
        }
        const verifier = parameters.get('code_verifier') ?? '';
        const actualChallenge = base64url(sha256(verifier));
        const expected = Buffer.from(record.challenge);
        const actual = Buffer.from(actualChallenge);
        if (expected.length !== actual.length || !timingSafeEqual(expected, actual)) {
          return oauthError(response, 400, 'invalid_grant', 'PKCE verification failed');
        }

        const fault = controls.nextFault;
        controls.nextFault = null;
        if (controls.hold) {
          controls.hold = false;
          await new Promise(resolve => { controls.release = resolve; });
        }
        if (fault === 'missing_id_token') return sendJson(response, 200, { access_token: base64url(randomBytes(32)), token_type: 'Bearer', expires_in: TOKEN_LIFETIME_SECONDS });
        const now = Math.floor(Date.now() / 1000);
        const claims = {
          iss: fault === 'issuer' ? `${effectiveIssuer}/wrong` : effectiveIssuer,
          sub: record.sub,
          aud: fault === 'audience' ? 'wrong-client' : CLIENT_ID,
          iat: fault === 'expired' ? now - 600 : now,
          exp: fault === 'expired' ? now - 300 : now + TOKEN_LIFETIME_SECONDS,
          ...(record.nonce !== undefined ? { nonce: fault === 'nonce' ? `${record.nonce}-wrong` : record.nonce } : {}),
        };
        let signingKey = keyPair.privateKey;
        if (fault === 'wrong_signature') signingKey = generateKeyPairSync('rsa', { modulusLength: 2048 }).privateKey;
        return sendJson(response, 200, {
          access_token: base64url(randomBytes(32)), id_token: jwt(signingKey, kid, claims), token_type: 'Bearer', expires_in: TOKEN_LIFETIME_SECONDS,
        });
      }

      if (testControls && request.method === 'POST' && requestUrl.pathname === '/__test/next-id-token-fault') {
        const data = JSON.parse(await readBody(request));
        const allowed = ['wrong_signature', 'issuer', 'audience', 'nonce', 'expired', 'missing_id_token'];
        if (!allowed.includes(data.fault)) return oauthError(response, 400, 'invalid_request', 'unknown fault');
        controls.nextFault = data.fault;
        return sendJson(response, 200, { ok: true });
      }
      if (testControls && request.method === 'GET' && requestUrl.pathname === '/__test/stats') {
        return sendJson(response, 200, { exchange_count: controls.exchangeCount, waiting: controls.release !== null });
      }
      if (testControls && request.method === 'POST' && requestUrl.pathname === '/__test/hold') {
        controls.hold = true;
        return sendJson(response, 200, { ok: true });
      }
      if (testControls && request.method === 'POST' && requestUrl.pathname === '/__test/release') {
        controls.release?.();
        controls.release = null;
        return sendJson(response, 200, { ok: true });
      }
      response.writeHead(404).end();
    } catch (error) {
      if (!response.headersSent) oauthError(response, 400, 'invalid_request', error.message);
      else response.destroy();
    }
  });

  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', resolve);
  });
  return {
    server,
    issuer: issuer ?? `http://127.0.0.1:${server.address().port}`,
    port: server.address().port,
    close: () => new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve())),
  };
}

function commandLineOptions(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--test-controls') options.testControls = true;
    else if (['--port', '--issuer', '--redirect-uri'].includes(argument)) {
      if (!argv[index + 1]) throw new Error(`${argument} requires a value`);
      const key = { '--port': 'port', '--issuer': 'issuer', '--redirect-uri': 'redirectUri' }[argument];
      options[key] = argument === '--port' ? Number(argv[++index]) : argv[++index];
    } else throw new Error(`unknown argument: ${argument}`);
  }
  if (!Number.isInteger(options.port) || options.port < 0 || options.port > 65535) throw new Error('--port must be a valid port');
  if (!options.issuer) throw new Error('--issuer is required');
  if (!options.redirectUri) throw new Error('--redirect-uri is required');
  new URL(options.issuer);
  new URL(options.redirectUri);
  return options;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const provider = await startOidcProvider(commandLineOptions(process.argv.slice(2)));
    console.log(`Local test identity provider listening on 127.0.0.1:${provider.port}`);
  } catch (error) {
    console.error(`oidc-provider: ${error.message}`);
    process.exitCode = 1;
  }
}
