// Neutral inline avatar shown when a user has no Discord avatar URL. Inlined as
// a data URI so it never depends on an external image host — this keeps avatars
// working offline and in production (the old https://via.placeholder.com fallback
// failed with ERR_CONNECTION_CLOSED when that host was unreachable).
export const DEFAULT_AVATAR =
  'data:image/svg+xml;utf8,' +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">' +
      '<rect width="64" height="64" rx="8" fill="#394150"/>' +
      '<circle cx="32" cy="25" r="12" fill="#7a8699"/>' +
      '<path d="M12 58c0-11 9-19 20-19s20 8 20 19z" fill="#7a8699"/>' +
      '</svg>',
  )
