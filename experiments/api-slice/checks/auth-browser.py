#!/usr/bin/env python3
"""Credential-free browser flow. Requires fresh auth API, web, and OIDC fixture."""
import argparse
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("url")
parser.add_argument("--artifacts", type=Path, required=True)
args = parser.parse_args()
args.artifacts.mkdir(parents=True, exist_ok=True)

def browser(*command):
    result = subprocess.run(["agent-browser", "--session", "iris-auth-check", *command],
                            capture_output=True, text=True, timeout=40)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)
    return result.stdout

def check(expression):
    browser("eval", f"(() => {{ if (!({expression})) throw new Error('Browser assertion failed'); return true; }})()")

def capture(name):
    browser("eval", "new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
    browser("screenshot", str((args.artifacts / f"auth-{name}.png").resolve()), "--full")

def button(name):
    browser("find", "role", "button", "click", "--name", name, "--exact")

def login(identity):
    button("Sign in with test provider")
    browser("wait", "--text", "Choose a fixed test identity")
    button(f"Continue as {identity}")
    browser("wait", "--text", f"Signed in as {identity}")

try:
    browser("open", args.url)
    browser("set", "viewport", "1280", "1000", "2")
    browser("wait", "--text", "Not signed in")
    check("devicePixelRatio === 2 && document.querySelector('fieldset').disabled && !document.querySelector('#identity')")
    capture("anonymous")
    browser("network", "route", "**/api/auth/session", "--abort")
    browser("reload")
    browser("wait", "--text", "Session unavailable")
    check("document.querySelector('fieldset').disabled && document.querySelector('.auth-panel button').disabled")
    capture("session-error")
    browser("network", "unroute", "**/api/auth/session")
    button("Refresh session")
    browser("wait", "--fn", "!document.querySelector('.auth-panel button').disabled")
    login("Alice")
    check("!document.querySelector('fieldset').disabled && !document.cookie.includes('iris-session')")
    browser("select", "#operation", "issue")
    button("Issue invitation")
    browser("wait", "--text", "201 · Invitation issued")
    check("JSON.parse(document.querySelector('pre').textContent).recipient_id === '29'")
    # Keep this disposable invitation in the test runner, never browser storage.
    token = json.loads(browser("eval", "JSON.parse(document.querySelector('pre').textContent).token"))
    button("Sign out")
    browser("wait", "--text", "Not signed in")
    check("document.querySelector('fieldset').disabled")
    login("Bob")
    browser("fill", "#token", token)
    button("Accept invitation")
    browser("wait", "--text", "200 · Invitation accepted")
    check("JSON.parse(document.querySelector('pre').textContent).project_id === '41' && JSON.parse(document.querySelector('pre').textContent).user_id === '29'")
    capture("accepted")
    button("Accept invitation")
    browser("wait", "--text", "409 · Already accepted")
    capture("replayed")
    browser("set", "viewport", "390", "844", "2")
    check("document.documentElement.scrollWidth <= innerWidth")
    capture("narrow")
    button("Sign out")
    browser("wait", "--text", "Not signed in")
    browser("reload")
    browser("wait", "--text", "Not signed in")
    check("document.querySelector('fieldset').disabled && localStorage.length === 0 && sessionStorage.length === 0")
    print("PASS: OIDC redirects, Alice issuance, Bob acceptance, replay rejection, HttpOnly visibility, logout, and narrow layout")
finally:
    browser("close")
