#!/usr/bin/env python3
"""A small OTLP metrics sender for tests: hand-encoded protobuf over OTLP/HTTP, no dependencies.

Drasi's OpenTelemetry source accepts binary protobuf only (not OTLP/JSON), over gRPC or HTTP.
The tests send through the Collector's OTLP/HTTP receiver (IGNITE_OTLP_PORT), which forwards the
health probes to Drasi over gRPC when IGNITE_HEALTH_SOURCE=otel, or straight to a source that
serves HTTP.

    python3 tests/otlp.py [--url URL] METRIC VALUE SERVICE [key=value ...]
    python3 tests/otlp.py --self-test
"""
import struct
import sys
import time
import urllib.request


def _varint(n):
    out = bytearray()
    while True:
        byte, n = n & 0x7F, n >> 7
        out.append(byte | (0x80 if n else 0))
        if not n:
            return bytes(out)


def _field(number, wire, payload):
    tag = _varint((number << 3) | wire)
    return tag + (_varint(len(payload)) + payload if wire == 2 else payload)


def _message(number, body):
    return _field(number, 2, body)


def _string(number, text):
    return _field(number, 2, text.encode())


def _attribute(key, value):
    # KeyValue{1: key, 2: AnyValue{1: string_value}}
    return _string(1, key) + _message(2, _string(1, str(value)))


def gauge_request(metric, value, service, attributes=None, time_ns=None):
    """An ExportMetricsServiceRequest with one double gauge point."""
    point = (
        b"".join(_message(7, _attribute(k, v)) for k, v in (attributes or {}).items())
        + _field(3, 1, struct.pack("<Q", time_ns or time.time_ns()))  # time_unix_nano
        + _field(4, 1, struct.pack("<d", float(value)))  # as_double
    )
    metric_message = _string(1, metric) + _message(5, _message(1, point))  # name, gauge{data_points}
    resource = _message(1, _attribute("service.name", service))  # Resource.attributes
    resource_metrics = _message(1, resource) + _message(2, _message(2, metric_message))
    return _message(1, resource_metrics)


def send(url, metric, value, service, attributes=None):
    """POST one gauge point; returns the HTTP status (200 when accepted)."""
    body = gauge_request(metric, value, service, attributes)
    request = urllib.request.Request(url, body, {"Content-Type": "application/x-protobuf"})
    with urllib.request.urlopen(request, timeout=5) as response:
        return response.status


def probe(url, deployment, status, run_epoch, service="ai-gateway"):
    """One health probe as the AI service exports it (workloads/src/probe.rs)."""
    code = {"ok": 0, "throttled": 1, "failed": 2, "unknown": 3}[status]
    return send(url, "deployment_health_probe_status", code, service, {"deployment": deployment, "run_epoch": run_epoch})


def _decode(data):
    """One protobuf message's fields as [(number, bytes)]; the caller decodes nested messages."""
    def varint(i):
        value, shift = 0, 0
        while True:
            byte = data[i]
            value, shift, i = value | (byte & 0x7F) << shift, shift + 7, i + 1
            if not byte & 0x80:
                return value, i

    fields, i = [], 0
    while i < len(data):
        key, i = varint(i)
        number, wire = key >> 3, key & 7
        if wire == 1:
            fields.append((number, data[i:i + 8]))
            i += 8
        elif wire == 2:
            length, i = varint(i)
            fields.append((number, data[i:i + length]))
            i += length
        else:
            raise ValueError(f"unexpected wire type {wire}")
    return fields


def _self_test():
    request = gauge_request("deployment_health_probe_status", 2, "ai-gateway",
                            {"deployment": "EU-A", "run_epoch": "r"}, time_ns=1_000_000_000)
    first = lambda data, number: next(v for k, v in _decode(data) if k == number)  # noqa: E731
    resource_metrics = first(request, 1)
    service = first(first(resource_metrics, 1), 1)
    assert (first(service, 1), first(first(service, 2), 1)) == (b"service.name", b"ai-gateway")
    metric = first(first(resource_metrics, 2), 2)
    assert first(metric, 1) == b"deployment_health_probe_status"
    point = first(first(metric, 5), 1)
    attributes = [(first(v, 1), first(first(v, 2), 1)) for k, v in _decode(point) if k == 7]
    assert attributes == [(b"deployment", b"EU-A"), (b"run_epoch", b"r")], attributes
    assert struct.unpack("<Q", first(point, 3))[0] == 1_000_000_000
    assert struct.unpack("<d", first(point, 4))[0] == 2.0
    print("otlp self-test: ok")


def main(argv):
    if argv == ["--self-test"]:
        return _self_test()
    url = "http://127.0.0.1:4318/v1/metrics"
    if argv[:1] == ["--url"]:
        url, argv = argv[1], argv[2:]
    metric, value, service, *rest = argv
    attributes = dict(item.split("=", 1) for item in rest)
    print(send(url, metric, value, service, attributes))


if __name__ == "__main__":
    main(sys.argv[1:])
