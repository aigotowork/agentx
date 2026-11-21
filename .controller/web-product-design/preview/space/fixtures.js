// Presentation data only. Exact immutable keys; all events/timestamps are synthetic.
export const FIXTURE = 'space-v1';
export const SCOPE = '/demo/agentx';
export const GENERATION = '841';
export const offices = [
  { id: '/projects/atlas', short: 'Atlas', title: 'Atlas 产品组', code: '01', x: 28, y: 20 },
  { id: '/projects/atlas/web', short: 'Atlas / Web', title: 'Web 制作组', code: '02', x: 71, y: 20 },
  { id: '/projects/studio', short: 'Studio', title: 'Studio 试验室', code: '03', x: 78, y: 71 },
  { id: ':global', short: '全局', title: '共享休息区', code: 'G', x: 22, y: 71 },
  { id: ':unknown', short: '归属待确认', title: '归属待确认', code: '?', x: null }
];
const roster = [
  ['mora','104','','/projects/atlas/web','前端工程','front','waiting','接口已经对齐。你可以先帮我确认一下这两条空状态的动线。','10:42',2,66,37,'fern'],
  ['luna','127','','/projects/atlas/web','交互设计','design','working','列表的第二版方案已经整理好，我把需要取舍的地方标出来了。','10:36',1,79,37,'lilac'],
  ['orbi','192','','/projects/atlas','产品协调','lead','working','交付清单补上了复核项。窄屏这条可以单独验。','10:14',0,21.5,37,'rust'],
  ['sora','188','','/projects/atlas','质量验证','review','waiting','可以给我一个带历史实例的测试入口吗？','09:58',0,34,37,'peach'],
  ['vega','191','','/projects/atlas/web','前端工程','front','offline','我先停在导航状态这块，余下复核记在交接里。','09:27',0,79,50.5,'blue'],
  ['nami','124','','/projects/atlas/web','前端工程','front','working','浏览器后退可以回到原来的记录了。','09:15',0,66,50.5,'gold'],
  ['taro','208','','/projects/atlas','技术写作','writer','waiting','刚更新了路径表，可以发给评审。','09:03',0,21.5,50.5,'blue'],
  ['nova','211','',':global','工作台协助','helper','waiting','如果找不到归属，我可以帮忙确认。','昨天',0,22,79,'lilac'],
  ['kiri','219','',':unknown','—','','unknown','尚无可读的会话摘要。','—',0,null,null,'gold'],
  ['mora','97','632','/projects/atlas/web','前端工程','front','stopped','归档：首次整理收件状态与消息计数。','11/14',0,null,null,'fern'],
  ['iris','116','619','/projects/atlas','质量验证','review','stopped','归档：第一轮 Office 视图走查。','11/13',0,null,null,'peach']
];
export const keyOf = ({ name, inc, stop }) => JSON.stringify([SCOPE, GENERATION, name, inc, stop, stop ? 'stopped' : 'current']);
export const agents = roster.map(([name,inc,stop,office,role,post,status,preview,time,unread,x,y,tone]) => {
  const agent = {name,inc,stop,office,role,post,status,preview,time,unread,x,y,tone};
  return {...agent,key:keyOf(agent)};
});
export const statuses = {working:['工作中','success'],waiting:['等待消息','warning'],offline:['未运行','neutral'],unknown:['状态未知','neutral'],stopped:['历史实例','neutral']};
export const posts = [
  {id:'front',office:'/projects/atlas/web', title:'前端工程', path:'roles/frontend.md',desc:'负责 Web 工作区：导航、会话与嵌入窗口中的交互实现。'},
  {id:'design',office:'/projects/atlas/web', title:'交互设计',path:'roles/design.md',desc:'把任务路径与组件反馈整理成交互稿。'},
  {id:'lead',office:'/projects/atlas', title:'产品协调',path:'roles/lead.md',desc:'梳理交付范围与各角色的对接。'},
  {id:'review',office:'/projects/atlas', title:'质量验证',path:'roles/review.md',desc:'用实际路径确认关键状态与返回行为。'},
  {id:'writer',office:'/projects/atlas', title:'技术写作',path:'roles/writer.md',desc:'维护交付的使用说明。'},
  {id:'research',office:'/projects/studio', title:'探索研究',path:'roles/research.md',desc:'组织试验；当前没有任职成员。'}
];
const envelope = (id, who, time, text, extra={}) => ({id,who,time,text,...extra});
export const recent = [
  envelope('a31','mora','10:12','我把 Web 制作组的交接拆成两部分：会话入口、以及发出消息以后的状态。今天先确定入口和阅读顺序。'),
  envelope('a32','你','10:18','先从当前实例打开，历史实例仍能被搜索到。窄屏从会话返回时，请把焦点放回原来的那一行。'),
  envelope('a33','mora','10:29','收到。我保留了原来两个入口的身份校验。发送结果不确定时，草稿会留在该会话，等回执核对完再继续。',{reply:'a32',thread:'会话导航'}),
  envelope('a34','mora','10:42','接口已经对齐。你可以先帮我确认一下这两条空状态的动线。',{file:'Web 空状态 · 评审摘录.txt'})
];
export const older = [
  envelope('a24','mora','昨天 16:10','办公室里角色重复很正常，座位上的名字和列表需要指向同一批身份。'),
  envelope('a25','你','昨天 16:16','好的。历史记录不会出现在地图座位上，也不要退回到同名当前实例。'),
  envelope('a26','mora','昨天 16:38','我会把这条放进评审走查。附件：测试场景目录。',{file:'测试场景 · 导航.txt'})
];
export const records = [
  {id:'r01',role:'用户输入',time:'09:49',text:'核对窄屏会话路径：从 Web 办公室发现实例，读取最近消息并返回。'},
  {id:'r02',role:'模型输出',time:'09:50',text:'整理了中断后的恢复路径：保留搜索、定位原来入口、继续对应实例的草稿。',tool:'read_file · design/space-flows.md\nResult: 3 navigation scenarios',reason:'先核对引用的实例，再用窄屏布局验证返回点。'}
];
export const mail = [
  {id:'h04',from:'luna',inc:'127',time:'10:46',text:'窄屏的交互稿有两处需要你选择：返回时是否保留筛选、以及发送反馈的位置。',read:false},
  {id:'h03',from:'orbi',inc:'192',time:'10:10',text:'今天的交付清单已经补齐。方便时看一下无成员工作区。',read:false},
  {id:'h02',from:'mora',inc:'104',time:'09:35',text:'复核材料已放在当前会话。',read:true},
  {id:'h01',from:'sora',inc:'188',time:'昨天',text:'上一版走查已留档。',read:true}
];
