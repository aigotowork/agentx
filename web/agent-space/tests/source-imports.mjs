/** Resolve Space's browser-style extensionless imports for Node transform-mode cases. */
import { existsSync } from 'node:fs';
import { registerHooks } from 'node:module';

registerHooks({ resolve(specifier, context, next) {
  if (specifier.startsWith('.') && context.parentURL) {
    const exact = new URL(specifier, context.parentURL);
    if (!existsSync(exact) && existsSync(new URL(`${exact.href}.ts`))) return next(`${exact.href}.ts`, context);
  }
  return next(specifier, context);
} });
