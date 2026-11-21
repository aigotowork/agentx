(() => {
  'use strict';
  const gallery = document.querySelector('#studio-gallery');
  const boards = [
    {id:'frames',title:'01 阶梯框与按钮',note:'完整状态 · 图标微动'},
    {id:'tools',title:'02 锤子、碎屑与图标',note:'斜握下敲 · 可补回靶块'},
    {id:'signals',title:'03 状态节奏与图表',note:'精确值 · 缺失保持未知'},
    {id:'investigation',title:'04 恢复调查',note:'真实层级 · 离线场景'},
  ];
  let selected = boards.some(board => `#${board.id}` === location.hash) ? location.hash.slice(1) : 'overview';
  let userReduced = false;
  let sound = false;
  let theme = 'light';
  const systemMotion = matchMedia('(prefers-reduced-motion: reduce)');
  gallery.innerHTML = boards.map(board => `<article class="artboard-card" data-artboard="${board.id}" aria-label="${board.title}"><header class="artboard-header"><h2>${board.title}</h2><span class="note">${board.note}</span><a href="${board.id}.html" target="_blank" rel="noopener">单独打开</a><button type="button" class="pixel small" data-focus-board="${board.id}"><span class="face">放大操作</span></button></header><div class="artboard-stage"><iframe src="${board.id}.html?embedded=1" title="${board.title}交互画板" data-frame="${board.id}"></iframe></div></article>`).join('');
  const cards = [...gallery.querySelectorAll('.artboard-card')];
  const frames = [...gallery.querySelectorAll('iframe')];
  function preferences() {
    document.documentElement.dataset.theme = theme;
    document.querySelector('#studio-theme .face').textContent = theme === 'light' ? '切换深色' : '切换明亮';
    document.querySelector('#studio-motion').setAttribute('aria-pressed',String(userReduced || systemMotion.matches));
    document.querySelector('#studio-motion .face').textContent = userReduced || systemMotion.matches ? '动态：减少' : '动态：系统';
    document.querySelector('#studio-sound').setAttribute('aria-pressed',String(sound));
    document.querySelector('#studio-sound .face').textContent = sound ? '声音：开' : '声音：关';
    for (const frame of frames) {
      const doc = frame.contentDocument;
      if (!doc?.querySelector('#app .shell')) continue;
      doc.documentElement.dataset.panelHidden = String(selected !== 'overview' && frame.dataset.frame !== selected);
      doc.dispatchEvent(new frame.contentWindow.CustomEvent('inspector-preview-preferences',{detail:{theme,reduced:userReduced,sound}}));
    }
  }
  function layout() {
    const overview = selected === 'overview';
    gallery.dataset.view = overview ? 'overview' : 'focus';
    const narrow = innerWidth <= 900;
    for (const card of cards) {
      card.hidden = !overview && card.dataset.artboard !== selected;
      if (card.hidden) continue;
      const stage = card.querySelector('.artboard-stage');
      const width = stage.clientWidth;
      const available = Math.max(210,(innerHeight - gallery.getBoundingClientRect().top - 48) / 2 - 44);
      const scale = overview ? Math.min(width / 1280,narrow ? 1 : available / 720) : 1;
      const height = overview ? 720 * scale : Math.max(580,innerHeight - stage.getBoundingClientRect().top - 20);
      stage.style.setProperty('--frame-scale',String(scale));
      stage.style.setProperty('--frame-height',`${height}px`);
      stage.style.setProperty('--frame-width',`${overview ? 1280 : width}px`);
      stage.style.setProperty('--frame-native-height',`${overview ? 720 : height}px`);
      stage.style.setProperty('--frame-left',`${overview ? (width - 1280 * scale) / 2 : 0}px`);
      card.querySelector('[data-focus-board] .face').textContent = overview ? '放大操作' : '返回总览';
    }
  }
  function select(id) {
    selected = id;
    history.replaceState(null,'',id === 'overview' ? location.pathname : `#${id}`);
    document.querySelectorAll('[data-board-target]').forEach(button => button.setAttribute('aria-pressed',String(button.dataset.boardTarget === id)));
    document.querySelector('#studio-readout').textContent = id === 'overview' ? '四张画板均可放大操作，切换保留状态。' : '直接操作画板；切换保留输入、靶块和选择。';
    layout(); preferences();
  }
  document.querySelectorAll('[data-board-target]').forEach(button => button.addEventListener('click',() => select(button.dataset.boardTarget)));
  document.querySelectorAll('[data-focus-board]').forEach(button => button.addEventListener('click',() => select(selected === 'overview' ? button.dataset.focusBoard : 'overview')));
  frames.forEach(frame => frame.addEventListener('load',() => { preferences(); layout(); }));
  document.querySelector('#studio-theme').addEventListener('click',() => { theme = theme === 'light' ? 'dark' : 'light'; preferences(); });
  document.querySelector('#studio-motion').addEventListener('click',() => { userReduced = !userReduced; preferences(); });
  document.querySelector('#studio-sound').addEventListener('click',() => { sound = !sound; preferences(); });
  systemMotion.addEventListener('change',preferences);
  document.addEventListener('visibilitychange',() => { if (document.hidden) { sound = false; preferences(); } });
  new ResizeObserver(layout).observe(gallery);
  window.addEventListener('resize',layout);
  select(selected);
})();
