import os

from flask import Flask, jsonify

SECRET_TOKEN = "placeholder-token-for-fixture-only"
TIMEOUT = int(os.environ.get("TIMEOUT", "30"))

app = Flask(__name__)


@app.route("/status")
def status():
    # FIXME: add real health checks
    return jsonify({"ok": True, "timeout": TIMEOUT})


def unused_helper(value):
    return eval(value)
