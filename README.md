# bruth
bruh + auth = bruth  
Yes I have no idea how to name it

---

## Configuration

### 1. Config file (`config.toml`)

Controls non-secret runtime options — host, port, database path, bcrypt cost, token duration.  
Copy and adjust as needed; **do not put secrets here**.

```toml
[main]
host = "0.0.0.0"
port = 8080

[database]
path = "users.db"

[hash]
cost = 12          # bcrypt work factor — never go below 10 in production

[token]
duration = 900     # JWT lifetime in seconds (15 min)
```

You can point to a different config file at startup:

```sh
CONFIG_PATH=/path/to/my.toml ./bruth
```

---

### 2. Secrets (`.env`)

All secrets are loaded from environment variables.  
Copy the template and fill in real values:

```sh
cp .env.example .env
$EDITOR .env
```

**`.env.example`**:

```dotenv
# Sign / verify JWTs — generate with: openssl rand -hex 32
JWT_SECRET=replace_with_a_random_256bit_hex_string

# Service-to-service key for /users and /users/search (X-Api-Key header)
# generate with: openssl rand -hex 32
SERVICE_API_KEY=replace_with_a_random_256bit_hex_string

# Exact origin your front-end is served from
CORS_ALLOWED_ORIGIN=https://your-frontend-domain.com
```

> **Note:** The server will refuse to start if any of these three variables are missing.

---

## API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/register` | — | Create a new user |
| `POST` | `/login` | — | Log in, receive JWT |
| `POST` | `/verify` | `Authorization: Bearer <token>` | Validate a JWT and return user info |
| `GET`  | `/users` | `X-Api-Key: <key>` | List all users *(service only)* |
| `GET`  | `/users/search?q=&limit=` | `X-Api-Key: <key>` | Search users *(service only)* |

### Service-to-service endpoints

`/users` and `/users/search` are intended to be called by **other backend services**, not by end-user clients.  
Every request must include:

```
X-Api-Key: <SERVICE_API_KEY>
```

The key is compared in constant time to prevent timing attacks.  
Keep this key out of any front-end code.

---

## Running

```sh
# Development
cargo run

# Production (with .env in the working directory)
cargo build --release
./target/release/bruth

# Or override the config path
CONFIG_PATH=/etc/bruth/config.toml ./target/release/bruth
```

## Tests

```sh
python test.py
```

The test runner starts the server automatically using `testconfig.toml` and a `.env` (or env vars) for secrets.  
Make sure `JWT_SECRET`, `SERVICE_API_KEY`, and `CORS_ALLOWED_ORIGIN` are set in your environment or a `.env` file before running tests.
