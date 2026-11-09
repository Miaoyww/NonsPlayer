export function batchModule() {
  const selection = {
    keys: new Set(),
    onToggle() {},
    onToggleAll() {},
    onVisibleChange() {}
  }
  return {
    active: false,
    selected: new Set(),
    selectedTracks: [],
    selectedCount: 0,
    selection,
    enter() {},
    exit() {},
    invert() {},
    useTrackBatch: () => ({
      active: false,
      selected: new Set(),
      selectedTracks: [],
      selectedCount: 0,
      selection,
      enter() {},
      exit() {},
      invert() {}
    }),
    BatchMenuButton: 'BatchMenuButton',
    BatchToolbar: 'BatchToolbar',
    BatchDownloadButton: 'BatchDownloadButton',
    BatchMenuItem: 'BatchMenuItem'
  }
}
