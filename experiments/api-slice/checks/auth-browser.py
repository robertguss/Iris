#!/usr/bin/env python3
"""Credential-free browser flow. Requires fresh auth API, web, and OIDC fixture."""
import argparse
import json
from pathlib import Path
import subprocess
import re
import time
from urllib.request import urlopen

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("url")
parser.add_argument("--artifacts", type=Path, required=True)
parser.add_argument("--mailpit", help="Verify delivery using this local Mailpit API instead of the response token")
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

def mail_json(path):
    with urlopen(args.mailpit.rstrip("/") + path, timeout=5) as response:
        return json.load(response)

def delivered_link(previous):
    for _ in range(100):
        for mail in mail_json("/api/v1/messages")["messages"]:
            if mail["ID"] in previous:
                continue
            message = mail_json("/api/v1/message/" + mail["ID"])
            if message["To"][0]["Address"] != "bob@example.test":
                continue
            match = re.search(r"https?://[^\s]+/#invitation=[a-f0-9]{64}", message["Text"])
            if match and match[0].startswith(args.url.rstrip("/") + "/#"):
                return match[0]
        time.sleep(0.2)
    raise AssertionError("No matching invitation email arrived")

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
    previous = {m["ID"] for m in mail_json("/api/v1/messages")["messages"]} if args.mailpit else set()
    browser("select", "#operation", "issue")
    button("Issue invitation")
    browser("wait", "--text", "201 · Invitation issued")
    check("JSON.parse(document.querySelector('pre').textContent).recipient_id === '29'")
    capture("queued")
    if args.mailpit:
        link = delivered_link(previous)
        browser("open", link)
        browser("wait", "--text", "Invitation loaded from email")
        button("Accept invitation")
        browser("wait", "--text", "404 · Invitation not found")
    else:
        token = json.loads(browser("eval", "JSON.parse(document.querySelector('pre').textContent).token"))
    button("Sign out")
    browser("wait", "--text", "Not signed in")
    check("document.querySelector('fieldset').disabled")
    if args.mailpit:
        browser("open", link)
        browser("wait", "--text", "Invitation loaded from email")
        capture("email-signed-out")
    login("Bob")
    if args.mailpit:
        browser("open", link)
        browser("wait", "--text", "Invitation loaded from email")
        check("location.hash === '' && localStorage.length === 0 && sessionStorage.length === 0")
    else:
        browser("fill", "#token", token)
    button("Accept invitation")
    browser("wait", "--text", "200 · Invitation accepted")
    check("JSON.parse(document.querySelector('pre').textContent).project_id === '41' && JSON.parse(document.querySelector('pre').textContent).user_id === '29'")
    capture("accepted")
    button("Accept invitation")
    browser("wait", "--text", "409 · Already accepted")
    capture("replayed")
    browser("select", "#operation", "role")
    button("Change member role")
    browser("wait", "--text", "403 · Owner permission required")
    button("Sign out")
    browser("wait", "--text", "Not signed in")
    login("Alice")
    browser("select", "#operation", "role")
    button("Change member role")
    browser("wait", "--text", "200 · Member role changed")
    check("JSON.parse(document.querySelector('pre').textContent).role === 'viewer'")
    capture("member-role")
    browser("select", "#operation", "remove")
    check("document.querySelector('button[type=submit]').disabled")
    browser("check", "input[type=checkbox]")
    button("Remove member")
    browser("wait", "--text", "200 · Member removed")
    check("JSON.parse(document.querySelector('pre').textContent).role === null")
    browser("fill", "#member", "11")
    browser("check", "input[type=checkbox]")
    button("Remove member")
    browser("wait", "--text", "409 · Last owner must remain")
    capture("member-last-owner")
    browser("set", "viewport", "390", "844", "2")
    check("document.documentElement.scrollWidth <= innerWidth")
    capture("narrow")
    button("Sign out")
    browser("wait", "--text", "Not signed in")
    browser("reload")
    browser("wait", "--text", "Not signed in")
    check("document.querySelector('fieldset').disabled && localStorage.length === 0 && sessionStorage.length === 0")
    print("PASS: OIDC redirects, Alice issuance, Bob acceptance, replay rejection, member role/removal/last-owner protection, HttpOnly visibility, logout, and narrow layout" + ("; Mailpit email link, wrong-recipient rejection, and fragment scrubbing" if args.mailpit else ""))
finally:
    browser("close")
