import { cpSync, existsSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const websiteDir = path.resolve(scriptDir, '..');
const sourceDir = path.resolve(websiteDir, '..', 'docs');
const generatedRoot = path.resolve(websiteDir, '.content');
const outputDir = path.resolve(generatedRoot, 'docs');

if (!existsSync(sourceDir)) {
  throw new Error(`Canonical documentation directory is missing: ${sourceDir}`);
}

if (!outputDir.startsWith(`${generatedRoot}${path.sep}`)) {
  throw new Error(`Refusing to write outside generated content: ${outputDir}`);
}

rmSync(outputDir, { recursive: true, force: true });

cpSync(sourceDir, outputDir, {
  recursive: true,
  filter(source) {
    if (source === sourceDir) return true;
    const extension = path.extname(source).toLowerCase();
    return extension === '' || extension === '.md' || extension === '.mdx' || extension === '.json';
  },
});

console.log(`Synced canonical docs: ${sourceDir} -> ${outputDir}`);
