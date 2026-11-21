/** Configure only the iframe service origin; native discovery and errors belong to Space. */
export function serviceOrigin(value) {
  const url = new URL(value);
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash || !['', '/'].includes(url.pathname)) throw new Error('请输入纯 HTTP(S) 服务地址（IP 和端口），不要附加路径或凭据');
  return url.origin;
}
