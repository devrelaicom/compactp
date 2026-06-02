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
