/** Load Vite with the binding supported by the host's filesystem. */
export function loadVite() {
  // Rolldown WASI cannot resolve Windows drive paths. On other hosts keep it
  // mandatory: the published Linux x64 native initializer requires BMI2.
  if (process.platform === 'win32') {
    delete process.env.NAPI_RS_FORCE_WASI;
    delete process.env.NAPI_RS_WASI_FLAVOR;
  } else {
    process.env.NAPI_RS_FORCE_WASI = 'error';
    process.env.NAPI_RS_WASI_FLAVOR = 'wasm32-wasi';
  }
  process.env.NAPI_RS_ENFORCE_VERSION_CHECK = '1';
  return import('vite');
}
