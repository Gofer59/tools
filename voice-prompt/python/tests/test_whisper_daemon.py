"""
pytest suite for whisper_daemon.py.

All tests mock WhisperModel — no real model downloads required.
To run against real models (slow): VOICE_PROMPT_REAL_MODEL_TESTS=1 pytest python/tests/
"""
import io
import json
import os
import subprocess
import sys
import threading
import wave
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

DAEMON = Path(__file__).parent.parent / "whisper_daemon.py"


# ── Helpers ───────────────────────────────────────────────────────────────────

def make_wav(path: Path, duration_s: float = 0.5, sample_rate: int = 16000) -> None:
    """Write a minimal silent WAV file for testing."""
    n_samples = int(duration_s * sample_rate)
    with wave.open(str(path), "w") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        wf.writeframes(b"\x00\x00" * n_samples)


def make_segment(text: str, start: float = 0.0, end: float = 1.0):
    seg = MagicMock()
    seg.text = text
    seg.start = start
    seg.end = end
    return seg


def make_info(language: str = "en", language_probability: float = 0.99):
    info = MagicMock()
    info.language = language
    info.language_probability = language_probability
    return info


def spawn_daemon(model: str = "tiny", compute: str = "int8", device: str = "cpu",
                  model_dir: str = "/tmp/models") -> subprocess.Popen:
    """Start daemon as a subprocess with a mocked WhisperModel injected via env."""
    env = {**os.environ, "_VOICE_PROMPT_MOCK_DAEMON": "1"}
    return subprocess.Popen(
        [sys.executable, str(DAEMON), model, compute, model_dir, device],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )


def send_cmd(proc: subprocess.Popen, cmd: dict) -> dict:
    assert proc.stdin and proc.stdout
    line = json.dumps(cmd) + "\n"
    proc.stdin.write(line)
    proc.stdin.flush()
    response = proc.stdout.readline()
    return json.loads(response)


# ── Unit tests (mocked WhisperModel inline) ────────────────────────────────────

class TestDaemonLogicMocked:
    """Test daemon stdin/stdout protocol by importing the module directly with mocks."""

    def _run_daemon_in_thread(self, requests: list[dict], mock_model) -> list[dict]:
        """Run the daemon's main loop against a list of requests; return responses."""
        import importlib.util
        import sys as _sys

        input_lines = "\n".join(json.dumps(r) for r in requests) + "\n"
        fake_stdin = io.StringIO(input_lines)
        collected: list[str] = []

        def fake_print(*args, **kwargs):
            file = kwargs.get("file")
            if file is _sys.stderr:
                return  # suppress debug logs
            collected.append(str(args[0]) if args else "")

        # Re-import the daemon module with patched WhisperModel and I/O.
        spec = importlib.util.spec_from_file_location("whisper_daemon_test", DAEMON)
        assert spec is not None and spec.loader is not None
        mod = importlib.util.module_from_spec(spec)

        with patch("faster_whisper.WhisperModel", return_value=mock_model), \
             patch.object(_sys, "argv", ["daemon", "tiny", "int8", "/tmp/models", "cpu"]), \
             patch.object(_sys, "stdin", fake_stdin), \
             patch("builtins.print", side_effect=fake_print):
            spec.loader.exec_module(mod)  # type: ignore[union-attr]
            mod.main()

        responses: list[dict] = []
        for line in collected:
            line = line.strip()
            if line:
                try:
                    responses.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
        return responses

    def test_transcribe_basic(self, tmp_path):
        wav = tmp_path / "test.wav"
        make_wav(wav)

        mock_model = MagicMock()
        mock_model.transcribe.return_value = (
            iter([make_segment("hello world")]),
            make_info("en", 0.98),
        )

        responses = self._run_daemon_in_thread(
            [
                {"cmd": "transcribe", "wav": str(wav), "language": "en", "vad": False},
                {"cmd": "quit"},
            ],
            mock_model,
        )

        assert responses[0]["status"] == "ready"
        assert responses[1]["status"] == "ok"
        assert responses[1]["text"] == "hello world"
        assert responses[1]["language"] == "en"

    def test_transcribe_language_auto(self, tmp_path):
        wav = tmp_path / "test.wav"
        make_wav(wav)

        mock_model = MagicMock()
        mock_model.transcribe.return_value = (
            iter([make_segment("bonjour le monde")]),
            make_info("fr", 0.97),
        )

        responses = self._run_daemon_in_thread(
            [
                {"cmd": "transcribe", "wav": str(wav), "language": "auto", "vad": False},
                {"cmd": "quit"},
            ],
            mock_model,
        )

        # language="auto" must be forwarded as None to model.transcribe
        call_kwargs = mock_model.transcribe.call_args
        assert call_kwargs.kwargs.get("language") is None

        assert responses[1]["language"] == "fr"
        assert "bonjour" in responses[1]["text"]

    def test_unknown_command_returns_error(self, tmp_path):
        wav = tmp_path / "test.wav"
        make_wav(wav)

        mock_model = MagicMock()
        mock_model.transcribe.return_value = (iter([]), make_info())

        responses = self._run_daemon_in_thread(
            [
                {"cmd": "badcmd"},
                {"cmd": "quit"},
            ],
            mock_model,
        )

        error_responses = [r for r in responses if r.get("status") == "error"]
        assert len(error_responses) >= 1

    def test_duration_ms_populated(self, tmp_path):
        wav = tmp_path / "test.wav"
        make_wav(wav)

        mock_model = MagicMock()
        mock_model.transcribe.return_value = (
            iter([make_segment("test")]),
            make_info(),
        )

        responses = self._run_daemon_in_thread(
            [
                {"cmd": "transcribe", "wav": str(wav), "language": "en", "vad": False},
                {"cmd": "quit"},
            ],
            mock_model,
        )
        assert isinstance(responses[1].get("duration_ms"), int)

    def test_device_arg_passed_to_model(self) -> None:
        constructor_calls: list[dict] = []

        def mock_constructor(**kwargs):
            constructor_calls.append(kwargs)
            m = MagicMock()
            m.transcribe.return_value = (iter([make_segment("ok")]), make_info())
            return m

        import importlib.util
        import sys as _sys

        spec = importlib.util.spec_from_file_location("whisper_daemon_dev", DAEMON)
        assert spec is not None and spec.loader is not None
        mod = importlib.util.module_from_spec(spec)

        fake_stdin = io.StringIO(json.dumps({"cmd": "quit"}) + "\n")

        with patch("faster_whisper.WhisperModel", side_effect=mock_constructor), \
             patch.object(_sys, "argv", ["d", "tiny", "int8", "/tmp", "cuda"]), \
             patch.object(_sys, "stdin", fake_stdin), \
             patch("builtins.print"):
            spec.loader.exec_module(mod)  # type: ignore[union-attr]
            mod.main()

        assert len(constructor_calls) == 1
        assert constructor_calls[0]["device"] == "cuda"


class TestPreviewLenUnicode:
    """Verify that Python len() returns Unicode scalar count (not byte count).

    This matches Rust's str::chars().count() used for backspace delete.
    """

    def test_ascii(self):
        assert len("hello") == 5

    def test_accented(self):
        # "à bientôt" is 9 Unicode chars but 11 bytes in UTF-8
        text = "à bientôt"
        assert len(text) == 9
        assert len(text.encode("utf-8")) == 11

    def test_cjk(self):
        text = "你好"
        assert len(text) == 2          # 2 Unicode scalars
        assert len(text.encode("utf-8")) == 6  # 6 bytes


# ── Two-instance concurrency test (subprocess-based) ──────────────────────────

@pytest.mark.skipif(
    not (Path(sys.executable)).exists(),
    reason="Python subprocess required",
)
class TestTwoInstances:
    """Verify two independent daemon processes transcribing the same WAV don't interfere."""

    def test_two_instances_same_wav(self, tmp_path):
        """Two daemons receive the same WAV path; both return valid JSON."""
        wav = tmp_path / "shared.wav"
        make_wav(wav, duration_s=0.5)

        # We can't inject mocks into subprocesses easily, so skip if faster_whisper
        # is not importable (CI without model deps).
        try:
            import faster_whisper  # noqa: F401
        except ImportError:
            pytest.skip("faster_whisper not installed — subprocess test skipped")

        # Spawn two daemons pointing at the same model dir.
        # They will download tiny on first run if not cached.
        model_dir = tmp_path / "models"
        model_dir.mkdir()

        proc_a = subprocess.Popen(
            [sys.executable, str(DAEMON), "tiny", "int8", str(model_dir), "cpu"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
        )
        proc_b = subprocess.Popen(
            [sys.executable, str(DAEMON), "tiny", "int8", str(model_dir), "cpu"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
        )
        assert proc_a.stdin and proc_a.stdout
        assert proc_b.stdin and proc_b.stdout

        try:
            # Wait for "ready" from both.
            ready_a = json.loads(proc_a.stdout.readline())
            ready_b = json.loads(proc_b.stdout.readline())
            assert ready_a["status"] == "ready"
            assert ready_b["status"] == "ready"

            # Send transcribe to both simultaneously.
            results: dict = {}

            def run(proc: subprocess.Popen, key: str) -> None:
                assert proc.stdin and proc.stdout
                req = json.dumps({"cmd": "transcribe", "wav": str(wav), "language": "en", "vad": False}) + "\n"
                proc.stdin.write(req)
                proc.stdin.flush()
                results[key] = json.loads(proc.stdout.readline())

            t1 = threading.Thread(target=run, args=(proc_a, "a"))
            t2 = threading.Thread(target=run, args=(proc_b, "b"))
            t1.start()
            t2.start()
            t1.join(timeout=60)
            t2.join(timeout=60)

            assert results.get("a", {}).get("status") == "ok", results.get("a")
            assert results.get("b", {}).get("status") == "ok", results.get("b")
        finally:
            for p in (proc_a, proc_b):
                try:
                    if p.stdin:
                        p.stdin.write(json.dumps({"cmd": "quit"}) + "\n")
                        p.stdin.flush()
                except Exception:
                    pass
                p.terminate()


# ── Real-model integration tests (opt-in) ─────────────────────────────────────

REAL = os.getenv("VOICE_PROMPT_REAL_MODEL_TESTS") == "1"


class TestStreamingProtocol:
    """Verify stream_start / stream_chunk / stream_stop emit the expected NDJSON shapes."""

    def test_stream_round_trip(self, tmp_path):
        from unittest.mock import MagicMock, patch
        import importlib.util
        import io as _io
        import sys as _sys

        wav = tmp_path / "test.wav"
        make_wav(wav)

        mock_model = MagicMock()
        mock_model.transcribe.return_value = (
            iter([make_segment("partial text")]),
            make_info("en", 0.95),
        )

        requests = [
            {"cmd": "stream_start", "language": "en", "vad": False, "sample_rate": 16000},
            {"cmd": "stream_chunk", "wav": str(wav), "seq": 1},
            {"cmd": "stream_chunk", "wav": str(wav), "seq": 2},
            {"cmd": "stream_stop"},
            {"cmd": "quit"},
        ]
        input_lines = "\n".join(json.dumps(r) for r in requests) + "\n"
        fake_stdin = _io.StringIO(input_lines)
        collected = []

        def fake_print(*args, **kwargs):
            file = kwargs.get("file")
            if file is _sys.stderr:
                return
            collected.append(str(args[0]) if args else "")

        spec = importlib.util.spec_from_file_location("whisper_daemon_stream", DAEMON)
        assert spec is not None and spec.loader is not None
        mod = importlib.util.module_from_spec(spec)
        with patch("faster_whisper.WhisperModel", return_value=mock_model), \
             patch.object(_sys, "argv", ["d", "tiny", "int8", "/tmp", "cpu"]), \
             patch.object(_sys, "stdin", fake_stdin), \
             patch("builtins.print", side_effect=fake_print):
            spec.loader.exec_module(mod)
            mod.main()

        responses = [json.loads(line) for line in collected if line.strip()]
        kinds = [r.get("status") or r.get("event") for r in responses]
        assert kinds == ["ready", "streaming", "partial", "partial", "final", "idle", "quitting"]
        partials = [r for r in responses if r.get("event") == "partial"]
        assert partials[0]["seq"] == 1
        assert partials[0]["text"] == "partial text"
        assert partials[1]["seq"] == 2


@pytest.mark.skipif(not REAL, reason="Set VOICE_PROMPT_REAL_MODEL_TESTS=1 to run")
class TestRealModel:
    def test_real_transcribe_english(self, tmp_path):
        wav = tmp_path / "en.wav"
        make_wav(wav, duration_s=2.0)
        model_dir = Path.home() / ".local/share/voice-prompt/models"
        proc = subprocess.Popen(
            [sys.executable, str(DAEMON), "tiny", "int8", str(model_dir), "cpu"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
        )
        assert proc.stdin and proc.stdout
        try:
            ready = json.loads(proc.stdout.readline())
            assert ready["status"] == "ready"
            req = json.dumps({"cmd": "transcribe", "wav": str(wav), "language": "en", "vad": False}) + "\n"
            proc.stdin.write(req)
            proc.stdin.flush()
            resp = json.loads(proc.stdout.readline())
            assert resp["status"] == "ok"
            assert isinstance(resp["text"], str)
        finally:
            proc.stdin.write('{"cmd":"quit"}\n')
            proc.stdin.flush()
            proc.terminate()
