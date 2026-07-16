#!/usr/bin/env node
import { runCli } from './lib/validate-capability-audit-core.mjs';

try {
  process.stdout.write(runCli(process.argv.slice(2)));
} catch (error) {
  process.stderr.write((error && error.message) ? error.message + '\n' : String(error) + '\n');
  process.exit(error && typeof error.exitCode === 'number' ? error.exitCode : 1);
}
