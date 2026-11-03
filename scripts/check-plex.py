#!/usr/bin/env python3
"""Offline evidence gate: fonts are pinned; typography never alters terminal data."""
import hashlib
import json
from html.parser import HTMLParser
from pathlib import Path


class Cells(HTMLParser):
    def __init__(self):
        super().__init__()
        self.in_pre = False
        self.cell = None
        self.cells = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'pre':
            self.in_pre = True
        if tag == 'span' and self.in_pre:
            style = dict(part.split(':', 1) for part in attrs['style'].split(';') if ':' in part)
            self.cell = (style['color'], style['background'])

    def handle_endtag(self, tag):
        if tag == 'pre':
            self.in_pre = False
        if tag == 'span':
            self.cell = None

    def handle_data(self, data):
        if self.cell is not None:
            self.cells.extend((char, *self.cell) for char in data)


records = json.loads(Path('fonts/SOURCES.json').read_text())
assert len(records) == 8
for record in records:
    data = (Path('fonts') / record['file']).read_bytes()
    assert data[:4] == b'wOF2'
    assert len(data) == record['bytes']
    assert hashlib.sha256(data).hexdigest() == record['sha256']
    assert record['version'] == '5.3.0' and record['licence'] == 'SIL OFL 1.1'
    assert record['source'].startswith('https://cdn.jsdelivr.net/npm/@fontsource/')
count = 0
for gallery in ['runner', 'map', 'themes']:
    paths = sorted((Path('shots/plex/before') / gallery).glob('*.html'))
    paths = [path for path in paths if not path.name.startswith('contact-')]
    assert paths
    for before in paths:
        after = Path(str(before).replace('/before/', '/after/'))
        assert before.with_suffix('.txt').read_bytes() == after.with_suffix('.txt').read_bytes(), before
        parsers = [Cells(), Cells()]
        parsers[0].feed(before.read_text())
        parsers[1].feed(after.read_text())
        assert parsers[0].cells == parsers[1].cells, f'content/colour changed: {before}'
        html = after.read_text()
        assert html.count('@font-face') == 8
        assert html.count('rel="preload"') == 3
        assert 'font-weight:700' not in html
        assert '<script' not in html
        count += 1
print(f'PASS: 8 pinned fonts; {count} frames preserve every terminal glyph and foreground/background colour.')
