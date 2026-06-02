(() => {
  const clockEl = document.getElementById('clock');
  if (!clockEl) return;

  const TZ = 'America/Cayman';
  const fmt = new Intl.DateTimeFormat('en-GB', {
    timeZone: TZ,
    weekday: 'long',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });

  const renderClock = () => {
    const parts = fmt.formatToParts(new Date());
    const weekday = parts.find(p => p.type === 'weekday').value.toUpperCase();
    const hour    = parts.find(p => p.type === 'hour').value;
    const minute  = parts.find(p => p.type === 'minute').value;
    const second  = parts.find(p => p.type === 'second').value;
    clockEl.textContent = `${weekday} ${hour}:${minute}:${second} EST`;
  };

  renderClock();
  setInterval(renderClock, 1000);
})();

(() => {
  const wordmark = document.getElementById('wordmark');
  if (!wordmark) return;

  const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (prefersReduced) return;

  // Scramble alphabet: ASCII printable minus space and the cell's final glyph.
  const ALPHABET = '!#$%&*+-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ\\^_`abcdefghijklmnopqrstuvwxyz{|}~';

  // Split the wordmark text into cells, preserving original characters.
  // Render to an array of <span>s so we can mutate individual cells.
  const sourceText = wordmark.textContent;
  const lines = sourceText.split('\n');

  // Build a 2D array of cells. Track only non-whitespace cells as
  // "scrambleable" — leading/trailing whitespace per line is left
  // untouched to preserve the wordmark's offset.
  const cells = [];
  const fragment = document.createDocumentFragment();

  lines.forEach((line, lineIdx) => {
    if (lineIdx > 0) fragment.appendChild(document.createTextNode('\n'));
    for (let col = 0; col < line.length; col++) {
      const ch = line[col];
      if (ch === ' ') {
        fragment.appendChild(document.createTextNode(' '));
      } else {
        const span = document.createElement('span');
        span.textContent = ch;
        fragment.appendChild(span);
        cells.push({ line: lineIdx, col, final: ch, span });
      }
    }
  });

  wordmark.textContent = '';
  wordmark.appendChild(fragment);

  // Pick ~40% of cells at random for scrambling.
  const SCRAMBLE_RATIO = 0.4;
  const scramblers = cells.filter(() => Math.random() < SCRAMBLE_RATIO);

  const randomGlyph = (avoid) => {
    let g;
    do { g = ALPHABET[Math.floor(Math.random() * ALPHABET.length)]; }
    while (g === avoid);
    return g;
  };

  // Each scrambler runs 3 swaps over ~600ms (200ms apart) then settles.
  scramblers.forEach((cell) => {
    let frame = 0;
    const totalFrames = 3 + Math.floor(Math.random() * 2);
    const interval = 600 / totalFrames;
    const tick = () => {
      if (frame < totalFrames) {
        cell.span.textContent = randomGlyph(cell.final);
        frame++;
        setTimeout(tick, interval + (Math.random() * 40 - 20));
      } else {
        cell.span.textContent = cell.final;
      }
    };
    setTimeout(tick, Math.random() * 80);
  });

  // Expose the cells array on the wordmark element so Task 8's hover
  // handler can reuse the same DOM split rather than re-parsing.
  wordmark._cells = cells;
})();
