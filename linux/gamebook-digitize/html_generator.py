"""Stage 9 -- Convert gamebook markdown into a self-contained HTML player.

Two entry points:
    main_standalone(md_path, html_path)   -- called from CLI
    generate_from_markdown(md_path, html_path)  -- called from pipeline
"""

from __future__ import annotations

import base64
import html
import logging
import mimetypes
import os
import re
import sys
from typing import NamedTuple

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Cross-reference patterns
# ---------------------------------------------------------------------------

XREF_FR = re.compile(
    r'\b(rendez-vous au|rendez vous au|allez au|tournez-vous au|passez au|au paragraphe)\s+(\d{1,3})\b',
    re.IGNORECASE,
)
XREF_EN = re.compile(
    r'\b(go to (?:\u00a7\s*)?|turn to (?:\u00a7\s*)?|go to paragraph |turn to paragraph )(\d{1,3})\b',
    re.IGNORECASE,
)

# Section heading
SECTION_RE = re.compile(r'^## \u00a7 (\d+)\s*$', re.MULTILINE)

# Image line inside a section
IMAGE_RE = re.compile(r'^!\[([^\]]*)\]\(([^)]+)\)\s*$')

# Reference image block
REF_COMMENT_RE = re.compile(r'<!--\s*REF:\s*(.+?)\s*-->')
REF_IMAGE_RE = re.compile(r'!\[([^\]]*)\]\(([^)]+)\)')

# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------


class ParsedSection(NamedTuple):
    number: int
    body_html: str


class ParsedMarkdown(NamedTuple):
    title: str
    lang: str
    ref_images: list[tuple[str, str]]  # (label, data_uri)
    sections: list[ParsedSection]


# ---------------------------------------------------------------------------
# Image helpers
# ---------------------------------------------------------------------------


def _image_to_data_uri(path: str) -> str:
    """Read an image file and return a base64 data URI string."""
    mime, _ = mimetypes.guess_type(path)
    if mime is None:
        mime = 'application/octet-stream'
    with open(path, 'rb') as f:
        data = f.read()
    encoded = base64.b64encode(data).decode('ascii')
    return f'data:{mime};base64,{encoded}'


# ---------------------------------------------------------------------------
# Parsing
# ---------------------------------------------------------------------------


def _parse_frontmatter(text: str) -> tuple[dict[str, str], str]:
    """Extract YAML-like frontmatter and return (metadata, remaining text).

    Handles simple ``key: value`` pairs only -- no YAML library needed.
    """
    if not text.startswith('---'):
        return {}, text
    end = text.find('\n---', 3)
    if end == -1:
        return {}, text
    fm_block = text[3:end].strip()
    rest = text[end + 4:]  # skip past closing ---
    meta: dict[str, str] = {}
    for line in fm_block.splitlines():
        line = line.strip()
        if ':' in line:
            key, _, val = line.partition(':')
            meta[key.strip()] = val.strip()
    return meta, rest


def _extract_ref_images(text: str, base_dir: str) -> tuple[list[tuple[str, str]], str]:
    """Pull out reference image blocks and return (ref_images, cleaned text).

    Each block is a ``<!-- REF: label -->`` comment followed by an image line.
    """
    refs: list[tuple[str, str]] = []
    lines = text.splitlines()
    cleaned: list[str] = []
    i = 0
    while i < len(lines):
        m = REF_COMMENT_RE.match(lines[i].strip())
        if m:
            label = m.group(1)
            # Look for the image on the next non-blank line
            j = i + 1
            while j < len(lines) and lines[j].strip() == '':
                j += 1
            if j < len(lines):
                img_m = REF_IMAGE_RE.match(lines[j].strip())
                if img_m:
                    img_path = os.path.join(base_dir, img_m.group(2))
                    if os.path.isfile(img_path):
                        refs.append((label, _image_to_data_uri(img_path)))
                    else:
                        logger.warning('Reference image not found: %s', img_path)
                    i = j + 1
                    continue
        cleaned.append(lines[i])
        i += 1
    return refs, '\n'.join(cleaned)


def _linkify_xrefs(text: str, lang: str) -> str:
    """Replace gamebook cross-references with HTML links."""

    def _repl(m: re.Match) -> str:  # type: ignore[type-arg]
        num = m.group(2)
        full = m.group(0)
        return f'<a href="#section-{num}" class="xref">{html.escape(full)}</a>'

    # Apply both language patterns to catch mixed-language references
    text = XREF_FR.sub(_repl, text)
    text = XREF_EN.sub(_repl, text)
    return text


def _section_body_to_html(body: str, lang: str, base_dir: str) -> str:
    """Convert the raw body text of a section into HTML fragments."""
    paragraphs = re.split(r'\n\s*\n', body.strip())
    parts: list[str] = []
    for para in paragraphs:
        para = para.strip()
        if not para:
            continue
        # Skip markdown horizontal rules (--- separators between sections)
        if re.match(r'^-{3,}\s*$', para):
            continue
        # Check if the paragraph is a single image line
        img_m = IMAGE_RE.match(para)
        if img_m:
            alt = html.escape(img_m.group(1))
            img_path = os.path.join(base_dir, img_m.group(2))
            if os.path.isfile(img_path):
                uri = _image_to_data_uri(img_path)
                parts.append(f'<img src="{uri}" alt="{alt}">')
            else:
                logger.warning('Section image not found: %s', img_path)
            continue
        # Regular text paragraph -- escape, then linkify
        escaped = html.escape(para)
        # Preserve line breaks within a paragraph
        escaped = escaped.replace('\n', '<br>\n')
        linked = _linkify_xrefs(escaped, lang)
        parts.append(f'<p>{linked}</p>')
    return '\n'.join(parts)


def parse_gamebook_markdown(md_path: str) -> ParsedMarkdown:
    """Parse a gamebook markdown file into structured data."""
    base_dir = os.path.dirname(os.path.abspath(md_path))
    with open(md_path, 'r', encoding='utf-8') as f:
        text = f.read()

    meta, text = _parse_frontmatter(text)
    title = meta.get('title', 'Gamebook')
    lang = meta.get('lang', 'en')

    ref_images, text = _extract_ref_images(text, base_dir)

    # Split into sections
    splits = SECTION_RE.split(text)
    # splits[0] is preamble (before first section), then alternating: number, body
    sections: list[ParsedSection] = []
    i = 1
    while i < len(splits) - 1:
        num = int(splits[i])
        body_raw = splits[i + 1]
        body_html = _section_body_to_html(body_raw, lang, base_dir)
        sections.append(ParsedSection(number=num, body_html=body_html))
        i += 2

    return ParsedMarkdown(
        title=title,
        lang=lang,
        ref_images=ref_images,
        sections=sections,
    )


# ---------------------------------------------------------------------------
# HTML rendering
# ---------------------------------------------------------------------------

_CSS = r"""
:root {
  --bg: #1a1a2e;
  --surface: #16213e;
  --text: #e0e0e0;
  --text-muted: #8892a0;
  --accent: #e94560;
  --accent-gold: #f0c040;
  --link: #0f9b8e;
  --nav-bg: #0f3460;
  --sidebar-bg: #16213e;
  --divider: #2a2a4a;
}

*, *::before, *::after { box-sizing: border-box; }

html {
  scroll-behavior: smooth;
}

body {
  margin: 0;
  padding: 0;
  background: var(--bg);
  color: var(--text);
  font-family: system-ui, -apple-system, sans-serif;
  display: grid;
  grid-template-columns: auto 1fr;
  min-height: 100vh;
  user-select: text;
  -webkit-user-select: text;
}

/* ---- Sidebar ---- */

#sidebar {
  width: 280px;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--divider);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: width 0.25s ease;
}

#sidebar.collapsed {
  width: 0;
  border-right: none;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem;
  border-bottom: 1px solid var(--divider);
  white-space: nowrap;
}

.sidebar-header h2 {
  margin: 0;
  font-size: 1rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

#sidebar-toggle {
  background: none;
  border: 1px solid var(--divider);
  color: var(--text-muted);
  cursor: pointer;
  font-size: 1rem;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  line-height: 1;
}

#sidebar-toggle:hover {
  color: var(--text);
  border-color: var(--text-muted);
}

.sidebar-content {
  overflow-y: auto;
  padding: 1rem;
  flex: 1;
}

.ref-block {
  margin-bottom: 1.5rem;
}

.ref-block .ref-label {
  font-size: 0.85rem;
  color: var(--text-muted);
  margin-bottom: 0.4rem;
}

.ref-block img {
  max-width: 100%;
  border-radius: 6px;
  display: block;
}

/* ---- Main wrapper ---- */

#main-wrapper {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

/* ---- Nav bar ---- */

#section-nav {
  position: sticky;
  top: 0;
  z-index: 100;
  background: var(--nav-bg);
  display: flex;
  gap: 0.25rem;
  padding: 0.5rem 1rem;
  overflow-x: auto;
  border-bottom: 1px solid var(--divider);
  scrollbar-width: thin;
}

#section-nav::-webkit-scrollbar {
  height: 4px;
}

#section-nav::-webkit-scrollbar-thumb {
  background: var(--divider);
  border-radius: 2px;
}

.nav-item {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 2rem;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-size: 0.85rem;
  color: var(--text-muted);
  text-decoration: none;
  background: transparent;
  transition: background 0.15s, color 0.15s;
  cursor: pointer;
}

.nav-item:hover {
  background: rgba(255,255,255,0.08);
  color: var(--text);
}

.nav-item.active {
  background: var(--accent-gold);
  color: #1a1a2e;
  font-weight: 700;
}

/* ---- Content ---- */

#content {
  max-width: 50rem;
  width: 100%;
  margin: 0 auto;
  padding: 2rem;
}

.section {
  scroll-margin-top: 3.5rem;
}

.section h2 {
  color: var(--accent);
  font-size: 1.5rem;
  margin: 0 0 1rem 0;
}

.section-body {
  font-family: Georgia, "Times New Roman", serif;
  font-size: 17px;
  line-height: 1.7;
}

.section-body p {
  margin: 0 0 1rem 0;
}

.section-body img {
  max-width: 100%;
  border-radius: 8px;
  margin: 1rem auto;
  display: block;
}

.section-body a.xref {
  color: var(--link);
  text-decoration: underline;
  cursor: pointer;
}

.section-body a.xref:hover {
  color: #12c4b3;
}

hr.section-divider {
  border: none;
  border-top: 1px solid var(--divider);
  margin: 2rem 0;
}

/* ---- Mobile ---- */

@media (max-width: 768px) {
  body {
    grid-template-columns: 1fr;
  }
  #sidebar {
    display: none;
  }
  #sidebar.mobile-open {
    display: flex;
    position: fixed;
    top: 0; left: 0; bottom: 0;
    z-index: 200;
    width: 260px;
    box-shadow: 4px 0 20px rgba(0,0,0,0.5);
  }
  .nav-item {
    font-size: 0.75rem;
    min-width: 1.6rem;
    padding: 0.15rem 0.35rem;
  }
  #content {
    padding: 1rem;
  }
}
"""

_JS = r"""
(function() {
  // Sidebar toggle
  var sidebar = document.getElementById('sidebar');
  var toggleBtn = document.getElementById('sidebar-toggle');
  if (toggleBtn) {
    toggleBtn.addEventListener('click', function() {
      sidebar.classList.toggle('collapsed');
      toggleBtn.textContent = sidebar.classList.contains('collapsed') ? '\u25B6' : '\u25C0';
    });
  }

  // Nav items and sections
  var navItems = document.querySelectorAll('.nav-item');
  var sections = document.querySelectorAll('.section');

  // Smooth scroll for nav clicks
  navItems.forEach(function(item) {
    item.addEventListener('click', function(e) {
      e.preventDefault();
      var target = document.querySelector(item.getAttribute('href'));
      if (target) {
        target.scrollIntoView({ behavior: 'smooth' });
      }
    });
  });

  // Smooth scroll for xref clicks
  document.querySelectorAll('a.xref').forEach(function(link) {
    link.addEventListener('click', function(e) {
      e.preventDefault();
      var target = document.querySelector(link.getAttribute('href'));
      if (target) {
        target.scrollIntoView({ behavior: 'smooth' });
        // Update nav highlighting after scroll
        setTimeout(function() { highlightNav(link.getAttribute('href').replace('#', '')); }, 100);
      }
    });
  });

  // Scroll-based nav highlighting with IntersectionObserver
  function highlightNav(sectionId) {
    navItems.forEach(function(item) {
      if (item.getAttribute('href') === '#' + sectionId) {
        item.classList.add('active');
        item.scrollIntoView({ inline: 'center', block: 'nearest' });
      } else {
        item.classList.remove('active');
      }
    });
  }

  if ('IntersectionObserver' in window) {
    var observer = new IntersectionObserver(function(entries) {
      entries.forEach(function(entry) {
        if (entry.isIntersecting) {
          highlightNav(entry.target.id);
        }
      });
    }, { threshold: 0.2 });

    sections.forEach(function(sec) {
      observer.observe(sec);
    });
  }
})();
"""


def render_html(parsed: ParsedMarkdown) -> str:
    """Render parsed markdown into a complete self-contained HTML string."""
    lang = html.escape(parsed.lang)
    title = html.escape(parsed.title)

    # Build sidebar reference images
    ref_blocks: list[str] = []
    for label, data_uri in parsed.ref_images:
        ref_blocks.append(
            f'      <div class="ref-block">\n'
            f'        <div class="ref-label">{html.escape(label)}</div>\n'
            f'        <img src="{data_uri}" alt="{html.escape(label)}">\n'
            f'      </div>'
        )
    ref_html = '\n'.join(ref_blocks)

    # Build nav items
    nav_items: list[str] = []
    for sec in parsed.sections:
        nav_items.append(
            f'      <a href="#section-{sec.number}" class="nav-item">{sec.number}</a>'
        )
    nav_html = '\n'.join(nav_items)

    # Build section blocks
    section_blocks: list[str] = []
    for i, sec in enumerate(parsed.sections):
        section_blocks.append(
            f'      <div class="section" id="section-{sec.number}">\n'
            f'        <h2>&sect; {sec.number}</h2>\n'
            f'        <div class="section-body">\n'
            f'          {sec.body_html}\n'
            f'        </div>\n'
            f'      </div>'
        )
        if i < len(parsed.sections) - 1:
            section_blocks.append('      <hr class="section-divider">')
    sections_html = '\n'.join(section_blocks)

    return f"""<!DOCTYPE html>
<html lang="{lang}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>{_CSS}</style>
</head>
<body>
  <aside id="sidebar">
    <div class="sidebar-header">
      <h2>References</h2>
      <button id="sidebar-toggle" title="Toggle sidebar">\u25C0</button>
    </div>
    <div class="sidebar-content">
{ref_html}
    </div>
  </aside>

  <div id="main-wrapper">
    <nav id="section-nav">
{nav_html}
    </nav>

    <main id="content">
{sections_html}
    </main>
  </div>

  <script>{_JS}</script>
</body>
</html>
"""


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def generate_from_markdown(md_path: str, html_path: str) -> None:
    """Parse markdown and write HTML file.  Called from main pipeline."""
    parsed = parse_gamebook_markdown(md_path)
    out = render_html(parsed)
    with open(html_path, 'w', encoding='utf-8') as f:
        f.write(out)
    logger.info('Wrote HTML gamebook: %s (%d sections)', html_path, len(parsed.sections))


def main_standalone(md_path: str, html_path: str) -> None:
    """Standalone entry point for --from-markdown mode."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(levelname)s: %(message)s',
    )
    if not os.path.isfile(md_path):
        logger.error('Markdown file not found: %s', md_path)
        sys.exit(1)
    generate_from_markdown(md_path, html_path)
    print(f'Generated: {html_path}')


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f'Usage: {sys.argv[0]} <input.md> <output.html>', file=sys.stderr)
        sys.exit(1)
    main_standalone(sys.argv[1], sys.argv[2])
