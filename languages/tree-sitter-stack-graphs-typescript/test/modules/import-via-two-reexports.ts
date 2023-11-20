/* --- path: src/foo/index.ts --- */

export * from "../bar";

/* --- path: src/bar/index.ts --- */

export * from "./quz";

/* --- path: src/bar/quz.ts --- */

export const QUZ = 42;

/* --- path: src/test.ts --- */

import { QUZ } from "./foo";
