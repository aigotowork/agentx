(() => {
  const root = document.querySelector('[data-app]');
  if (!root) return;
  let lastFocus = null;
  let mobileFocus = null;
  let instanceCount = 0;
  let toastTimer;
  const emit = (event, description) => {
    root.querySelector('[data-event-output]')?.replaceChildren(document.createTextNode(`${event} · ${description}`));
  };
  const toast = (message) => {
    const el = root.querySelector('[data-toast]');
    el.textContent = message;
    el.classList.add('is-visible');
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => el.classList.remove('is-visible'), 1800);
  };
  const closeLayers = () => {
    root.querySelectorAll('[data-library], [data-sheet], [data-sheet-backdrop]').forEach(el => el.classList.remove('is-open'));
    lastFocus?.focus();
  };
  const closeInspector = () => {
    root.querySelector('[data-inspector]')?.classList.remove('is-mobile-open');
    mobileFocus?.focus();
  };
  const select = (element) => {
    const subject = element.dataset.select;
    root.querySelectorAll('[data-select]').forEach(el => {
      const active = el.dataset.select === subject;
      el.classList.toggle('is-selected', active);
      if (el.getAttribute('role') === 'button' && !el.classList.contains('layer-item')) el.classList.toggle('selection-outline', active);
    });
    root.querySelectorAll('[data-selected-name]').forEach(el => el.textContent = subject);
    root.querySelectorAll('[data-selected-path]').forEach(el => el.textContent = element.dataset.path || '当前组合 / 新实例');
    const geometry = {
      'Inspector 看板': ['1280','720','Sidebar + Main + Aside'],
      '活动表格': ['自适应','160','Table / 3 rows'],
      '指标卡组': ['自适应','80','Grid / 3 columns'],
      '执行趋势': ['自适应','136','Chart / 24h'],
      '颜色样本': ['4 色','22','Palette / semantic'],
      '文字层级': ['自适应','48','Type / 3 levels'],
      'Layout 槽位': ['自适应','56','main + footer + aside']
    }[subject] || ['自动','自动','组件实例'];
    ['width','height','layout'].forEach((key,index) => root.querySelector(`[data-field-${key}]`)?.replaceChildren(document.createTextNode(geometry[index])));
    emit('selection.change', subject);
  };
  root.querySelectorAll('[data-inspector-tab]').forEach(el => el.setAttribute('aria-selected', String(el.classList.contains('is-active'))));
  const initialSelection = root.querySelector('.layer-item.is-selected');
  if (initialSelection) select(initialSelection);
  root.addEventListener('click', event => {
    const button = event.target.closest('button, [role="button"]');
    if (!button || !root.contains(button)) return;
    if (button.dataset.nav) {
      root.querySelectorAll('[data-nav]').forEach(el => el.classList.toggle('is-active', el === button));
      if (button.dataset.nav === '资源') {
        lastFocus = button;
        root.querySelector('[data-library]')?.classList.add('is-open');
      }
      if (button.dataset.nav === '场景') {
        select(root.querySelector('[data-select="活动表格"]'));
        toast('交互场景：选择表格并查看事件');
      }
      if (button.dataset.nav === '设置') {
        lastFocus = button;
        root.querySelector('[data-sheet-backdrop]')?.classList.add('is-open');
        root.querySelector('[data-sheet]')?.classList.add('is-open');
      }
      emit('navigation.change', button.dataset.nav);
    }
    if (button.dataset.select) select(button);
    if (button.dataset.inspectorTab) {
      root.querySelectorAll('[data-inspector-tab]').forEach(el => {
        el.classList.toggle('is-active', el === button);
        el.setAttribute('aria-selected', String(el === button));
      });
      root.querySelectorAll('[data-inspector-panel]').forEach(el => el.classList.toggle('is-active', el.dataset.inspectorPanel === button.dataset.inspectorTab));
    }
    if (button.hasAttribute('data-open-library')) {
      lastFocus = button;
      root.querySelector('[data-library]')?.classList.toggle('is-open');
      root.querySelector('[data-library-search]')?.focus();
    }
    if (button.dataset.addResource) {
      const resource = button.dataset.addResource;
      root.querySelector('[data-library]')?.classList.remove('is-open');
      const element = document.createElement('button');
      element.type = 'button';
      element.className = 'added-instance';
      element.dataset.select = `${resource} ${++instanceCount}`;
      element.dataset.dcId = `added-instance-${instanceCount}`;
      element.textContent = `${resource} · 实例 ${instanceCount}`;
      root.querySelector('[data-added-resource]')?.append(element);
      select(element);
      element.scrollIntoView({block:'nearest'});
      element.focus();
      emit('resource.add', `${resource} ${instanceCount}`);
      toast(`已加入画布：${resource}`);
    }
    if (button.hasAttribute('data-open-doc')) {
      lastFocus = button;
      root.querySelector('[data-sheet]')?.classList.add('is-open');
      root.querySelector('[data-sheet-backdrop]')?.classList.add('is-open');
      root.querySelector('[data-sheet-close]')?.focus();
    }
    if (button.hasAttribute('data-close-layer')) closeLayers();
    if (button.hasAttribute('data-layout-toggle')) {
      const on = root.querySelector('[data-layout-canvas]')?.classList.toggle('show-slots');
      button.setAttribute('aria-pressed', String(Boolean(on)));
      button.querySelector('[data-toggle-label]')?.replaceChildren(document.createTextNode(on ? '隐藏槽位' : '显示槽位'));
      emit('layout.toggle', on ? '编辑槽位可见' : '运行预览');
    }
    if (button.hasAttribute('data-mode')) {
      root.querySelectorAll('[data-mode]').forEach(el => el.classList.toggle('is-active', el === button));
      root.querySelector('[data-layout-canvas]')?.classList.toggle('show-slots', button.dataset.mode === 'edit');
      emit('layout.toggle', button.dataset.mode === 'edit' ? '编辑预览' : '运行预览');
    }
    if (button.hasAttribute('data-mobile-inspector')) {
      mobileFocus = button;
      root.querySelector('[data-inspector]')?.classList.add('is-mobile-open');
      root.querySelector('[data-mobile-back]')?.focus();
    }
    if (button.hasAttribute('data-mobile-back')) closeInspector();
    if (button.hasAttribute('data-collapse-layers')) root.classList.toggle('is-layers-collapsed');
    if (button.hasAttribute('data-focus-local')) {
      const focused = root.classList.toggle('is-focused');
      button.querySelector('[data-focus-label]').textContent = focused ? '返回整体' : '聚焦';
      emit('canvas.focus', focused ? '局部' : '整体');
    }
    if (button.hasAttribute('data-reset-canvas')) {
      root.querySelector('[data-added-resource]')?.replaceChildren();
      instanceCount = 0;
      root.classList.remove('is-focused');
      const first = root.querySelector('.layer-item');
      if (first) select(first);
      emit('canvas.reset', '已恢复初始组合');
    }
    if (button.dataset.sampleAction) {
      toast(button.dataset.sampleAction);
      emit('sample.activate', button.textContent);
    }
  });
  root.addEventListener('input', event => {
    if (!event.target.matches('[data-layer-search], [data-library-search]')) return;
    const query = event.target.value.trim().toLowerCase();
    const selector = event.target.matches('[data-library-search]') ? '.library-item' : '.layer-item';
    root.querySelectorAll(selector).forEach(item => item.hidden = Boolean(query && !item.textContent.toLowerCase().includes(query)));
  });
  root.addEventListener('click', event => {
    if (event.target.matches('[data-sheet-backdrop]')) closeLayers();
  });
  const tooltip = document.createElement('div');
  tooltip.id = 'workbench-tooltip';
  tooltip.className = 'shared-tooltip';
  tooltip.setAttribute('role', 'tooltip');
  tooltip.hidden = true;
  root.append(tooltip);
  let tipOwner;
  let tipTimer;
  const hideTip = () => { tooltip.hidden = true; tipOwner?.removeAttribute('aria-describedby'); };
  const showTip = (element) => {
    if (!element?.matches('.icon-btn[aria-label], .rail-btn[aria-label]')) return;
    hideTip();
    tipOwner = element;
    element.removeAttribute('title');
    tooltip.textContent = element.getAttribute('aria-label');
    tooltip.hidden = false;
    const rect = element.getBoundingClientRect();
    const box = tooltip.getBoundingClientRect();
    tooltip.style.setProperty('--tooltip-x', `${Math.max(8, Math.min(innerWidth - box.width - 8, rect.left + rect.width / 2 - box.width / 2))}px`);
    tooltip.style.setProperty('--tooltip-y', `${Math.min(innerHeight - box.height - 8, rect.bottom + 6)}px`);
    element.setAttribute('aria-describedby', tooltip.id);
  };
  root.addEventListener('focusin', event => showTip(event.target));
  root.addEventListener('focusout', hideTip);
  root.addEventListener('pointerover', event => { if(event.pointerType !== 'touch') showTip(event.target.closest('button')); });
  root.addEventListener('pointerout', event => { if (event.target.closest('button') === tipOwner && document.activeElement !== tipOwner) hideTip(); });
  root.addEventListener('pointerdown', event => {
    if (event.pointerType !== 'touch') return;
    showTip(event.target.closest('button'));
    clearTimeout(tipTimer);
    tipTimer = setTimeout(hideTip, 1500);
  });
  document.addEventListener('keydown', event => {
    if ((event.key === 'Enter' || event.key === ' ') && event.target.matches('[role="button"]')) {
      event.preventDefault();
      event.target.click();
    }
    if (event.key === 'Escape') {
      hideTip();
      if (root.querySelector('[data-sheet].is-open, [data-library].is-open')) closeLayers();
      else if (root.querySelector('[data-inspector].is-mobile-open')) closeInspector();
      else if (root.classList.contains('is-focused')) root.querySelector('[data-focus-local]')?.click();
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      lastFocus = document.activeElement;
      root.querySelector('[data-library]')?.classList.add('is-open');
      root.querySelector('[data-library-search]')?.focus();
    }
    const dialog = root.querySelector('[data-sheet].is-open');
    if (event.key === 'Tab' && dialog) {
      const focusables = [...dialog.querySelectorAll('button, input, [tabindex="0"]')];
      const first = focusables[0], last = focusables[focusables.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    }
  });
})();
