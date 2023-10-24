////////////////////////////////////////////////////////////////////////////////
//
//  Basic Exports and Imports
//
////////////////////////////////////////////////////////////////////////////////

/*--- path: a_basic.js ---*/

// Direct exports of declarations
export let a;
export let b = 1;
export function c() { }
export class D { }

// Export list
let e = 2;
export { e };

// Renaming exports
let f = 3;
export { f as g };

// Exporting destructured assignments with renaming
export const { "k0": h, "k1": i } = { "k0": 4, "k1": 5 };
export const [j,
              k] = [1,2];
export const { "a": [l] } = { "a": [1] };
export const { m = 1 } = { "m": 2 };

/*--- path: b_basic_0.js ---*/

import { a, b as b2 } from "./a_basic.js";

   a;
// ^ defined: 10, 32

   b2;
// ^ defined: 11, 32

/*--- path: b_basic_1.js ---*/

import * as mod from "./a_basic.js";

   mod;
// ^ defined: 42

   mod.a;
//     ^ defined: 10

   mod.b;
//     ^ defined: 11

   mod.c;
//     ^ defined: 12

   mod.D;
//     ^ defined: 13

   mod.e;
//     ^ defined: 16, 17

   mod.f;
//     ^ defined:

   mod.g;
//     ^ defined: 20, 21

   mod.h;
//     ^ defined: 24

   mod.i;
//     ^ defined: 24

   mod.j;
//     ^ defined: 25

   mod.k;
//     ^ defined: 26

   mod.l;
//     ^ defined: 27

   mod.m;
//     ^ defined: 28


////////////////////////////////////////////////////////////////////////////////
//
//  Default exports
//
////////////////////////////////////////////////////////////////////////////////

/*--- path: a_default_0.js ---*/

let n = 6;
export default n;


/*--- path: a_default_1.js ---*/

export default function () { }


/*--- path: a_default_2.js ---*/

export default function* () { }


/*--- path: a_default_3.js ---*/
export default class { }


/*--- path: a_default_4.js ---*/

let o = 7;

export { o as default };


/*--- path: b_default.js ---*/

import p from "./a_default_0.js"
import q from "./a_default_1.js"
import r from "./a_default_2.js"
import s from "./a_default_3.js"

   p;
// ^ defined: 95, 96, 122

   q;
// ^ defined: 101, 123

   r;
// ^ defined: 106, 124

   s;
// ^ defined: 110, 125


////////////////////////////////////////////////////////////////////////////////
//
//  Aggregating Modules
//
////////////////////////////////////////////////////////////////////////////////

/*--- path: b_aggregating_0.js ---*/

export * from "./a_basic.js";


/*--- path: b_aggregating_1.js ---*/

export * as t from "./a_basic.js";


/*--- path: b_aggregating_2.js ---*/

export { c, D as D1 } from "./a_basic.js";


/*--- path: b_aggregating_3.js ---*/

export { e as default } from "./a_basic.js";


/*--- path: c_aggregating.js ---*/

import { a } from "./b_aggregating_0.js";

   a;
// ^ defined: 10, 168

   b;
// ^ defined:

import { t } from "./b_aggregating_1.js";

   t.a;
// ^ defined: 153, 176
//   ^ defined: 10

import { c, D1 } from "./b_aggregating_2.js";

   c;
// ^ defined: 12, 158, 182

   D1;
// ^ defined: 13, 158, 182

import e2 from "./b_aggregating_3.js";

   e2;
// ^ defined: 16, 17, 163, 190

////////////////////////////////////////////////////////////////////////////////
//
//  Imports from super and subdirectories
//
////////////////////////////////////////////////////////////////////////////////

/*--- path: dirs/foo.js ---*/

export let x = 42;

/*--- path: dirs/bar/baz.js ---*/

export let x = 42;

/*--- path: dirs/test_0.js ---*/

import { x } from "./foo"
//       ^ defined: 203

/*--- path: dirs/test_1.js ---*/

import { x } from "./bar/baz"
//       ^ defined: 207

/*--- path: dirs/bar/test_2.js ---*/

import { x } from "../foo"
//       ^ defined: 203

////////////////////////////////////////////////////////////////////////////////
//
//  Import using `import()` function
//
////////////////////////////////////////////////////////////////////////////////

/*--- path: b_function_1.js ---*/

const mod = await import("./a_basic.js");

   mod;
// ^ defined: 232

   mod.a;
//     ^ defined: 10

////////////////////////////////////////////////////////////////////////////////
//
//  Import directory with `index.js`
//
////////////////////////////////////////////////////////////////////////////////

/*--- path: dir_imports/foo/index.js ---*/

export let x = 42;

/*--- path: dir_imports/test_0.js ---*/

import { x } from "./foo/"
//       ^ defined: 248

/*--- path: dir_imports/foo/test_1.js ---*/

import { x } from "./"
//       ^ defined: 248

/*--- path: dir_imports/bar/test_2.js ---*/

import { x } from "../foo/"
//       ^ defined: 248
