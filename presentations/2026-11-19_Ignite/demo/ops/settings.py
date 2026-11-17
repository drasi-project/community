"""The demo's ports, names and keys, from demo/ports.env (one place for all of them)."""
import os
from pathlib import Path

DEMO = Path(__file__).resolve().parents[1]
# The one Cargo build folder of this copy (demo/.cargo/config.toml, ops/native-build.sh).
TARGET = Path(os.environ.get("CARGO_TARGET_DIR") or DEMO.parents[3] / ".cargo-target")


def load(path=None):
    path = Path(path or os.environ.get("IGNITE_SETTINGS") or DEMO / "ports.env")
    if not path.is_absolute():
        path = DEMO / path
    values = {}
    for line in path.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            key, separator, value = line.partition("=")
            if not separator:
                raise ValueError(f"{path}: expected KEY=VALUE, got {line!r}")
            values[key.strip()] = value.strip()
    # An exported variable of the same name wins, as it does for compose.
    values.update({key: os.environ[key] for key in values if os.environ.get(key)})
    return values


VALUES = load()
PROJECT = VALUES["IGNITE_PROJECT"]
PUBLICATION = VALUES["IGNITE_PG_PUBLICATION"]
SLOT = VALUES["IGNITE_PG_SLOT"]
STORAGE_KEY = VALUES["IGNITE_STORAGE_KEY"]


def port(name):
    """port("WORKLOAD") -> IGNITE_WORKLOAD_PORT as an int."""
    return int(VALUES[f"IGNITE_{name}_PORT"])


def url(name, host="127.0.0.1"):
    return f"http://{host}:{port(name)}"


def container(service):
    """The compose container of a service in this copy's project."""
    return f"{PROJECT}-{service}-1"


def environment():
    """Every setting, for passing to a child process."""
    return dict(VALUES)


if __name__ == "__main__":
    # `python3 ops/settings.py` prints shell assignments: eval "$(python3 ops/settings.py)"
    for key, value in VALUES.items():
        print(f"export {key}='{value}'")
