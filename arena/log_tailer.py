"""Tail MTGA's Player.log and emit parsed game events in real time.

LogEventParser mirrors parse_game.parse_log, but works incrementally on
text chunks so it can run against a growing file. PlayerLogTailer follows
the file by polling, and survives partial lines, truncation, and rotation
to Player-prev.log when Arena restarts.

The tailer opens the file per poll instead of holding it open: on Windows
a held-open file cannot be renamed, which would break Arena's own log
rotation.
"""

import codecs
import json
import os
from pathlib import Path


class LogEventParser:
    """Incrementally parse Player.log text into (event_type, data) events.

    event_type: 'gre', 'match_state', 'client_msg' — same as
    parse_game.parse_log.
    """

    def __init__(self):
        self._tail = ""  # partial last line, no newline yet
        self._accum = None  # multi-line client message lines, or None
        self._depth = 0

    def feed(self, text):
        """Consume a text chunk and return the list of completed events."""
        events = []
        self._tail += text
        while True:
            nl = self._tail.find("\n")
            if nl < 0:
                break
            line = self._tail[:nl].rstrip("\r")
            self._tail = self._tail[nl + 1:]
            event = self._feed_line(line)
            if event:
                events.append(event)
        return events

    def reset(self):
        """Drop buffered state (call when the underlying file is replaced)."""
        self._tail = ""
        self._accum = None
        self._depth = 0

    def _feed_line(self, line):
        if self._accum is not None:
            self._accum.append(line)
            self._depth += line.count("{") - line.count("}")
            if self._depth <= 0:
                blob = "\n".join(self._accum)
                self._accum = None
                try:
                    return ("client_msg", json.loads(blob))
                except json.JSONDecodeError:
                    return None
            return None

        # Server→Client GRE messages and match state (single-line JSON)
        if line.startswith("{ ") or line.startswith('{"'):
            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                return None
            if "greToClientEvent" in data:
                return ("gre", data)
            if "matchGameRoomStateChangedEvent" in data:
                return ("match_state", data)
            return None

        # Client→Server messages (multi-line JSON after a header line)
        if "ClientToGremessage" in line or "ClientToGreuimessage" in line:
            self._accum = []
            self._depth = 0
        return None


class PlayerLogTailer:
    """Follow Player.log across appends, truncation, and rotation.

    Call poll() repeatedly; each call returns the new (event_type, data)
    events since the last call. The file does not need to exist yet.
    """

    def __init__(self, path, from_start=False, parser=None):
        """`parser` is any object with feed(text) -> list and reset();
        defaults to LogEventParser. collect.MatchSlicer uses this to get
        whole matches from poll() instead of single events."""
        self.path = Path(path)
        self.prev_path = self.path.with_name("Player-prev.log")
        self._from_start = from_start
        self._offset = None  # None until the file is seen for the first time
        self._ino = None
        self._parser = parser or LogEventParser()
        self._decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def poll(self):
        """Return new events since the last call."""
        try:
            fh = open(self.path, "rb")
        except OSError:
            return []

        events = []
        with fh:
            st = os.fstat(fh.fileno())
            if self._offset is None:
                # First sighting: start at the end unless replaying history.
                self._offset = 0 if self._from_start else st.st_size
            elif self._is_new_file(st):
                events.extend(self._drain_rotated())
                self._parser.reset()
                self._decoder.reset()
                self._offset = 0
            self._ino = st.st_ino
            fh.seek(self._offset)
            events.extend(self._read_all(fh, self._parser, self._decoder))
            self._offset = fh.tell()
        return events

    def _is_new_file(self, st):
        if st.st_ino and self._ino and st.st_ino != self._ino:
            return True  # Arena restarted: Player.log was replaced
        return st.st_size < self._offset  # truncated in place

    def _drain_rotated(self):
        """Read the tail of the old file after it moved to Player-prev.log."""
        try:
            fh = open(self.prev_path, "rb")
        except OSError:
            return []
        with fh:
            st = os.fstat(fh.fileno())
            if not (st.st_ino and self._ino and st.st_ino == self._ino):
                return []  # not the file we were reading
            if st.st_size < self._offset:
                return []
            fh.seek(self._offset)
            return self._read_all(fh, self._parser, self._decoder)

    @staticmethod
    def _read_all(fh, parser, decoder):
        events = []
        while True:
            chunk = fh.read(65536)
            if not chunk:
                break
            events.extend(parser.feed(decoder.decode(chunk)))
        return events
