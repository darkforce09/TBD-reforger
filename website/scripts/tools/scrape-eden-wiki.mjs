#!/usr/bin/env node
/**
 * Scrape Eden Editor wiki pages from eden/wiki_manifest.yaml.
 * Outputs:
 *   artifacts/eden-wiki/<slug>.md
 *   artifacts/eden-feds-draft.jsonl
 *
 * Usage: node scripts/tools/scrape-eden-wiki.mjs [--force]
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, '../..');
const MANIFEST = path.join(
  ROOT,
  'Design_Docs/Mission_Creator_Architecture/eden/wiki_manifest.yaml',
);
const OUT_DIR = path.join(ROOT, 'artifacts/eden-wiki');
const JSONL = path.join(ROOT, 'artifacts/eden-feds-draft.jsonl');

const FORCE = process.argv.includes('--force');

const MIRRORS = {
  primary: 'https://community.bistudio.com/wiki/',
  fallback: 'https://community.bohemia.net/wiki/',
};

/** Minimal YAML parser for our manifest shape only */
function parseManifest(text) {
  const pages = [];
  let category = '';
  let current = null;

  for (const line of text.split('\n')) {
    const cat = line.match(/^  (\w+):$/);
    if (cat) {
      category = cat[1];
      continue;
    }
    const slug = line.match(/^\s+- slug: (.+)$/);
    if (slug) {
      current = { category, slug: slug[1].trim(), ticket: 'T-078', status: 'pending' };
      pages.push(current);
      continue;
    }
    const ticket = line.match(/^\s+ticket: (.+)$/);
    if (ticket && current) current.ticket = ticket[1].trim();
  }

  // dedupe by slug
  const seen = new Set();
  return pages.filter((p) => {
    if (seen.has(p.slug)) return false;
    seen.add(p.slug);
    return true;
  });
}

function slugToFilename(slug) {
  return slug.replace(/:/g, '_') + '.md';
}

function htmlToText(html) {
  let t = html;
  t = t.replace(/<script[\s\S]*?<\/script>/gi, '');
  t = t.replace(/<style[\s\S]*?<\/style>/gi, '');
  t = t.replace(/<h([1-6])[^>]*>(.*?)<\/h\1>/gi, (_, n, inner) => {
    const level = '#'.repeat(Math.min(Number(n) + 1, 6));
    return `\n${level} ${inner.replace(/<[^>]+>/g, '').trim()}\n`;
  });
  t = t.replace(/<li[^>]*>(.*?)<\/li>/gi, '- $1\n');
  t = t.replace(/<br\s*\/?>/gi, '\n');
  t = t.replace(/<\/p>/gi, '\n\n');
  t = t.replace(/<\/tr>/gi, '\n');
  t = t.replace(/<td[^>]*>/gi, '| ');
  t = t.replace(/<[^>]+>/g, '');
  t = t
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&#39;/g, "'")
    .replace(/&quot;/g, '"');
  t = t.replace(/\n{3,}/g, '\n\n');
  return t.trim();
}

function extractSections(text) {
  const sections = [];
  const lines = text.split('\n');
  let current = { heading: 'Introduction', lines: [] };

  for (const line of lines) {
    const m = line.match(/^(#{2,4})\s+(.+)$/);
    if (m) {
      if (current.lines.length) sections.push({ ...current });
      current = { heading: m[2].trim(), lines: [] };
    } else {
      current.lines.push(line);
    }
  }
  if (current.lines.length) sections.push(current);
  return sections;
}

function sectionToDraftId(slug, heading) {
  const base = slug.replace(/^Eden_Editor:_/, '').replace(/_/g, '-').toUpperCase();
  const h = heading
    .replace(/[^\w\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .toUpperCase()
    .slice(0, 40);
  return `WIKI-${base}-${h || 'INTRO'}`;
}

async function fetchPage(slug) {
  const urls = [MIRRORS.primary + slug, MIRRORS.fallback + slug];
  for (const url of urls) {
    try {
      const res = await fetch(url, {
        headers: { 'User-Agent': 'TBD-Eden-Doc-Scraper/1.0 (mission-creator docs)' },
      });
      if (!res.ok) continue;
      const html = await res.text();
      // MediaWiki content div
      const match =
        html.match(/<div[^>]+class="[^"]*mw-parser-output[^"]*"[^>]*>([\s\S]*?)<\/div>\s*<\/div>\s*<\/div>/i) ||
        html.match(/<div id="mw-content-text"[^>]*>([\s\S]*?)<\/div>\s*<\/div>/i);
      if (!match) continue;
      return { url, text: htmlToText(match[1]) };
    } catch {
      /* try next mirror */
    }
  }
  return null;
}

async function main() {
  if (!fs.existsSync(MANIFEST)) {
    console.error('Manifest not found:', MANIFEST);
    process.exit(1);
  }

  fs.mkdirSync(OUT_DIR, { recursive: true });

  const pages = parseManifest(fs.readFileSync(MANIFEST, 'utf8'));
  const drafts = [];
  const results = { scraped: 0, failed: 0, skipped: 0 };

  for (const page of pages) {
    const outFile = path.join(OUT_DIR, slugToFilename(page.slug));
    if (!FORCE && fs.existsSync(outFile)) {
      results.skipped++;
      continue;
    }

    process.stdout.write(`Fetching ${page.slug}... `);
    const fetched = await fetchPage(page.slug);
    if (!fetched) {
      console.log('FAILED');
      results.failed++;
      drafts.push({
        id: `WIKI-${page.slug.replace(/:/g, '-')}-FAILED`,
        wiki_url: MIRRORS.primary + page.slug,
        ui_surface: '—',
        feature_kind: 'ui_chrome',
        draft_trigger: 'UNVERIFIED — fetch failed',
        raw_excerpt: '',
        status: 'failed',
        category: page.category,
      });
      continue;
    }

    const header = `# ${page.slug}\n\nSource: ${fetched.url}\n\n---\n\n`;
    fs.writeFileSync(outFile, header + fetched.text, 'utf8');

    const sections = extractSections(fetched.text);
    for (const sec of sections) {
      const excerpt = sec.lines.join('\n').trim().slice(0, 800);
      if (excerpt.length < 20) continue;
      drafts.push({
        id: sectionToDraftId(page.slug, sec.heading),
        wiki_url: `${fetched.url}#${sec.heading.replace(/\s+/g, '_')}`,
        ui_surface: inferSurface(page.slug, sec.heading),
        feature_kind: inferKind(page.slug, sec.heading),
        draft_trigger: inferTrigger(excerpt),
        raw_excerpt: excerpt,
        status: 'draft',
        category: page.category,
        ticket: page.ticket,
      });
    }

    console.log('OK');
    results.scraped++;
    await new Promise((r) => setTimeout(r, 300));
  }

  fs.writeFileSync(JSONL, drafts.map((d) => JSON.stringify(d)).join('\n') + '\n', 'utf8');

  console.log('\nDone:', results);
  console.log('Wiki cache:', OUT_DIR);
  console.log('Draft JSONL:', JSONL, `(${drafts.length} rows)`);
}

function inferSurface(slug, heading) {
  const s = slug + heading;
  if (/Asset_Browser|Object_Categorization/i.test(s)) return 'AssetBrowser';
  if (/Toolbar|Transformation_Widget|Entity_Transforming/i.test(s)) return 'Toolbar';
  if (/Menu_Bar/i.test(s)) return 'MenuBar';
  if (/Status_Bar/i.test(s)) return 'StatusBar';
  if (/Entity_Attributes|Setting_Attributes/i.test(s)) return 'AttributesDialog';
  if (/Scenario_Attributes/i.test(s)) return 'ScenarioAttributes';
  if (/Context_Menu|Connecting/i.test(s)) return 'ContextMenu';
  if (/Layer/i.test(s)) return 'EntityList';
  if (/Entity_Placing/i.test(s)) return 'View';
  return '—';
}

function inferKind(slug, heading) {
  if (/Setting_Attributes|Entity_Attributes/i.test(slug)) return 'attribute_field';
  if (/Connecting/i.test(slug)) return 'connection_type';
  if (/Asset_Browser/i.test(slug) && /Mode|Submode|Search/i.test(heading)) return 'browser_mode';
  if (/Actions/i.test(slug)) return 'engine_action';
  return 'interaction';
}

function inferTrigger(excerpt) {
  const lower = excerpt.toLowerCase();
  if (lower.includes('left mouse button') || lower.includes('lmb')) return 'LMB';
  if (lower.includes('right mouse button') || lower.includes('rmb')) return 'RMB';
  if (lower.includes('ctrl')) return 'Ctrl + (see excerpt)';
  if (lower.includes('shift')) return 'Shift + (see excerpt)';
  if (lower.includes('f1')) return 'F1–F6 or tab (see excerpt)';
  return 'See wiki excerpt';
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
