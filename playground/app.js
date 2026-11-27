/* Azra playground: Monaco editor (with textarea fallback) + WASM interpreter. */

const EXAMPLES = {
  "Hello": 'note: your first Azra program\nsay "Hello from Azra!"\n',
  "Math & vectors": [
    'set a to vec3 1, 2, 3',
    'set b to vec3 4, 5, 6',
    'say "dot:", dot of a and b',
    'say "cross:", cross of a and b',
    'say "pi:", pi',
    'say "log2(1024):", log base 2 of 1024',
    'say "lerp:", lerp 3 to 7 by 0.25',
    'say "degrees:", radians to degrees of pi',
    ''
  ].join('\n'),
  "Loops & functions": [
    'define fib with n:',
    '    if n less_than 2:',
    '        give back n',
    '    set left to fib with n - 1',
    '    set right to fib with n - 2',
    '    give back left + right',
    '',
    'repeat with i from 1 to 10 by 2:',
    '    say "fib", i, "=", fib with i',
    ''
  ].join('\n'),
  "Tests": [
    'test "addition works":',
    '    assert equals 2 + 2, 4',
    '    assert "azra" contains "az"',
    '',
    'say "run `azra test main.vibe` locally to execute these"',
    ''
  ].join('\n'),
};

let wasm = null;
let editor = null;
const statusEl = document.getElementById('status');
const outputEl = document.getElementById('output');
const exitEl = document.getElementById('exit-status');
const codeEl = document.getElementById('code');
const selectEl = document.getElementById('example-select');

function source() {
  return editor ? editor.getValue() : codeEl.value;
}

function setSource(text) {
  if (editor) editor.setValue(text);
  else codeEl.value = text;
}

function encodeShare(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  bytes.forEach(b => { binary += String.fromCharCode(b); });
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function decodeShare(hash) {
  try {
    const base64 = hash.replace(/^#code=/, '').replace(/-/g, '+').replace(/_/g, '/');
    const binary = atob(base64);
    const bytes = Uint8Array.from(binary, c => c.charCodeAt(0));
    return new TextDecoder().decode(bytes);
  } catch (e) {
    return null;
  }
}

function setupEditor() {
  if (!window.require || !window.monaco) {
    codeEl.style.display = 'block';
    return;
  }
  window.require.config({ paths: { vs: 'https://cdn.jsdelivr.net/npm/monaco-editor@0.52.2/min/vs' } });
  window.require(['vs/editor/editor.main'], () => {
    monaco.languages.register({ id: 'vibe' });
    monaco.languages.setMonarchTokensProvider('vibe', {
      keywords: ['set','make','add','remove','if','otherwise','repeat','for','each','while','define','give','back','when','use','try','fail','test','assert','equals','not_equals','less_than','greater_than','and','or','not','power','modulo'],
      builtins: ['say','warn','error','pi','e','sin','cos','tan','log','log10','exp','lerp','sqrt','round','floor','ceil','min','max','clamp','vec3','dot','cross','magnitude','normalize','distance','angle','degrees','radians'],
      tokenizer: {
        root: [
          [/#.*$/, 'comment'],
          [/note:.*$/, 'comment'],
          [/"([^"\\]|\\.)*"/, 'string'],
          [/\d+(\.\d+)?/, 'number'],
          [/[a-zA-Z_]\w*/, { cases: { '@keywords': 'keyword', '@builtins': 'type.identifier', '@default': 'identifier' } }],
        ],
      },
    });
    editor = monaco.editor.create(document.getElementById('editor-container'), {
      value: codeEl.value,
      language: 'vibe',
      theme: 'vs-dark',
      automaticLayout: true,
      fontSize: 14,
      minimap: { enabled: false },
    });
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, run);
  });
}

async function initWasm() {
  try {
    const response = await fetch('azra.wasm');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const bytes = await response.arrayBuffer();
    const module = await WebAssembly.instantiate(bytes, {});
    wasm = module.instance.exports;
    statusEl.textContent = 'Interpreter ready';
    run();
  } catch (err) {
    statusEl.textContent = 'WASM failed to load';
    outputEl.className = 'error';
    outputEl.textContent = `Could not load azra.wasm: ${err}\n\nBuild it with:\n  cargo build --release --target wasm32-unknown-unknown --lib\n  copy target/wasm32-unknown-unknown/release/azra.wasm here`;
  }
}

function run() {
  if (!wasm) return;
  const bytes = new TextEncoder().encode(source());
  const inputPtr = wasm.azra_input_ptr();
  new Uint8Array(wasm.memory.buffer).set(bytes, inputPtr);
  const code = wasm.azra_run(bytes.length);
  const length = wasm.azra_output_len();
  // The interpreter may have grown linear memory, so re-read the pointer.
  const text = new TextDecoder().decode(
    new Uint8Array(wasm.memory.buffer, wasm.azra_output_ptr(), length)
  );
  outputEl.textContent = text || (code === 0 ? '(no output)' : '(error)');
  outputEl.className = code === 0 ? '' : 'error';
  exitEl.textContent = code === 0 ? 'exit 0' : `exit ${code}`;
}

Object.keys(EXAMPLES).forEach(name => {
  const option = document.createElement('option');
  option.value = name;
  option.textContent = name;
  selectEl.appendChild(option);
});

selectEl.addEventListener('change', () => setSource(EXAMPLES[selectEl.value]));
document.getElementById('run-button').addEventListener('click', run);
document.getElementById('clear-button').addEventListener('click', () => {
  setSource('');
  outputEl.textContent = '';
});
document.getElementById('share-button').addEventListener('click', () => {
  const url = `${location.origin}${location.pathname}#code=${encodeShare(source())}`;
  navigator.clipboard?.writeText(url);
  history.replaceState(null, '', `#code=${encodeShare(source())}`);
  statusEl.textContent = 'Share link copied to clipboard';
});

const shared = decodeShare(location.hash);
const saved = localStorage.getItem('azra-playground-source');
setSource(shared || saved || EXAMPLES.Hello);
codeEl.addEventListener('input', () => localStorage.setItem('azra-playground-source', codeEl.value));
document.addEventListener('keydown', e => {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); run(); }
});

setupEditor();
initWasm();
