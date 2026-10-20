#!/usr/bin/env python3
"""Local xAI chat/TTS bridge for muku.

The xAI API key stays in macOS Keychain. The browser talks only to this
localhost server; completed replies are sent to muku's authenticated webhook.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import secrets
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

MAX_REQUEST_BYTES = 64 * 1024
MAX_HISTORY_MESSAGES = 16
MAX_REPLY_CHARS = 180

SYSTEM_PROMPT = """\
あなたは小さなゲーム機に住む「muku」です。
日本語で、親しみやすく、落ち着いた短い返答をしてください。
原則として1〜3文、180文字以内に収めてください。
ユーザーを急かさず、必要なら一度に一つだけ質問してください。
不確かな最新情報は、Web検索が有効な場合にだけ確認してください。
自分の模型名やAPIの内部事情には触れないでください。
"""


def load_xai_key(service: str, account: str) -> str:
    result = subprocess.run(
        [
            "/usr/bin/security",
            "find-generic-password",
            "-a",
            account,
            "-s",
            service,
            "-w",
        ],
        check=False,
        capture_output=True,
        text=True,
        timeout=10,
    )
    key = result.stdout.strip()
    if result.returncode != 0 or len(key) < 20:
        raise RuntimeError(
            f"xAI API key was not found in Keychain service {service!r}"
        )
    return key


def load_muku_token(path: Path) -> str:
    data = json.loads(path.read_text(encoding="utf-8"))
    token = data.get("token")
    if not isinstance(token, str) or len(token) < 16:
        raise RuntimeError("muku webhook token is missing or too short")
    return token


def request_bytes(
    url: str, *, api_key: str | None = None, payload: dict[str, Any] | None = None
) -> tuple[int, str, bytes]:
    headers = {"Accept": "application/json, audio/wav"}
    data = None
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    if payload is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    request = Request(url, data=data, headers=headers, method="POST" if data else "GET")
    try:
        with urlopen(request, timeout=120) as response:
            return response.status, response.headers.get_content_type(), response.read()
    except HTTPError as error:
        body = error.read()
        try:
            detail = json.loads(body).get("error", {}).get("message", "")
        except Exception:
            detail = body[:400].decode("utf-8", "replace")
        raise RuntimeError(f"HTTP {error.code}: {detail or error.reason}") from error
    except URLError as error:
        raise RuntimeError(f"network error: {error.reason}") from error


def output_text(response: dict[str, Any]) -> str:
    parts: list[str] = []
    for item in response.get("output", []):
        if item.get("type") != "message":
            continue
        for content in item.get("content", []):
            if content.get("type") == "output_text" and content.get("text"):
                parts.append(content["text"])
    text = "\n".join(parts).strip()
    if not text:
        raise RuntimeError("xAI returned no output text")
    return text


def normalize_wav(content_type: str, body: bytes) -> bytes:
    if body[:4] == b"RIFF" and body[8:12] == b"WAVE":
        return body
    if content_type == "application/json":
        payload = json.loads(body)
        encoded = payload.get("audio")
        if isinstance(encoded, str):
            wav = base64.b64decode(encoded)
            if wav[:4] == b"RIFF" and wav[8:12] == b"WAVE":
                return wav
    raise RuntimeError("xAI TTS did not return a WAV file")


class Bridge:
    def __init__(
        self,
        *,
        api_key: str,
        muku_url: str,
        muku_token: str,
        model: str,
        voice: str,
    ):
        self.api_key = api_key
        self.muku_url = muku_url
        self.muku_token = muku_token
        self.model = model
        self.voice = voice
        self.history: list[dict[str, str]] = []
        self.lock = threading.Lock()

    def _grok(self, prompt: str, web_search: bool) -> tuple[str, str]:
        payload: dict[str, Any] = {
            "model": self.model,
            "instructions": SYSTEM_PROMPT,
            "input": [*self.history, {"role": "user", "content": prompt}],
            "store": False,
            "max_output_tokens": 300,
        }
        if web_search:
            payload["tools"] = [{"type": "web_search"}]
        _, _, body = request_bytes(
            "https://api.x.ai/v1/responses", api_key=self.api_key, payload=payload
        )
        response = json.loads(body)
        if response.get("error"):
            raise RuntimeError(f"xAI response error: {response['error']}")
        return output_text(response)[:MAX_REPLY_CHARS], str(response.get("id", ""))

    def _tts(self, text: str) -> bytes:
        payload = {
            "text": text,
            "voice_id": self.voice,
            "language": "ja",
            "output_format": {"codec": "wav", "sample_rate": 22050},
            "text_normalization": True,
        }
        _, content_type, body = request_bytes(
            "https://api.x.ai/v1/tts", api_key=self.api_key, payload=payload
        )
        return normalize_wav(content_type, body)

    def _send_to_muku(self, text: str, wav: bytes) -> None:
        payload = {
            "title": "muku",
            "body": text,
            "source": "xai",
            "kind": "chat",
            "audio_wav_base64": base64.b64encode(wav).decode("ascii"),
        }
        data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        request = Request(
            self.muku_url,
            data=data,
            method="POST",
            headers={
                "Authorization": f"Bearer {self.muku_token}",
                "Content-Type": "application/json",
            },
        )
        try:
            with urlopen(request, timeout=20) as response:
                if response.status != 202:
                    raise RuntimeError(f"muku webhook returned {response.status}")
        except HTTPError as error:
            raise RuntimeError(f"muku webhook returned HTTP {error.code}") from error
        except URLError as error:
            raise RuntimeError(f"muku is offline: {error.reason}") from error

    def chat(self, prompt: str, web_search: bool) -> dict[str, Any]:
        with self.lock:
            reply, response_id = self._grok(prompt, web_search)
            wav = self._tts(reply)
            self._send_to_muku(reply, wav)
            self.history.extend(
                [
                    {"role": "user", "content": prompt},
                    {"role": "assistant", "content": reply},
                ]
            )
            self.history = self.history[-MAX_HISTORY_MESSAGES:]
            return {
                "reply": reply,
                "response_id": response_id,
                "audio_bytes": len(wav),
                "delivered": True,
            }


PAGE = r"""<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>muku × Grok</title>
<style>
:root{font-family:ui-rounded,-apple-system,BlinkMacSystemFont,"Hiragino Sans",sans-serif;
color:#25251f;background:#f4f1e8}
body{max-width:760px;margin:0 auto;padding:24px}
h1{font-size:1.35rem;margin:0 0 4px}p{color:#6f746a}
#log{display:flex;flex-direction:column;gap:10px;margin:24px 0}
.msg{padding:12px 15px;border-radius:16px;max-width:85%;white-space:pre-wrap}
.user{align-self:flex-end;background:#d9ead8}.muku{align-self:flex-start;background:#fff}
form{display:grid;grid-template-columns:1fr auto;gap:10px}
textarea{min-height:72px;padding:12px;border:1px solid #c9c8bb;border-radius:14px;
font:inherit;resize:vertical;background:#fffdf8}
button{border:0;border-radius:14px;padding:0 20px;background:#557b5c;color:white;font:inherit}
.options{margin-top:8px;color:#697268}.error{color:#a4493d}
</style>
</head>
<body>
<h1>muku × Grok</h1>
<p>返答はR36MAX 2へ表示・音声再生されます。</p>
<div id="log"></div>
<form id="chat">
<textarea id="message" placeholder="mukuになに話しかける？" required></textarea>
<button id="send">送信</button>
</form>
<label class="options"><input id="search" type="checkbox"> 最新情報をWeb検索する</label>
<script>
const token = "__BRIDGE_TOKEN__";
const log = document.querySelector("#log");
const form = document.querySelector("#chat");
const message = document.querySelector("#message");
const send = document.querySelector("#send");
function add(cls, text) {
  const node = document.createElement("div");
  node.className = `msg ${cls}`;
  node.textContent = text;
  log.append(node);
  node.scrollIntoView({behavior:"smooth", block:"end"});
}
form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const text = message.value.trim();
  if (!text) return;
  add("user", text);
  message.value = "";
  send.disabled = true;
  send.textContent = "考え中…";
  try {
    const response = await fetch("/api/chat", {
      method:"POST",
      headers:{"Content-Type":"application/json", "X-Muku-Bridge":token},
      body:JSON.stringify({message:text, web_search:document.querySelector("#search").checked})
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "request failed");
    add("muku", data.reply);
  } catch (error) {
    add("muku error", `送信できませんでした: ${error.message}`);
  } finally {
    send.disabled = false;
    send.textContent = "送信";
    message.focus();
  }
});
message.focus();
</script>
</body>
</html>
"""


def make_handler(bridge: Bridge, bridge_token: str) -> type[BaseHTTPRequestHandler]:
    page = PAGE.replace("__BRIDGE_TOKEN__", bridge_token).encode("utf-8")

    class Handler(BaseHTTPRequestHandler):
        server_version = "MukuXAIBridge/1"

        def log_message(self, fmt: str, *args: object) -> None:
            print(f"{self.address_string()} {fmt % args}")

        def send_body(self, status: int, content_type: str, body: bytes) -> None:
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            self.wfile.write(body)

        def json(self, status: int, value: dict[str, Any]) -> None:
            self.send_body(
                status,
                "application/json; charset=utf-8",
                json.dumps(value, ensure_ascii=False).encode("utf-8"),
            )

        def do_GET(self) -> None:
            if self.path == "/":
                self.send_body(200, "text/html; charset=utf-8", page)
            elif self.path == "/health":
                sensitive_names = [
                    name
                    for name in os.environ
                    if any(
                        marker in name.upper()
                        for marker in ("TOKEN", "SECRET", "API_KEY", "PASSWORD")
                    )
                ]
                self.json(
                    200,
                    {
                        "ok": True,
                        "history_messages": len(bridge.history),
                        "environment_isolated": not sensitive_names,
                    },
                )
            else:
                self.json(404, {"error": "not found"})

        def do_POST(self) -> None:
            if self.path != "/api/chat":
                self.json(404, {"error": "not found"})
                return
            if not secrets.compare_digest(
                self.headers.get("X-Muku-Bridge", ""), bridge_token
            ):
                self.json(403, {"error": "forbidden"})
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
            except ValueError:
                length = 0
            if not 0 < length <= MAX_REQUEST_BYTES:
                self.json(413, {"error": "request is too large"})
                return
            try:
                payload = json.loads(self.rfile.read(length))
                prompt = payload.get("message", "").strip()
                web_search = bool(payload.get("web_search", False))
                if not prompt or len(prompt) > 1000:
                    raise ValueError("message must contain 1 to 1000 characters")
                result = bridge.chat(prompt, web_search)
                self.json(200, result)
            except Exception as error:
                self.json(500, {"error": str(error)})

    return Handler


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", default=8790, type=int)
    parser.add_argument("--device-url", required=True)
    parser.add_argument("--muku-config", required=True, type=Path)
    parser.add_argument("--model", default="grok-4.7")
    parser.add_argument("--voice", default="eve")
    parser.add_argument("--keychain-service", default="tamago-muku-xai")
    parser.add_argument("--keychain-account", default="kan")
    args = parser.parse_args()

    api_key = load_xai_key(args.keychain_service, args.keychain_account)
    muku_token = load_muku_token(args.muku_config)
    bridge = Bridge(
        api_key=api_key,
        muku_url=args.device_url,
        muku_token=muku_token,
        model=args.model,
        voice=args.voice,
    )
    bridge_token = secrets.token_urlsafe(32)
    server = ThreadingHTTPServer(
        (args.host, args.port), make_handler(bridge, bridge_token)
    )
    server.daemon_threads = True
    print(f"muku xAI bridge: http://{args.host}:{args.port}")
    print(f"model={args.model} voice={args.voice}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
