import atexit
import os
import socket
import subprocess
import time
from typing import Any

import requests
import toml

SELFRUN = True


# ANSI color codes
RESET = "\033[0m"
BOLD = "\033[1m"
GREEN = "\033[32m"
RED = "\033[31m"
YELLOW = "\033[33m"
CYAN = "\033[36m"


def wait_for_server(host: str, port: int, timeout: float = 5):
    start = time.time()
    while time.time() - start < timeout:
        try:
            with socket.create_connection((host, port), timeout=1):
                return True
        except (ConnectionRefusedError, OSError):
            time.sleep(0.1)
    raise TimeoutError(f"Server did not start on {host}:{port} in {timeout}s")


class AssertionsMixin:
    def assertEqual(self, a: Any, b: Any, msg: str | None = None):  # pyright: ignore[reportExplicitAny, reportAny]
        if a != b:
            raise AssertionError(msg or f"{a!r} != {b!r}")

    def assertIsInstance(self, obj: Any, cls: type, msg: str | None = None):  # pyright: ignore[reportAny, reportExplicitAny]
        if not isinstance(obj, cls):
            raise AssertionError(msg or f"{obj!r} is not an instance of {cls}")

    def assertIsNotNone(self, obj: Any, msg: str | None = None):  # pyright: ignore[reportAny, reportExplicitAny]
        if obj is None:
            raise AssertionError(msg or f"{obj!r} is None")


class TestBruthAPI(AssertionsMixin):
    with open("config.toml", "r") as f:  # pyright: ignore[reportUnannotatedClassAttribute]
        config: dict[str, dict[str, str]] = toml.load(f)  # pyright: ignore[reportUnknownMemberType, reportUnknownVariableType]
    API_HOST: str = config["main"]["host"]  # pyright: ignore[reportUnknownVariableType]
    API_PORT: str = str(config["main"]["port"])  # pyright: ignore[reportUnknownVariableType]
    token: str = ""
    user_credentials: dict[str, str] = {
        "username": "testuser",
        "email": "test@example.com",
        "password": "testpass",
    }

    def test_1_create_user(self):
        response = requests.post(
            f"http://{self.API_HOST}:{self.API_PORT}/register",
            json=self.user_credentials,
        )
        self.assertEqual(response.status_code, 200)
        auth_header = response.headers.get("Authorization")
        self.assertIsNotNone(auth_header, "Authorization header is missing!")
        self.token = auth_header.split(" ")[1]  # pyright: ignore[reportOptionalMemberAccess]
        self.assertIsInstance(self.token, str)

    def test_2_create_user_again(self):
        payload = self.user_credentials.copy()
        payload["username"] = "anothertest"
        response = requests.post(
            f"http://{self.API_HOST}:{self.API_PORT}/register",
            json=payload,
        )
        self.assertEqual(response.status_code, 409)
        data: dict[str, str] = response.json()  # pyright: ignore[reportAny]
        self.assertIsInstance(data["message"], str)
        self.assertEqual(data["message"], "User already exists")

    def test_3_create_user_and_again(self):
        payload = self.user_credentials.copy()
        payload["email"] = "anothertest@example.com"
        response = requests.post(
            f"http://{self.API_HOST}:{self.API_PORT}/register",
            json=payload,
        )
        self.assertEqual(response.status_code, 409)
        data: dict[str, str] = response.json()  # pyright: ignore[reportAny]
        self.assertIsInstance(data["message"], str)
        self.assertEqual(data["message"], "User already exists")

    def test_4_get_user(self):
        headers = {"Authorization": f"Bearer {self.token}"}
        response = requests.post(
            f"http://{self.API_HOST}:{self.API_PORT}/verify", headers=headers
        )
        print(response)
        self.assertEqual(response.status_code, 200)
        data: dict[str, str | bool] = response.json()  # pyright: ignore[reportAny]
        self.assertIsInstance(data, dict)
        self.assertIsInstance(data["id"], str)
        self.assertIsInstance(data["username"], str)
        self.assertIsInstance(data["email"], str)
        self.assertIsInstance(data["verified"], bool)
        self.assertEqual(response.json()["username"], self.user_credentials["username"])
        self.assertEqual(response.json()["email"], self.user_credentials["email"])

    def test_5_get_fake_user(self):
        headers = {"Authorization": "Bearer fake_token_123"}
        response = requests.post(
            f"http://{self.API_HOST}:{self.API_PORT}/verify", headers=headers
        )
        print(response)
        self.assertEqual(response.status_code, 401)
        data: dict[str, str] = response.json()  # pyright: ignore[reportAny]
        self.assertIsInstance(data, dict)
        self.assertIsInstance(data["message"], str)
        self.assertEqual(data["message"], "Invalid/Expired Token")  # Updated message


def cleanup():
    if SELFRUN:
        if proc.poll() is None:
            proc.terminate()
            try:
                _ = proc.wait(timeout=5)
                print("Server stopped")
            except subprocess.TimeoutExpired:
                proc.kill()
    try:
        os.remove("test.db")
        print("test.db removed")
    except FileNotFoundError:
        pass


if __name__ == "__main__":
    _ = atexit.register(cleanup)
    env = os.environ.copy()
    env["CONFIG_PATH"] = "testconfig.toml"
    if SELFRUN:
        proc = subprocess.Popen(
            ["cargo", "run"],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
    _ = wait_for_server(TestBruthAPI.API_HOST, int(TestBruthAPI.API_PORT))
    # ---- Custom top-to-bottom test runner ----
    print("Running tests in order...")
    test_instance = TestBruthAPI()

    # Collect all methods that start with "test_"
    test_methods = [m for m in dir(TestBruthAPI) if m.startswith("test_")]
    test_methods.sort()  # optional, ensures deterministic order

    passed = 0
    failed = 0
    errors = 0
    total = 0

    for method_name in test_methods:
        total += 1
        print(f"\nRunning {method_name}...")
        method = getattr(test_instance, method_name)  # pyright: ignore[reportAny]
        try:
            method()
            print(f"{method_name} ✅ Passed")
            passed += 1
        except AssertionError as e:
            print(f"{method_name} ❌ Failed: {e}")
            failed += 1
        except Exception as e:
            print(f"{method_name} 💣 Error: {e}")
            errors += 1

    print(f"\n{BOLD}{CYAN}================ Test Summary ================{RESET}")
    print(f"{BOLD}Total tests run :{RESET} {total}")
    print(f"{BOLD}{GREEN}✅ Passed       :{RESET} {passed}")
    if failed > 0:
        print(f"{BOLD}{RED}❌ Failed       :{RESET} {failed}")
    if errors > 0:
        print(f"{BOLD}{YELLOW}💣 Errors       :{RESET} {errors}")
    print(f"{BOLD}{CYAN}=============================================={RESET}")
