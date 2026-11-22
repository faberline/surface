#!/usr/bin/env python3
import json
import os
import socket
import subprocess
import sys
from pathlib import Path

def read_native_snapshot(session_file):
    session = json.loads(Path(session_file).read_text())
    addr, port = session['endpoint'].rsplit(':', 1)
    assert addr == '127.0.0.1'
    request = {
        'token': session['token'],
        'request': {'version': 1, 'id': 687, 'command': {'method': 'snapshot'}},
    }
    with socket.create_connection((addr, int(port)), timeout=10) as connection:
        connection.sendall(json.dumps(request).encode() + b'\n')
        with connection.makefile('rb') as response:
            frame = response.readline(16 * 1024 * 1024 + 1)
    result = {'wire_bytes': len(frame), 'wire_newline': frame.endswith(b'\n')}
    if result['wire_newline']:
        try:
            payload = json.loads(frame)
            result.update(wire_success=payload.get('ok'),
                          wire_code=payload.get('code'),
                          snapshot_version=payload.get('result', {}).get('version'))
        except (ValueError, AttributeError) as error:
            result.update(wire_parse=type(error).__name__)
    return result

binary = os.environ['SURFACE_BRIDGE_DIAGNOSTIC_BIN']
arguments = sys.argv[1:]
if arguments[-2:] != ['ui', 'snapshot'] or '--app-session' not in arguments:
    os.execv(binary, [binary, *arguments])
result = subprocess.run([binary, *arguments], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
entry = {'exit': result.returncode, 'stdout_bytes': len(result.stdout),
         'stderr_bytes': len(result.stderr)}
if result.returncode:
    fields = ['incomplete or oversized application response', 'invalid native application response',
              'application response version/id mismatch', 'application did not reply',
              'Error:', 'connection reset']
    entry['stderr_indicators'] = [field for field in fields if field.encode() in result.stderr]
    try:
        entry.update(read_native_snapshot(arguments[arguments.index('--app-session') + 1]))
    except (OSError, ValueError, KeyError) as error:
        entry['wire_error'] = type(error).__name__
with open(os.environ['SURFACE_BRIDGE_DIAGNOSTIC_LOG'], 'a', encoding='utf-8') as report:
    print(json.dumps(entry), file=report)
sys.stdout.buffer.write(result.stdout)
sys.stderr.buffer.write(result.stderr)
sys.exit(result.returncode)
