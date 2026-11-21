export const rooms = {
  workshop: { title: '信号工坊', tint: '#dcb17b' },
  glasshouse: { title: '玻璃温室', tint: '#a9c0a2' },
};

export const people = {
  mina: {
    name: '米娜', role: '无线电技师', activity: '调试电台', room: 'workshop',
    glyph: 'work', color: '#ba694e', plate: '#633b32', x: 405, y: 313, motion: 'tune',
    history: [
      ['you', '19:34', '西塔又收到了双重回声。你能听出来是哪一段吗？'],
      ['them', '19:37', '复听了第三段，两次回声隔了半拍。备用天线是干净的，我已经切过去了。'],
      ['them', '19:41', '我录了 12 秒对照。今晚先用备用线，主线留到天亮查接头。'],
    ],
    reply: '我把这条记进今晚的波段记录。备用天线还在线，下一轮监听我也会一起核对。',
  },
  rowan: {
    name: '劳恩', role: '站内信使', activity: '派送便签', room: 'workshop',
    glyph: 'walk', color: '#c6a36f', plate: '#545139', x: 658, y: 379, motion: 'walk',
    history: [
      ['you', '19:29', '夜班交接单送到温室了吗？'],
      ['them', '19:32', '蓝色那份交给七禾了。西塔还有一份，我正在等米娜写完频率备注。'],
      ['them', '19:38', '第二轮经过投递柜时还空着。要是有新便签，我可以一起带走。'],
    ],
    reply: '收到。我把这条放在今晚的投递簿，下一趟经过温室和西塔时会核对。',
  },
  lota: {
    name: '洛塔', role: '线缆技师', activity: '修补线缆', room: 'workshop',
    glyph: 'work', color: '#91a4b3', plate: '#374f5c', x: 526, y: 466, motion: 'mend',
    history: [
      ['you', '19:17', '南侧进线束还剩几根没修？'],
      ['them', '19:21', '两根。一根接头松了，另一根的外皮磨得薄，我把它们分开放在红线轴边。'],
      ['them', '19:33', '松接头的已经换好。等补完外皮，我再把两根一起测一遍。'],
    ],
    reply: '我把这条别在线轴的检修单上，复测时一起核对。',
  },
  oak: {
    name: '阿栎', role: '温室园丁', activity: '照料幼苗', room: 'glasshouse',
    glyph: 'work', color: '#8aac97', plate: '#355449', x: 425, y: 340, motion: 'tend',
    history: [
      ['you', '19:23', '靠北窗的苗今晚需要多浇一点吗？'],
      ['them', '19:27', '先别浇北窗。叶子垂下来是因为玻璃冷，移到中间一排就会好些。'],
      ['them', '19:39', '我已经给新位置插了竹签，明早看土面是不是还湿。'],
    ],
    reply: '我把这条记到北窗那床的苗牌上。今夜先观察，明早再决定要不要补水。',
  },
  qi: {
    name: '七禾', role: '夜班记录员', activity: '茶歇', room: 'glasshouse',
    glyph: 'rest', color: '#afa0b2', plate: '#55455e', x: 636, y: 403, motion: 'tea',
    history: [
      ['you', '19:26', '交接单上的 04 号是谁补签的？'],
      ['them', '19:30', '是我。原件在温室长凳这边，红线框里是今晚改过的值班时间。'],
      ['them', '19:40', '我在这儿喝杯茶，等会儿就把本子带回去。'],
    ],
    reply: '收到了，我先记在交接单的边栏，回值班台时会和原件一起核对。',
  },
};
