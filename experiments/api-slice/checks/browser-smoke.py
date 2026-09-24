#!/usr/bin/env python3
"""Requires agent-browser and a fresh demo server. Exercises real HTTP except for
the explicitly injected network-failure case. Saves screenshots for inspection.
"""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("url")
parser.add_argument("--artifacts", type=Path, required=True)
args = parser.parse_args()
args.artifacts.mkdir(parents=True, exist_ok=True)
base = ["agent-browser", "--session", "iris-smoke", "--init-script",
        str(Path(__file__).with_name("browser-init.js").resolve())]


def browser(*command):
    result = subprocess.run([*base, *command], capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)
    return result.stdout


def check(expression):
    browser("eval", f"(() => {{ if (!({expression})) throw new Error('Browser assertion failed: ' + {expression!r}); return 'PASS'; }})()")


def capture(name):
    browser("eval", "new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
    browser("screenshot", str((args.artifacts / f"{name}.png").resolve()), "--full")


def submit(text):
    browser("click", "button[type=submit]")
    browser("wait", "--text", text)


try:
    browser("open", args.url)
    browser("set", "viewport", "1280", "1000", "2")
    browser("wait", "--text", "Ready when you are")
    check("devicePixelRatio === 2")
    capture("iris-ready")
    browser("eval", "window.irisTest.hold = true")
    submit("Waiting for the API")
    check("document.querySelector('fieldset').disabled && document.querySelector('.response-panel').getAttribute('aria-busy') === 'true'")
    # A programmatic duplicate submit must also be guarded, not just the button.
    browser("eval", "document.querySelector('form').requestSubmit()")
    check("window.irisTest.requests === 1")
    capture("iris-loading")
    browser("eval", "window.irisTest.hold = false; window.irisTest.release()")
    browser("wait", "--text", "200 · Invitation accepted")
    check("JSON.parse(document.querySelector('pre').textContent).user_id === '11' && JSON.parse(document.querySelector('pre').textContent).project_id === '7'")
    capture("iris-success")
    submit("409 · Already accepted")
    capture("iris-conflict")
    browser("select", "#identity", "29")
    submit("404 · Invitation not found")
    check("JSON.parse(document.querySelector('pre').textContent).code === 'not_found'")
    browser("select", "#identity", "")
    submit("401 · Identity required")
    capture("iris-unauthorized")
    browser("find", "role", "button", "click", "--name", "Expired", "--exact")
    submit("409 · Invitation expired")
    capture("iris-expired")
    # Use keyboard events: this agent-browser version's empty fill does not
    # update React's controlled input state.
    browser("click", "#token")
    browser("press", "Control+a")
    browser("press", "Backspace")
    submit("400 · Invalid request")
    browser("find", "role", "button", "click", "--name", "Bob’s invitation", "--exact")
    submit("200 · Invitation accepted")
    check("JSON.parse(document.querySelector('pre').textContent).project_id === '19'")
    browser("eval", "window.irisTest.fail = true")
    submit("Couldn’t read an API response")
    check("!document.querySelector('fieldset').disabled")
    capture("iris-network-failure")
    browser("eval", "window.irisTest.fail = false")
    browser("find", "role", "button", "click", "--name", "Expired", "--exact")
    submit("409 · Invitation expired")
    browser("set", "viewport", "390", "844", "2")
    check("document.documentElement.scrollWidth === innerWidth")
    check("getComputedStyle(document.querySelector('.lab')).gridTemplateColumns.split(' ').length === 1")
    capture("iris-narrow")
    browser("set", "viewport", "1280", "1000", "2")
    browser("select", "#operation", "issue")
    browser("select", "#identity", "29")
    submit("403 · Owner permission required")
    capture("iris-issue-forbidden")
    browser("select", "#identity", "11")
    browser("eval", "window.irisTest.hold = true; window.irisTest.requests = 0")
    submit("Waiting for the API")
    browser("eval", "document.querySelector('form').requestSubmit()")
    check("window.irisTest.requests === 1 && document.querySelector('fieldset').disabled")
    capture("iris-issue-loading")
    browser("eval", "window.irisTest.hold = false; window.irisTest.release()")
    browser("wait", "--text", "201 · Invitation issued")
    check("JSON.parse(document.querySelector('pre').textContent).recipient_id === '29'")
    capture("iris-issued")
    browser("set", "viewport", "390", "844", "2")
    check("document.documentElement.scrollWidth === innerWidth")
    capture("iris-issued-narrow")
    browser("find", "role", "button", "click", "--name", "Switch to recipient and load token →", "--exact")
    check("document.querySelector('#identity').value === '29' && document.querySelector('#token').value.length === 64")
    submit("200 · Invitation accepted")
    check("JSON.parse(document.querySelector('pre').textContent).project_id === '41' && JSON.parse(document.querySelector('pre').textContent).user_id === '29'")
    browser("set", "viewport", "1280", "1000", "2")
    capture("iris-issued-accepted")
    browser("select", "#operation", "issue")
    browser("select", "#identity", "11")
    submit("409 · Already a member")
    capture("iris-issue-member")
    browser("select", "#identity", "29")
    browser("fill", "#project", "43")
    browser("select", "#recipient", "11")
    submit("201 · Invitation issued")
    submit("409 · Invitation pending")
    capture("iris-issue-pending")
    browser("eval", "window.irisTest.fail = true")
    submit("Couldn’t read an API response")
    capture("iris-issue-network")
    print("PASS: acceptance scenarios, owner rejection, issuance/loading guard, identity handoff, acceptance, membership conflict, pending conflict, network failure, narrow layout")
finally:
    browser("close")
