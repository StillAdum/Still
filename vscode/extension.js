const vscode = require('vscode');
const cp = require('child_process');
const path = require('path');

const KEYWORDS = ['func', 'type', 'extern', 'use', 'as', 'if', 'elif', 'else', 'while', 'for', 'in', 'and', 'or', 'not', 'is', 'break', 'continue', 'return', 'pass', 'true', 'false', 'none', 'self'];
const BUILTINS = ['print', 'len', 'push', 'pop', 'keys', 'pairs', 'str', 'int', 'float', 'bool', 'abs', 'min', 'max', 'sum', 'range', 'enumerate', 'reversed', 'type_of', 'input', 'clock', 'rand', 'seed', 'check', 'fail', 'exit', 'ord', 'chr', 'read_file', 'write_file', 'get_env', 'sleep_ms'];
const BUILTIN_DOCS = {
  print: 'print(value, ...) — print values separated by spaces',
  len: 'len(x) — length of a str, list, dict or @soa value',
  push: 'push(list, value) — add a value to the end of a list',
  pop: 'pop(list) — remove and return the last item of a list',
  keys: 'keys(dict) — list of the dict keys',
  pairs: 'pairs(dict) — list of (key, value) tuples',
  str: 'str(x) — convert to a string',
  int: 'int(x) — convert to int (floats truncate, strings must be whole numbers)',
  float: 'float(x) — convert to float',
  bool: 'bool(x) — convert to bool (none, false, 0, 0.0, "" and [] are false)',
  abs: 'abs(x) — absolute value',
  min: 'min(a, b, ...) or min(list)',
  max: 'max(a, b, ...) or max(list)',
  sum: 'sum(list) — total of a list of numbers',
  range: 'range(n) / range(a, b) / range(a, b, step) — numbers to loop over',
  enumerate: 'enumerate(list) — pairs of (index, item)',
  reversed: 'reversed(list) — copy of the list backwards',
  type_of: 'type_of(x) — name of the value type as a string',
  input: 'input(prompt) — read one line from the user',
  clock: 'clock() — seconds since the epoch as a float',
  rand: 'rand(bound) — random whole number from 0 to bound-1',
  seed: 'seed(n) — seed the random generator for repeatable results',
  check: 'check(cond, msg) — stop the program with a message if cond is false',
  fail: 'fail(msg) — stop the program with a message',
  exit: 'exit(code) — end the program with a status code',
  ord: 'ord(c) — byte value of a one-character string',
  chr: 'chr(n) — one-character string from a byte value',
  read_file: 'read_file(path) — whole file as a string',
  write_file: 'write_file(path, data) — write a string to a file',
  get_env: 'get_env(name) — environment variable value or none',
  sleep_ms: 'sleep_ms(n) — pause for n milliseconds'
};

let diagnosticCollection;
let terminal;
let checkTimer;

function compilerPath() {
  return vscode.workspace.getConfiguration('still').get('compilerPath') || 'still';
}

function runStill(args, cwd) {
  return new Promise((resolve) => {
    const cmd = compilerPath();
    cp.execFile(cmd, args, { cwd, timeout: 15000, maxBuffer: 10 * 1024 * 1024 }, (err, stdout, stderr) => {
      resolve({ err, stdout: stdout || '', stderr: stderr || '' });
    });
  });
}

async function checkDocument(doc) {
  if (!doc || doc.languageId !== 'still') return;
  const res = await runStill(['ide', 'check', doc.uri.fsPath]);
  let diags = [];
  try {
    const parsed = JSON.parse(res.stdout.trim().length ? res.stdout : '[]');
    if (Array.isArray(parsed)) {
      diags = parsed.map((d) => {
        const range = new vscode.Range(
          Math.max(0, (d.line || 1) - 1), Math.max(0, (d.col || 1) - 1),
          Math.max(0, (d.line || 1) - 1), Math.max(1, (d.col || 1))
        );
        const message = (d.file && !d.file.endsWith('.strc')) ? d.message : d.message;
        return new vscode.Diagnostic(range, d.message || String(d), vscode.DiagnosticSeverity.Error);
      });
    }
  } catch (e) {
    if (res.stderr) {
      vscode.window.showWarningMessage(`still check: ${res.stderr.split('\n')[0]}`);
    }
  }
  diagnosticCollection.set(doc.uri, diags);
}

async function fileSymbols(file) {
  const res = await runStill(['ide', 'symbols', file]);
  try {
    return JSON.parse(res.stdout).symbols || [];
  } catch (e) {
    return [];
  }
}

function makeTerminal() {
  if (!terminal) {
    terminal = vscode.window.createTerminal('Still');
  }
  return terminal;
}

function quoteIfNeeded(p) {
  return p.includes(' ') ? `"${p}"` : p;
}

function activate(context) {
  diagnosticCollection = vscode.languages.createDiagnosticCollection('still');
  context.subscriptions.push(diagnosticCollection);

  context.subscriptions.push(
    vscode.commands.registerCommand('still.runFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'still') {
        vscode.window.showErrorMessage('Open a .strc file to run it.');
        return;
      }
      await editor.document.save();
      const t = makeTerminal();
      t.show(true);
      t.sendText(`${quoteIfNeeded(compilerPath())} run ${quoteIfNeeded(editor.document.uri.fsPath)}`, true);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('still.debugFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'still') {
        vscode.window.showErrorMessage('Open a .strc file to debug it.');
        return;
      }
      await editor.document.save();
      await vscode.debug.startDebugging(undefined, {
        type: 'still',
        request: 'launch',
        name: `Debug ${path.basename(editor.document.uri.fsPath)}`,
        program: editor.document.uri.fsPath,
        compilerPath: compilerPath()
      });
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('still.checkFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      await checkDocument(editor.document);
      const diags = diagnosticCollection.get(editor.document.uri) || [];
      if (diags.length === 0) {
        vscode.window.showInformationMessage('Still: no errors found.');
      } else {
        vscode.window.showWarningMessage(`Still: ${diags.length} error(s) — see Problems.`);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('still.showSetup', () => {
      const readme = path.join(context.extensionPath, 'README.md');
      vscode.commands.executeCommand('markdown.showPreview', vscode.Uri.file(readme));
      vscode.window.showInformationMessage('Still setup instructions opened — see the preview.');
    })
  );

  const checkConfig = () => {
    const cfg = vscode.workspace.getConfiguration('still');
    return { save: cfg.get('diagnosticsOnSave', true), type: cfg.get('diagnosticsOnType', true) };
  };

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      const c = checkConfig();
      if (c.save || c.type) checkDocument(doc);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((e) => {
      const c = checkConfig();
      if (!c.type) return;
      if (checkTimer) clearTimeout(checkTimer);
      checkTimer = setTimeout(() => checkDocument(e.document), 350);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      const c = checkConfig();
      if (c.save) checkDocument(doc);
    })
  );

  context.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(
      'still',
      {
        async provideCompletionItems(document, position) {
          const items = [];
          const seen = new Set();
          const lineText = document.getText(new vscode.Range(position.with(undefined, 0), position));
          const afterDot = /\.\s*[a-zA-Z_]*$/.test(lineText);

          if (!afterDot) {
            for (const kw of KEYWORDS) {
              const item = new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword);
              item.detail = 'Still keyword';
              items.push(item);
              seen.add(kw);
            }
          }
          for (const b of BUILTINS) {
            if (seen.has(b)) continue;
            const item = new vscode.CompletionItem(b, vscode.CompletionItemKind.Function);
            item.detail = 'Still builtin';
            item.documentation = new vscode.MarkdownString(BUILTIN_DOCS[b] || '');
            items.push(item);
            seen.add(b);
          }
          const syms = await fileSymbols(document.uri.fsPath);
          for (const s of syms) {
            if (seen.has(s.name)) continue;
            const kinds = {
              function: vscode.CompletionItemKind.Function,
              method: vscode.CompletionItemKind.Method,
              type: vscode.CompletionItemKind.Class,
              field: vscode.CompletionItemKind.Field,
              variable: vscode.CompletionItemKind.Variable,
              module: vscode.CompletionItemKind.Module
            };
            const item = new vscode.CompletionItem(s.name, kinds[s.kind] || vscode.CompletionItemKind.Variable);
            item.detail = s.detail || s.kind;
            const line = Math.max(0, (s.line || 1) - 1);
            item.range = new vscode.Range(line, 0, line, 0);
            item.command = undefined;
            items.push(item);
            seen.add(s.name);
          }
          return items;
        }
      },
      '.'
    )
  );

  context.subscriptions.push(
    vscode.languages.registerHoverProvider('still', {
      async provideHover(document, position) {
        const wordRange = document.getWordRangeAtPosition(position, /[a-zA-Z_][a-zA-Z0-9_]*/);
        if (!wordRange) return null;
        const word = document.getText(wordRange);
        if (BUILTIN_DOCS[word]) {
          return new vscode.Hover(new vscode.MarkdownString('```still\n' + BUILTIN_DOCS[word] + '\n```'), wordRange);
        }
        const syms = await fileSymbols(document.uri.fsPath);
        const sym = syms.find((s) => s.name === word);
        if (sym) {
          const label = sym.kind === 'type' ? `type ${sym.name}` : sym.kind === 'function' ? `func ${sym.name}${sym.detail || '()'}` : sym.name;
          return new vscode.Hover(new vscode.MarkdownString('```still\n' + label + '\n```'), wordRange);
        }
        return null;
      }
    })
  );

  context.subscriptions.push(
    vscode.languages.registerDefinitionProvider('still', {
      async provideDefinition(document, position) {
        const wordRange = document.getWordRangeAtPosition(position, /[a-zA-Z_][a-zA-Z0-9_]*/);
        if (!wordRange) return null;
        const word = document.getText(wordRange);
        const syms = await fileSymbols(document.uri.fsPath);
        const sym = syms.find((s) => s.name === word && s.line > 0);
        if (sym) {
          const line = Math.max(0, sym.line - 1);
          return new vscode.Location(document.uri, new vscode.Range(line, 0, line, 0));
        }
        return null;
      }
    })
  );

  context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory('still', {
      createDebugAdapterDescriptor(session) {
        const dapPath = path.join(context.extensionPath, 'dap.js');
        return new vscode.DebugAdapterExecutable('node', [dapPath]);
      }
    })
  );

  if (vscode.window.activeTextEditor && vscode.window.activeTextEditor.document.languageId === 'still') {
    checkDocument(vscode.window.activeTextEditor.document);
  }
}

function deactivate() {
  if (checkTimer) clearTimeout(checkTimer);
}

module.exports = { activate, deactivate };
