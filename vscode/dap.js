const cp = require('child_process');
const path = require('path');
const readline = require('readline');

let seq = 0;
let child = null;
let breakpoints = [];
let program = '';
let releaseMode = false;
let compiler = 'still';

function send(msg) {
  msg.seq = ++seq;
  const body = JSON.stringify(msg);
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

function sendEvent(event) {
  send({ type: 'event', event: event.event, body: event.body || {} });
}

function log(text) {
  sendEvent({ event: 'output', body: { category: 'stderr', output: text } });
}

async function readMessages(handler) {
  const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
  let buffer = '';
  let header = '';
  for await (const chunk of rl) {
    const line = chunk + '\n';
    if (header === null) continue;
    if (line.trim() === '') {
      const m = /Content-Length:\s*(\d+)/i.exec(header);
      if (m) {
        buffer = rl.read(m[1] ? null : null) || buffer;
      }
      header = '';
    } else {
      header += line;
    }
  }
}

async function main() {
  let pending = Buffer.alloc(0);
  const onData = (chunk) => {
    pending = Buffer.concat([pending, chunk]);
    while (true) {
      const headerEnd = pending.indexOf('\r\n\r\n');
      if (headerEnd < 0) return;
      const header = pending.slice(0, headerEnd).toString('utf8');
      const m = /Content-Length:\s*(\d+)/i.exec(header);
      if (!m) {
        pending = pending.slice(headerEnd + 4);
        continue;
      }
      const len = parseInt(m[1], 10);
      if (pending.length < headerEnd + 4 + len) return;
      const body = pending.slice(headerEnd + 4, headerEnd + 4 + len).toString('utf8');
      pending = pending.slice(headerEnd + 4 + len);
      let msg;
      try {
        msg = JSON.parse(body);
      } catch (e) {
        continue;
      }
      handle(msg).catch((err) => {
        log(`debug adapter error: ${err}\n`);
      });
    }
  };
  process.stdin.on('data', onData);
}

async function handle(msg) {
  const cmd = msg.command;
  if (cmd === 'initialize') {
    send({
      type: 'response',
      request_seq: msg.seq,
      success: true,
      command: cmd,
      body: {
        supportsConfigurationDoneRequest: true,
        supportsSetBreakpointsRequest: true,
        supportsTerminateRequest: true,
        supportsRestartRequest: false
      }
    });
    sendEvent({ event: 'initialized' });
    return;
  }
  if (cmd === 'setBreakpoints') {
    const requested = (msg.arguments.breakpoints || []).map((b) => b.line);
    breakpoints = requested;
    send({
      type: 'response',
      request_seq: msg.seq,
      success: true,
      command: cmd,
      body: {
        breakpoints: requested.map((line) => ({ id: line, verified: true, message: 'Stops the program at this line (debug build)' }))
      }
    });
    return;
  }
  if (cmd === 'launch') {
    program = msg.arguments.program;
    releaseMode = !!msg.arguments.release;
    compiler = msg.arguments.compilerPath || 'still';
    send({ type: 'response', request_seq: msg.seq, success: true, command: cmd, body: {} });
    return;
  }
  if (cmd === 'configurationDone') {
    send({ type: 'response', request_seq: msg.seq, success: true, command: cmd, body: {} });
    startProgram();
    return;
  }
  if (cmd === 'threads') {
    send({ type: 'response', request_seq: msg.seq, success: true, command: cmd, body: { threads: [{ id: 1, name: 'main' }] } });
    return;
  }
  if (cmd === 'stackTrace') {
    send({
      type: 'response',
      request_seq: msg.seq,
      success: true,
      command: cmd,
      body: { stackFrames: [], totalFrames: 0 }
    });
    return;
  }
  if (cmd === 'scopes' || cmd === 'variables') {
    send({ type: 'response', request_seq: msg.seq, success: true, command: cmd, body: { scopes: [], variables: [] } });
    return;
  }
  if (cmd === 'terminate' || cmd === 'disconnect') {
    if (child) {
      child.kill('SIGKILL');
    }
    send({ type: 'response', request_seq: msg.seq, success: true, command: cmd, body: {} });
    setTimeout(() => process.exit(0), 100);
    return;
  }
  send({ type: 'response', request_seq: msg.seq, success: true, command: cmd, body: {} });
}

function startProgram() {
  sendEvent({ event: 'output', body: { category: 'console', output: `building ${path.basename(program)} (debug)\n` } });
  const bpEnv = {};
  if (breakpoints.length > 0) {
    const file = path.basename(program);
    bpEnv.STILL_BREAKPOINTS = breakpoints.map((l) => `${file}:${l}`).join(',');
  }
  const buildArgs = ['build', program, '-o', '/tmp/still_debug_out'];
  if (releaseMode) buildArgs.push('--release');
  const build = cp.spawn(compiler, buildArgs, {
    env: { ...process.env, ...bpEnv }
  });
  let buildErr = '';
  build.stderr.on('data', (d) => {
    buildErr += d.toString();
  });
  build.on('error', (err) => {
    sendEvent({ event: 'output', body: { category: 'stderr', output: `cannot start the still compiler: ${err.message}\nIs it installed and on PATH? See "Still: Show Compiler Setup Instructions".\n` } });
    sendEvent({ event: 'terminated' });
  });
  build.on('close', (code) => {
    if (code !== 0) {
      sendEvent({ event: 'output', body: { category: 'stderr', output: buildErr + '\nbuild failed\n' } });
      sendEvent({ event: 'terminated' });
      return;
    }
    child = cp.spawn('/tmp/still_debug_out', [], { env: { ...process.env, ...bpEnv } });
    child.stdout.on('data', (d) => {
      sendEvent({ event: 'output', body: { category: 'stdout', output: d.toString() } });
    });
    child.stderr.on('data', (d) => {
      sendEvent({ event: 'output', body: { category: 'stderr', output: d.toString() } });
    });
    child.on('close', (code, signal) => {
      sendEvent({ event: 'output', body: { category: 'console', output: `\nprogram ${signal ? 'stopped by ' + signal : 'exited with code ' + code}\n` } });
      if (signal === 'SIGTRAP') {
        sendEvent({ event: 'output', body: { category: 'console', output: 'breakpoint reached — the program was halted. Use Run to start again.\n' } });
      }
      sendEvent({ event: 'exited', body: { exitCode: code || 0 } });
      sendEvent({ event: 'terminated' });
      child = null;
    });
  });
}

main().catch((e) => {
  process.stderr.write(`fatal: ${e}\n`);
  process.exit(1);
});
