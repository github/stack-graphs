/* --- path: index.ts --- */
import { foo } from "@my/lib";
//       ^ defined: 8

/* --- path: mod.ts --- */

declare module "@my/lib" {
    export const foo = 42;
}
