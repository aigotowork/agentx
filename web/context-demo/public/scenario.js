/** Illustrative order facts, independent of live AgentX discovery. */
export const orders = [
  { id: 'AX-20261115-0182', customer: '林女士', city: '杭州 · 西湖区', status: '需跟进', issue: '分包未齐 · 磨豆机外壳凹陷', due: '今日 16:00 前回复', amount: '¥1,268', contact: '客户希望周末前收到可用设备；暂不接受整单退货。', items: [['手冲咖啡套装', '1 套', '¥469', '已签收'], ['电动磨豆机 G2', '1 台', '¥799', '待核实损坏']], events: [['11/15 10:42', '客户反馈', '磨豆机右侧外壳凹陷，开机有异响；滤纸分包尚未收到。'], ['11/14 18:26', '主包签收', '收件柜签收。外箱照片显示右下角挤压，尚缺开箱视频。'], ['11/13 15:10', '分包发出', '滤纸由合作仓单独发货，物流最近更新为「杭州分拨中心」。'], ['11/12 09:18', '付款完成', '实付 ¥1,268；支付渠道：银行卡。']], policy: '签收 7 日内的运输损坏可优先换新，需补充商品损坏照片与外箱面单；分包未齐先查物流，不重复补发。', question: '请整理此订单的异常、已知事实与缺失证据，按优先级列出下一步。' },
  { id: 'AX-20261114-0097', customer: '周先生', city: '苏州 · 工业园区', status: '待查物流', issue: '配送延迟 · 礼品时效', due: '今日 18:00 前更新', amount: '¥328', contact: '11 月 17 日生日礼物，需要确认是否可按时送达。', items: [['随行保温杯礼盒', '2 盒', '¥328', '运输中']], events: [['11/15 08:20', '客户咨询', '运输状态 36 小时未更新，要求提供明确到货时间。'], ['11/13 20:10', '中转扫描', '到达苏州中转场，暂无异常签收记录。'], ['11/12 13:05', '付款完成', '实付 ¥328；礼盒刻字已完成。']], policy: '物流超过 48 小时未更新可发起承运商查询；刻字商品非质量问题不适用无理由退货。不得未经核实承诺到货时间。', question: '请判断是否达到物流查询阈值，并拟定不作无依据承诺的客户沟通要点。' },
  { id: 'AX-20261113-0063', customer: '陈女士', city: '南京 · 鼓楼区', status: '待补凭证', issue: '发票抬头更正', due: '明日 12:00 前确认', amount: '¥589', contact: '需要将个人抬头改为公司抬头，尚未提供纳税人识别号。', items: [['桌面阅读灯 L1', '1 台', '¥589', '已签收']], events: [['11/15 09:30', '更正申请', '客户提供公司名称，但未提供税号与原发票号码。'], ['11/14 11:08', '商品签收', '本人签收，无商品质量反馈。'], ['11/13 10:12', '开票完成', '电子普通发票；抬头为个人。']], policy: '抬头更正需核对原发票号码、公司全称和税号，财务确认红冲后重开。不得收集与开票无关的个人信息。', question: '请列出更正发票所需的最少补充信息，并草拟一次性告知客户的清单。' },
];

export function orderContext(order) {
  const items = order.items.map(row => row.join(' · ')).join('\n');
  const events = order.events.map(row => row.join(' · ')).join('\n');
  return `【固定演示订单】${order.id}\n客户：${order.customer}（${order.city}）\n状态：${order.status}；${order.due}\n实付：${order.amount}\n异常：${order.issue}\n客户诉求：${order.contact}\n商品：\n${items}\n记录：\n${events}\n售后规则：${order.policy}`;
}

export function renderOrderQueue(container, selectedId, onSelect) {
  container.replaceChildren(...orders.map(order => {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'order-row';
    button.dataset.order = order.id;
    button.setAttribute('aria-pressed', String(order.id === selectedId));
    button.innerHTML = `<span class="order-row-heading"><strong>${order.customer}</strong><span class="badge">${order.status}</span></span><span class="order-id">${order.id}</span><span>${order.issue}</span><small>${order.due}</small>`;
    button.addEventListener('click', () => onSelect(order));
    return button;
  }));
}

export function renderOrderDetail(order) {
  document.querySelector('#order-id').textContent = order.id;
  document.querySelector('#customer').textContent = `${order.customer} · ${order.city}`;
  document.querySelector('#order-issue').textContent = order.issue;
  document.querySelector('#order-due').textContent = order.due;
  document.querySelector('#customer-request').textContent = order.contact;
  document.querySelector('#order-amount').textContent = order.amount;
  document.querySelector('#order-policy').textContent = order.policy;
  document.querySelector('#order-items').innerHTML = order.items.map(row => `<tr>${row.map(value => `<td>${value}</td>`).join('')}</tr>`).join('');
  document.querySelector('#order-events').innerHTML = order.events.map(row => `<li><time>${row[0]}</time><div><strong>${row[1]}</strong><p>${row[2]}</p></div></li>`).join('');
}
