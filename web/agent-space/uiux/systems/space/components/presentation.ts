/** Registry shared by the Space selector, gallery and Design Canvas scene binding. */
const SURFACE_STYLES = [
  { value: "classic", label: "原版像素办公室", note: "像素办公室与原版阅读界面", colors: ["#faf8ed", "#6b827b", "#e1bf7b"] },
  { value: "folio", label: "工坊信笺", note: "纸纤维、装订边和印章", colors: ["#fcf6e7", "#3d624d", "#874b33"] },
  { value: "signal", label: "夜班联络台", note: "冷色仪表、深蓝面板和信号绿", colors: ["#152432", "#83e2cb", "#ffc7b9"] },
] as const;
const CATALOG_STUDIO = { value: "studio", label: "图录工作室", note: "手绘木刻、格窗、暖木地板和工坊人物", colors: ["#fbf4db", "#526c57", "#a96140"] } as const;
const STUDIO_WORLDS = [
  { value: "lunar", label: "月面站", note: "钴蓝月面舱、橙色气闸和机械角色", colors: ["#152746", "#ffae67", "#a9d4fc"] },
  { value: "terrarium", label: "苔庭", note: "剪纸温室、蕨叶和植物角色", colors: ["#fff7df", "#346650", "#ba7653"] },
  { value: "newsprint", label: "漫画社", note: "粗墨线、红蓝撞色和网点", colors: ["#fff3d6", "#a73124", "#1f5193"] },
] as const;
const GAME_STUDIES = [
  { value: "inkpost", label: "烛影邮局", note: "手绘暗黑童话、歪木和暖灯", colors: ["#2a2026", "#ffc077", "#a27260"] },
  { value: "windward", label: "风屿驿站", note: "云海、青铜观测塔和高地冒险", colors: ["#fff6dc", "#23605c", "#ecbe84"] },
  { value: "jadecourt", label: "流光庭院", note: "奇幻园林、琉璃和红铜", colors: ["#1e293e", "#9ae5cb", "#eed0a0"] },
] as const;
export const SPACE_LIBRARY_WORLDS = [
  { value: "citrus", label: "柚风小筑", note: "清新 · 海边日光、柚黄、亚麻和白石", colors: ["#f7fbef", "#237068", "#efc65a"] },
  { value: "reverie", label: "星绒梦室", note: "梦幻 · 星纱、珍珠光、长夜和云层", colors: ["#292945", "#f5c6dc", "#b6b6f4"] },
  { value: "wash", label: "雨后水彩", note: "水彩 · 纸纹、颜料晕边和雨后蓝绿", colors: ["#f9f3e6", "#405b75", "#c59067"] },
  { value: "collage", label: "彩纸事务所", note: "几何拼贴 · 原色剪纸、折边和错落投影", colors: ["#fcf1da", "#2453a3", "#e85030"] },
] as const;

export const SPACE_STYLE_GROUPS = [
  { id: "surface", label: "经典界面", styles: SURFACE_STYLES },
  { id: "studio", label: "场景主题", styles: [CATALOG_STUDIO, ...STUDIO_WORLDS] },
  { id: "game", label: "游戏世界", styles: GAME_STUDIES },
  { id: "library", label: "插画风格", styles: SPACE_LIBRARY_WORLDS },
] as const;

export const SPACE_GAME_WORLDS = [...GAME_STUDIES, ...SPACE_LIBRARY_WORLDS] as const;
export const SPACE_WORLDS = [...STUDIO_WORLDS, ...SPACE_GAME_WORLDS] as const;
type SpaceStyleEntry = (typeof SPACE_STYLE_GROUPS)[number]["styles"][number];
export const SPACE_STYLES = SPACE_STYLE_GROUPS.flatMap<SpaceStyleEntry>(group => group.styles);

export type SpaceStyle = SpaceStyleEntry["value"];
export type SpaceWorld = (typeof SPACE_WORLDS)[number]["value"];
export type SpaceGameWorld = (typeof SPACE_GAME_WORLDS)[number]["value"];

export function spaceGameWorld(world?: SpaceWorld): SpaceGameWorld | undefined {
  return SPACE_GAME_WORLDS.find(candidate => candidate.value === world)?.value;
}

export function spaceWorld(style: SpaceStyle): SpaceWorld | undefined {
  return SPACE_WORLDS.find(world => world.value === style)?.value;
}

export function usesStudioArt(style: SpaceStyle): boolean {
  return style === CATALOG_STUDIO.value || spaceWorld(style) !== undefined;
}

export function parseSpaceStyle(value: string | null): SpaceStyle | undefined {
  return SPACE_STYLES.find(style => style.value === value)?.value;
}
