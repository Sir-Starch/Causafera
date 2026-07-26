/**
 * The dictionary contract.
 *
 * `Copy` is derived from the English baseline rather than declared separately, so a key added
 * to `en.ts` is a compile error in every other locale until it is translated. That is the point:
 * the type system, not review discipline, is what keeps the dictionaries in step.
 */

import { en } from "./en";

export type Copy = typeof en;
