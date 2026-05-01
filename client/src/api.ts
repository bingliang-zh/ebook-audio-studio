const apiBaseUrl = import.meta.env.VITE_API_BASE_URL?.replace(/\/$/, "") ?? "";

export function apiUrl(path: string) {
  return `${apiBaseUrl}${path}`;
}

export function assetUrl(path: string) {
  if (/^https?:\/\//.test(path)) {
    return path;
  }

  return apiUrl(path);
}

