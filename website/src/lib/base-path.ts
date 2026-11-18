const basePath = (process.env.NEXT_PUBLIC_BASE_PATH ?? '').replace(/\/$/, '');

export function withBasePath(path: `/${string}`) {
  return `${basePath}${path}`;
}
