// Navigation hints are presentation state, never authority or file ownership.
export function workspaceReturn(value, origin) {
  if (!value || value.length > 4096) return null;
  try {
    const url = new URL(value, origin);
    if (url.origin !== origin || !/^\/(?:en\/)?pilveruum\/(?:$|(?:file|folder)\/[a-f0-9-]+$)/i.test(url.pathname)) return null;
    if (url.searchParams.has('picker')) return null;
    return url.pathname + url.search + url.hash;
  } catch { return null; }
}
export function editorContext(href, location) {
  const url = new URL(href, location.origin);
  const back = workspaceReturn(location.pathname + location.search + location.hash, location.origin);
  if (back) url.searchParams.set('workspace_return', back);
  return url.pathname + url.search + url.hash;
}
export const creationLabels = {
  wordcraft: 'Uus dokument', gridcraft: 'Uus tabel', deckcraft: 'Uus esitlus',
  photocraft: 'Uus pilt', pdfcraft: 'Uus PDF', figcraft: 'Uus disain',
  cadcraft: 'Uus joonis', soundcraft: 'Uus heliprojekt',
};
export function returnStateKey(principal, location) {
  return `subnet.workspace.return:${principal}:${location.pathname + location.search + location.hash}`;
}

// Per-editor startup flags are declared in the registry, not in file semantics.
export function creationContext(app, destination, location) {
  const params = new URLSearchParams(app.createQuery || {});
  if (destination) {
    params.set('cloud_service', 'cloud');
    params.set('cloud_space', destination.spaceId);
    params.set('cloud_parent', destination.id);
  }
  return editorContext(`/apps/${app.id}/?${params}`, location);
}
