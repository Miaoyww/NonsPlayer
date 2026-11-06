const metadataPrefix = /^\s*(?:\[[^\]\r\n]+\]\s*)*/

function isNeteaseMetadata(line: string) {
  const json = line.replace(metadataPrefix, '')
  if (!json.startsWith('{')) return false
  try {
    const value = JSON.parse(json) as { c?: { tx?: unknown }[] }
    return Array.isArray(value.c) && value.c.some((item) => typeof item?.tx === 'string')
  } catch {
    return false
  }
}

export function stripNeteaseMetadata(content: string) {
  return content
    .split(/\r?\n/)
    .filter((line) => !isNeteaseMetadata(line))
    .join('\n')
}
