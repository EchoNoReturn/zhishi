// MathJax → SVG / PNG。stdin: TeX 数学串（不含 $$）。
// argv: --out <path> --format <svg|png> --font-size <n> --margin <n> [--transparent]
const fs = require('fs');

function arg(name, def) {
  const i = process.argv.indexOf('--' + name);
  return i >= 0 && process.argv[i + 1] ? process.argv[i + 1] : def;
}

function scaleLen(val, s) {
  const m = /^([0-9.]+)([a-z%]*)$/i.exec(String(val).trim());
  if (!m) return val;
  return String(parseFloat(m[1]) * s) + (m[2] || '');
}

async function main() {
  const out = arg('out', '');
  const format = arg('format', 'png');
  const fontSize = parseFloat(arg('font-size', '16'));
  const margin = parseFloat(arg('margin', '8'));
  const transparent = process.argv.includes('--transparent');

  if (!out) {
    console.error('missing --out');
    process.exit(2);
  }

  let tex = '';
  process.stdin.setEncoding('utf8');
  for await (const chunk of process.stdin) tex += chunk;
  tex = tex.trim();
  if (!tex) {
    console.error('empty tex on stdin');
    process.exit(2);
  }

  const { mathjax } = require('mathjax-full/js/mathjax.js');
  const { TeX } = require('mathjax-full/js/input/tex.js');
  const { SVG } = require('mathjax-full/js/output/svg.js');
  const { liteAdaptor } = require('mathjax-full/js/adaptors/liteAdaptor.js');
  const { RegisterHTMLHandler } = require('mathjax-full/js/handlers/html.js');
  const { AllPackages } = require('mathjax-full/js/input/tex/AllPackages.js');

  const adaptor = liteAdaptor();
  RegisterHTMLHandler(adaptor);

  const texIn = new TeX({ packages: AllPackages });
  const svgOut = new SVG({ fontCache: 'local' });
  const doc = mathjax.document('', { InputJax: texIn, OutputJax: svgOut });
  const node = doc.convert(tex, { display: true });

  let svg = adaptor.innerHTML(node);
  if (!svg.trim().startsWith('<svg')) {
    const wrapped = adaptor.outerHTML(node);
    const m = wrapped.match(/<svg[\s\S]*<\/svg>/);
    if (m) svg = m[0];
  }
  if (svg.startsWith('<svg') && !svg.includes('xmlns=')) {
    svg = svg.replace(
      '<svg',
      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"',
    );
  }

  // 从 viewBox 取几何
  const vbM = /viewBox="([^"]+)"/.exec(svg);
  let vb = [0, 0, 200, 50];
  if (vbM) vb = vbM[1].split(/[\s,]+/).map(Number);
  const [vx, vy, vw, vh] = vb;

  // MathJax SVG 用户单位：1 em = 1000 units
  const emUnits = 1000;
  const scale = fontSize / emUnits; // px per unit
  const w = vw * scale + margin * 2;
  const h = vh * scale + margin * 2;
  const padUnits = margin / scale;

  svg = svg.replace(/<svg[^>]*>/, () => {
    return (
      `<svg xmlns="http://www.w3.org/2000/svg" ` +
      `xmlns:xlink="http://www.w3.org/1999/xlink" ` +
      `width="${w}" height="${h}" ` +
      `viewBox="${vx - padUnits} ${vy - padUnits} ${vw + padUnits * 2} ${vh + padUnits * 2}">`
    );
  });

  if (format === 'svg') {
    fs.writeFileSync(out, svg, 'utf8');
    process.exit(0);
  }

  const sharp = require('sharp');
  // 按 2x 像素密度出图，避免解析单位歧义
  const density = 192;
  await sharp(Buffer.from(svg), { density }).png({ adaptiveFiltering: true }).toFile(out);
}

main().catch((e) => {
  console.error(String(e && e.stack ? e.stack : e));
  process.exit(1);
});
