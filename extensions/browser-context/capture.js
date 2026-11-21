/** Explicit active-tab capture, independent of conversation selection. No runtime File hop. */
const MAX_BYTES = 10 * 1024 * 1024;
export async function snapshot() {
  const [tab] = await chrome.tabs.query({ active:true, currentWindow:true });
  if (!tab?.id || !Number.isInteger(tab.windowId) || !/^https?:\/\//.test(tab.url || '')) throw new Error('此页面不能采集。打开普通 HTTP(S) 网页，再点击扩展图标授权当前标签页');
  return { id:tab.id, windowId:tab.windowId, url:tab.url, title:tab.title || '' };
}
export async function verifyTab(tab) {
  const current = await snapshot();
  if (current.id !== tab.id || current.windowId !== tab.windowId || current.url !== tab.url) throw new Error('采集期间标签页或地址已变化；本次内容已取消，请重新采集');
}
export async function capture(kind, tab) {
  if (kind === 'screenshot') {
    const data = await chrome.tabs.captureVisibleTab(tab.windowId, { format:'png' });
    if (!data.startsWith('data:image/png;base64,') || data.length > Math.ceil(MAX_BYTES * 4 / 3) + 64) throw new Error('截图超过 10 MiB，请缩小浏览器窗口后重试');
    await verifyTab(tab);
    const encoded = data.slice(data.indexOf(',') + 1);
    const binary = atob(encoded);
    const bytes = Uint8Array.from(binary, character => character.charCodeAt(0));
    const blob = new Blob([bytes], {type:'image/png'});
    if (blob.size > MAX_BYTES) throw new Error('截图超过 10 MiB');
    if (tab.url.length > 4096) throw new Error('页面 URL 超过 4096 字符');
    return { title:tab.title.slice(0,512), url:tab.url, files:[new File([blob], 'page-' + new Date().toISOString().replace(/[:.]/g,'-') + '.png', {type:'image/png'})] };
  }
  const result = await chrome.scripting.executeScript({ target:{tabId:tab.id}, func:(mode) => ({ title:document.title.slice(0,512), url:location.href, text:(mode === 'selection' ? String(window.getSelection() || '') : document.body?.innerText || '').slice(0,24000) }), args:[kind] });
  await verifyTab(tab);
  const context = result[0]?.result;
  if (!context || context.url !== tab.url || !context.text.trim()) throw new Error(kind === 'selection' ? '未选中文字；请在网页中选择后再试' : '当前网页没有可读取正文');
  if (context.url.length > 4096) throw new Error('页面 URL 超过 4096 字符');
  return context;
}
