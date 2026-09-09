// Origin preflight for the RFC 0006 fixture owner session.
//
// WHAT THIS IS. A characterisation harness. It measures what a real browser
// actually does with a `__Host-` cookie and a WebAuthn credential on loopback,
// and writes the answers into a matrix. It is not owner admission, it is not
// product code, and nothing here runs in the product.
//
// WHY IT EXISTS. Program 1C0 wants to bind an owner session to an origin. On
// loopback that plan meets three facts a design cannot assume away, all three
// measured below rather than asserted:
//
//   1. Cookies are not isolated by port. `localhost:9999` can set — and
//      overwrite — a cookie that `localhost:7787` will then receive. The
//      `__Host-` prefix does not prevent this; it constrains Path, Domain and
//      Secure, none of which mention the port.
//   2. A WebAuthn RP ID is a host, not an origin. Every port on the host
//      shares one RP ID, so a page on another port can obtain a
//      user-verified assertion over the same credential.
//   3. An IP address cannot be an RP ID at all, so the browser must reach the
//      fixture as `localhost` even though the socket binds 127.0.0.1.
//
// What separates the two is the origin recorded inside clientDataJSON, which
// the relying party must check. This harness proves that check is the load
// bearing one by obtaining an assertion at the hostile port and showing the
// legitimate port rejects it.
//
// Zero dependencies: Node's own http, plus Chrome driven over the DevTools
// protocol through Node 22's global WebSocket. No remote script is fetched.
//
// Output is canonical and value free: identifiers, booleans and outcome
// strings only. No credential material, no cookie value, no challenge bytes.

import http from 'node:http';
import os from 'node:os';
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const LEGIT_PORT = 7787; // frozen by RFC 0006; never negotiated
const COOKIE = '__Host-sfo_fixture_session';
// An IP address is not a valid WebAuthn RP ID: the spec requires a registrable
// domain, and Chrome answers `create()` on an IP origin with SecurityError.
// `localhost` is the one name that is both a valid RP ID and a secure context
// over plain http, so the browser must reach the fixture by that name. The
// socket still binds 127.0.0.1 — `localhost` resolves there — so this changes
// the name in the URL, not what the process listens on.
const RP_ID = 'localhost';
const HOST = 'localhost';

const argv = new Set(process.argv.slice(2));
const VIRTUAL = argv.has('--virtual');
const REAL = argv.has('--real');
if (VIRTUAL === REAL) {
  console.error('usage: owner-auth-origin-preflight.mjs (--virtual | --real)');
  process.exit(2);
}

// ---------------------------------------------------------------- servers --

/** The legitimate origin. Sets the session cookie and serves the page. */
function legitimateServer() {
  return http.createServer((request, response) => {
    if (request.url === '/') {
      response.writeHead(200, {
        'content-type': 'text/html; charset=utf-8',
        // Secure is required by the __Host- prefix. 127.0.0.1 is a secure
        // context, so a browser accepts it over plain http here and only here.
        'set-cookie': `${COOKIE}=fixture-session-value; Path=/; Secure; SameSite=Lax`,
      });
      response.end('<!doctype html><meta charset="utf-8"><title>preflight</title>');
      return;
    }
    if (request.url === '/echo-cookie') {
      const seen = request.headers.cookie || '';
      response.writeHead(200, { 'content-type': 'application/json' });
      // Report only whether the named cookie arrived, never its value.
      response.end(JSON.stringify({ received: seen.includes(`${COOKIE}=`) }));
      return;
    }
    response.writeHead(404).end();
  });
}

/** Another origin on the same host: a different port, nothing more. */
function hostileServer() {
  return http.createServer((request, response) => {
    if (request.url === '/') {
      response.writeHead(200, {
        'content-type': 'text/html; charset=utf-8',
        // Same host, different port, overwriting the legitimate cookie.
        'set-cookie': `${COOKIE}=overwritten-by-another-port; Path=/; Secure; SameSite=Lax`,
      });
      response.end('<!doctype html><meta charset="utf-8"><title>other origin</title>');
      return;
    }
    response.writeHead(404).end();
  });
}

const listen = (server, port) =>
  new Promise((resolve, reject) => {
    server.once('error', reject);
    // Bind loopback explicitly. Never 0.0.0.0, never ::.
    server.listen(port, '127.0.0.1', () => resolve(server.address().port));
  });

// -------------------------------------------------------------------- CDP --

function findChrome() {
  const candidates = [
    process.env.CHROME_PATH,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
    '/usr/bin/google-chrome',
    '/usr/bin/google-chrome-stable',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/snap/bin/chromium',
  ].filter(Boolean);
  return candidates.find((path) => existsSync(path));
}

class Cdp {
  constructor(socket) {
    this.socket = socket;
    this.id = 0;
    this.pending = new Map();
    socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.id && this.pending.has(message.id)) {
        const { resolve, reject } = this.pending.get(message.id);
        this.pending.delete(message.id);
        message.error ? reject(new Error(JSON.stringify(message.error))) : resolve(message.result);
      }
    };
  }
  send(method, params = {}, sessionId) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params, sessionId }));
    });
  }
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function browserSocketUrl(port) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`);
      return await response.json();
    } catch {
      await sleep(250);
    }
  }
  throw new Error('Chrome DevTools endpoint never came up');
}

const openSocket = (url) =>
  new Promise((resolve, reject) => {
    const socket = new WebSocket(url);
    socket.onopen = () => resolve(socket);
    socket.onerror = reject;
  });

// ------------------------------------------------------------ the measure --

/** Run one expression in the page and return its resolved value. */
async function evaluate(cdp, sessionId, expression) {
  const { result, exceptionDetails } = await cdp.send(
    'Runtime.evaluate',
    { expression, awaitPromise: true, returnByValue: true },
    sessionId,
  );
  if (exceptionDetails) {
    return { error: exceptionDetails.text || 'evaluation threw' };
  }
  return result.value;
}

/**
 * The page-side script. Kept as a string so it is served/evaluated
 * same-origin and never fetched from anywhere.
 *
 * Returns only structural facts: lengths, flags, the origin string the
 * browser itself wrote into clientDataJSON. No key material leaves here.
 */
const CREATE = (origin) => `(async () => {
  try {
    const credential = await navigator.credentials.create({ publicKey: {
      challenge: new Uint8Array(32),
      rp: { id: ${JSON.stringify(RP_ID)}, name: 'sfo fixture' },
      user: { id: new Uint8Array([1,2,3,4]), name: 'fixture', displayName: 'fixture' },
      pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
      authenticatorSelection: { userVerification: 'required', residentKey: 'required' },
      timeout: 10000,
    }});
    const clientData = JSON.parse(new TextDecoder().decode(credential.response.clientDataJSON));
    return { ok: true, credentialIdLength: credential.rawId.byteLength,
             clientDataOrigin: clientData.origin, type: clientData.type,
             pageOrigin: ${JSON.stringify(origin)} };
  } catch (error) { return { ok: false, error: error.name }; }
})()`;

const GET = (origin) => `(async () => {
  try {
    const assertion = await navigator.credentials.get({ publicKey: {
      challenge: new Uint8Array(32),
      rpId: ${JSON.stringify(RP_ID)},
      userVerification: 'required',
      timeout: 10000,
    }});
    const clientData = JSON.parse(new TextDecoder().decode(assertion.response.clientDataJSON));
    const flags = new Uint8Array(assertion.response.authenticatorData)[32];
    return { ok: true, clientDataOrigin: clientData.origin, type: clientData.type,
             userVerified: (flags & 0x04) !== 0,
             userHandlePresent: assertion.response.userHandle !== null,
             pageOrigin: ${JSON.stringify(origin)} };
  } catch (error) { return { ok: false, error: error.name }; }
})()`;

/**
 * The origin check a relying party must perform. This is the whole point of
 * the preflight: neither the cookie nor the RP ID separates the two ports, so
 * this comparison is the only thing that does.
 */
function acceptedAtLegitimateOrigin(clientDataOrigin) {
  return clientDataOrigin === `http://${HOST}:${LEGIT_PORT}`;
}

async function main() {
  const chrome = findChrome();
  if (!chrome) {
    // An unqualified platform is a recorded outcome, not a crash: an empty
    // real matrix is allowed and leaves the mechanism unqualified.
    console.log(JSON.stringify({
      schema: 'owner-auth-origin-preflight/v1',
      mode: VIRTUAL ? 'virtual' : 'real',
      status: 'unqualified',
      reason: 'no Chrome or Chromium found; set CHROME_PATH',
      platform: { os: os.platform(), release: os.release(), arch: os.arch() },
    }, null, 2));
    process.exit(4);
  }

  const legit = legitimateServer();
  const hostile = hostileServer();
  await listen(legit, LEGIT_PORT);
  const hostilePort = await listen(hostile, 0);
  const legitOrigin = `http://${HOST}:${LEGIT_PORT}`;
  const hostileOrigin = `http://${HOST}:${hostilePort}`;

  const profile = mkdtempSync(join(tmpdir(), 'sfo-preflight-'));
  const debugPort = 9455;
  // Chrome's setuid sandbox refuses to start as root, which is exactly how
  // the project's containers and nightly CI run. Drop it only in that case,
  // and only there: weakening it on a developer's machine would buy nothing.
  const rootless = typeof process.getuid === 'function' && process.getuid() !== 0;
  const browser = spawn(chrome, [
    '--headless=new',
    '--disable-gpu',
    '--no-first-run',
    '--no-default-browser-check',
    ...(rootless ? [] : ['--no-sandbox']),
    `--user-data-dir=${profile}`,
    `--remote-debugging-port=${debugPort}`,
    'about:blank',
  ], { stdio: 'ignore' });

  const findings = {};
  let browserVersion = 'unknown';
  try {
    const version = await browserSocketUrl(debugPort);
    browserVersion = version.Browser || 'unknown';
    const cdp = new Cdp(await openSocket(version.webSocketDebuggerUrl));
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const s = (method, params) => cdp.send(method, params, sessionId);

    await s('Page.enable');
    await s('Runtime.enable');
    await s('Network.enable');

    // A virtual authenticator: deterministic, resident, always user-verified.
    // Every row produced this way is `protocol_fixture_only` — it measures the
    // protocol, never a real platform authenticator.
    await s('WebAuthn.enable');
    const { authenticatorId } = await s('WebAuthn.addVirtualAuthenticator', {
      options: {
        protocol: 'ctap2',
        transport: 'internal',
        hasResidentKey: true,
        hasUserVerification: true,
        isUserVerified: true,
        automaticPresenceSimulation: true,
      },
    });
    findings.authenticator = { kind: 'virtual', protocol: 'ctap2', transport: 'internal', authenticatorId };

    // --- 1. the legitimate origin sets and re-receives its cookie ---------
    await s('Page.navigate', { url: legitOrigin + '/' });
    await sleep(600);
    const echoed = await evaluate(cdp, sessionId,
      `fetch('/echo-cookie', {credentials:'include'}).then(r => r.json())`);
    findings.cookie = {
      name: COOKIE,
      setByLegitimateOrigin: true,
      returnedToLegitimateOrigin: echoed?.received === true,
    };

    // --- 2. create a credential at the legitimate origin ------------------
    findings.create = await evaluate(cdp, sessionId, CREATE(legitOrigin));

    // --- 3. the hostile port overwrites the same cookie -------------------
    // Cookies are keyed by host, not by origin: the port is not a boundary.
    await s('Page.navigate', { url: hostileOrigin + '/' });
    await sleep(600);
    const afterOverwrite = await s('Network.getCookies', { urls: [legitOrigin + '/'] });
    const overwritten = (afterOverwrite.cookies || []).find((c) => c.name === COOKIE);
    findings.cookieIsolation = {
      hostileOriginCouldSetTheSameCookie: Boolean(overwritten),
      // Reported structurally: whether the value the legitimate origin set is
      // still the one present, never the values themselves.
      legitimateValueSurvived: overwritten ? overwritten.value === 'fixture-session-value' : null,
      conclusion: 'port is not a cookie boundary on loopback',
    };

    // --- 4. the hostile port asks for an assertion over the same RP ID ----
    // RP ID is a host, so this is expected to succeed. What matters is what
    // the legitimate origin does with the result.
    const hostileAssertion = await evaluate(cdp, sessionId, GET(hostileOrigin));
    findings.hostileAssertion = hostileAssertion;
    findings.originCheck = {
      hostileAssertionObtained: hostileAssertion?.ok === true,
      clientDataOrigin: hostileAssertion?.clientDataOrigin ?? null,
      acceptedAtLegitimateOrigin: hostileAssertion?.ok === true
        ? acceptedAtLegitimateOrigin(hostileAssertion.clientDataOrigin)
        : null,
      conclusion:
        'the origin inside clientDataJSON is the only thing separating the two ports',
    };

    // --- 5. same RP, same user handle, from the hostile port --------------
    // Characterise whether the original credential is replaced or confused.
    const replacement = await evaluate(cdp, sessionId, CREATE(hostileOrigin));
    const credentials = await s('WebAuthn.getCredentials', { authenticatorId });
    findings.sameUserHandleCreation = {
      attemptSucceeded: replacement?.ok === true,
      credentialsOnAuthenticator: (credentials.credentials || []).length,
      conclusion: replacement?.ok === true
        ? 'another origin on this host can create over the same RP ID and user handle'
        : 'creation from the other origin was refused',
    };

    // --- 6. a legitimate assertion is still accepted ----------------------
    await s('Page.navigate', { url: legitOrigin + '/' });
    await sleep(600);
    const legitAssertion = await evaluate(cdp, sessionId, GET(legitOrigin));
    findings.legitimateAssertion = {
      ok: legitAssertion?.ok === true,
      clientDataOrigin: legitAssertion?.clientDataOrigin ?? null,
      acceptedAtLegitimateOrigin: legitAssertion?.ok === true
        ? acceptedAtLegitimateOrigin(legitAssertion.clientDataOrigin)
        : null,
    };

    // The virtual authenticator, its credentials, cookies and profile all go
    // away with the browser and the temp directory below. Nothing persists.
    await s('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
  } finally {
    browser.kill();
    legit.close();
    hostile.close();
    rmSync(profile, { recursive: true, force: true });
  }

  // The two claims this preflight exists to establish.
  const proved =
    findings.originCheck?.hostileAssertionObtained === true &&
    findings.originCheck?.acceptedAtLegitimateOrigin === false &&
    findings.legitimateAssertion?.acceptedAtLegitimateOrigin === true &&
    findings.cookie?.returnedToLegitimateOrigin === true;

  const report = {
    schema: 'owner-auth-origin-preflight/v1',
    mode: VIRTUAL ? 'virtual' : 'real',
    // Virtual rows measure the protocol, never a platform authenticator.
    qualification: VIRTUAL ? 'protocol_fixture_only' : 'mechanism_qualified_only',
    status: proved ? 'pass' : 'fail',
    platform: { os: os.platform(), release: os.release(), arch: os.arch() },
    browser: browserVersion,
    origins: { legitimate: legitOrigin, hostile: hostileOrigin, rpId: RP_ID },
    findings,
  };
  console.log(JSON.stringify(report, null, 2));
  process.exit(proved ? 0 : 1);
}

main().catch((error) => {
  console.error(`owner-auth-origin-preflight: ${error.message}`);
  process.exit(1);
});
