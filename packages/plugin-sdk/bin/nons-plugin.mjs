#!/usr/bin/env node
import { run } from '../lib/cli.mjs'

run(process.argv.slice(2)).catch((error) => {
  console.error(error?.message ?? error)
  process.exitCode = 1
})
