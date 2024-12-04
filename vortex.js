import init, { File } from './pkg/vortex_wasm';

let loaded = false;

async function loadInner(module_or_path) {
  await init(module_or_path);

  loaded = true;
}

export async function load(module_or_path) {
  if (!loaded) {
    await loadInner(module_or_path);
  }
}

export default {
  load: load,
  File: File,
};
