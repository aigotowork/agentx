// A standard extension action owns both activeTab invocation and the synchronous
// sidePanel.open gesture. Do not let automatic panel behavior consume the action.
chrome.sidePanel.setPanelBehavior({openPanelOnActionClick:false}).catch(console.error);
chrome.runtime.onInstalled.addListener(() => {
  chrome.sidePanel.setPanelBehavior({openPanelOnActionClick:false}).catch(console.error);
});
chrome.action.onClicked.addListener(tab => {
  if (typeof tab.windowId !== 'number') return;
  chrome.sidePanel.open({windowId:tab.windowId}).catch(console.error);
});
