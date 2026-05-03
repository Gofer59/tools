#!/usr/bin/env node
// math-speak SRE daemon: long-lived Speech Rule Engine bridge.
//
// Protocol: JSON-line stdin/stdout.
//   Request:  {"id": <int>, "input": "<text>", "type": "latex|mathml|asciimath", "locale": "en|fr"}
//   Response: {"id": <int>, "speech": "<spoken text>"}  or  {"id": <int>, "error": "<msg>"}
//
// LaTeX and ASCII inputs are converted to MathML via temml before being passed
// to SRE, which only accepts MathML.

'use strict';

const sre = require('speech-rule-engine');
const temml = require('temml');
const readline = require('readline');

function emit(obj) {
  process.stdout.write(JSON.stringify(obj) + '\n');
}

let lastLocale = null;
let lastDomain = null;

const DEFAULT_DOMAIN = process.env.MATH_SPEAK_SRE_DOMAIN || 'clearspeak';

async function configure(locale, domain) {
  const target = (locale === 'fr') ? 'fr' : 'en';
  const dom = domain || DEFAULT_DOMAIN;
  if (target === lastLocale && dom === lastDomain) return;
  await sre.setupEngine({
    locale: target,
    domain: dom,
    modality: 'speech',
    style: 'default',
    markup: 'none',
  });
  lastLocale = target;
  lastDomain = dom;
}

// Lightweight ASCII-math to LaTeX rewrite. Handles the common case shapes:
//   sqrt(x)        → \sqrt{x}
//   sum_{...}^{...}, int_{...}^{...}, prod_{...}^{...}, lim_{...}
// Leaves x^2, x_i, +, -, =, parens unchanged (LaTeX accepts those).
function asciiToLatex(s) {
  let out = s;
  out = out.replace(/sqrt\(([^()]*)\)/g, '\\sqrt{$1}');
  out = out.replace(/\bsum_/g, '\\sum_');
  out = out.replace(/\bint_/g, '\\int_');
  out = out.replace(/\bprod_/g, '\\prod_');
  out = out.replace(/\blim_/g, '\\lim_');
  return out;
}

function latexToMathML(latex) {
  return temml.renderToString(latex, { throwOnError: false, displayMode: false });
}

function processInput(input, kind) {
  let mml;
  if (kind === 'latex') {
    mml = latexToMathML(input);
  } else if (kind === 'asciimath') {
    mml = latexToMathML(asciiToLatex(input));
  } else {
    // assume already MathML
    mml = input;
  }
  return sre.toSpeech(mml);
}

async function main() {
  await sre.setupEngine({ locale: 'en', domain: DEFAULT_DOMAIN, modality: 'speech', markup: 'none' });
  lastLocale = 'en';
  lastDomain = DEFAULT_DOMAIN;
  emit({ status: 'ready' });

  const rl = readline.createInterface({ input: process.stdin });
  for await (const line of rl) {
    if (!line.trim()) continue;
    let req;
    try {
      req = JSON.parse(line);
    } catch (e) {
      emit({ error: 'bad-json: ' + e.message });
      continue;
    }
    const id = req.id;
    try {
      await configure(req.locale, req.domain);
      const speech = processInput(req.input, req.type);
      emit({ id, speech: (speech || '').trim() });
    } catch (e) {
      emit({ id, error: String((e && e.message) || e) });
    }
  }
}

main().catch((e) => {
  emit({ status: 'fatal', error: String((e && e.message) || e) });
  process.exit(1);
});
