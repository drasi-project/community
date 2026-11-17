#!/usr/bin/env python3
"""Create a shell-compatible, private local environment without printing secrets."""
import base64
import os
from pathlib import Path
import secrets
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import settings  # noqa: E402

path = Path(__file__).resolve().parents[1] / ".env"
if path.exists():
    if path.stat().st_mode & 0o077:
        raise SystemExit(f"{path} must have permissions 600")
    print("Existing private demo/.env retained.")
else:
    secret = lambda: secrets.token_hex(24)
    password, public_key, secret_key = secret(), "pk-lf-" + secret(), "sk-lf-" + secret()
    values = {
        "POSTGRES_USER": "ignite",
        "POSTGRES_DB": "ignite",
        "POSTGRES_PASSWORD": password,
        "DATABASE_URL": f"postgresql://ignite:{password}@127.0.0.1:{settings.port('POSTGRES')}/ignite",
        "LANGFUSE_DATABASE_URL": f"postgresql://ignite:{password}@postgres:5432/langfuse",
        "LANGFUSE_PUBLIC_KEY": public_key,
        "LANGFUSE_SECRET_KEY": secret_key,
        "LANGFUSE_AUTH_BASE64": base64.b64encode(f"{public_key}:{secret_key}".encode()).decode(),
        "LANGFUSE_SALT": secret(),
        "LANGFUSE_ENCRYPTION_KEY": secrets.token_hex(32),
        "LANGFUSE_NEXTAUTH_SECRET": secret(),
        "LANGFUSE_ADMIN_EMAIL": "operator@ignite.local",
        "LANGFUSE_ADMIN_PASSWORD": secret(),
        "CLICKHOUSE_PASSWORD": secret(),
        "REDIS_PASSWORD": secret(),
        "MINIO_PASSWORD": secret(),
        "GRAFANA_ADMIN_PASSWORD": secret(),
        "DRASI_ALERT_URL": "",
    }
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "w") as stream:
        stream.writelines(f"{key}='{value}'\n" for key, value in values.items())
    print("Created private demo/.env; no credentials printed.")
